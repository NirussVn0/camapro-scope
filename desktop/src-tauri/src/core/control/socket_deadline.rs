use std::net::{Shutdown, TcpStream};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

/// A socket close, rather than a per-read timeout, bounds even trickled TLS records.
pub(crate) struct SocketDeadline {
    cancel: Option<mpsc::Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl SocketDeadline {
    pub(crate) fn new(socket: &TcpStream, budget: Duration) -> std::io::Result<Self> {
        let deadline = std::time::Instant::now() + budget;
        let socket = socket.try_clone()?;
        let (cancel, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            if matches!(
                receiver
                    .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                let _ = socket.shutdown(Shutdown::Both);
            }
        });
        Ok(Self {
            cancel: Some(cancel),
            worker: Some(worker),
        })
    }
}

impl Drop for SocketDeadline {
    fn drop(&mut self) {
        self.cancel.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Instant;

    #[test]
    fn trickled_incomplete_tls_handshake_is_closed_at_absolute_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let config = crate::core::control::lan_tls::Identity::generate()
            .unwrap()
            .server_config()
            .unwrap();
        let worker = std::thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            let _deadline = SocketDeadline::new(&socket, Duration::from_millis(300)).unwrap();
            let mut stream =
                rustls::StreamOwned::new(rustls::ServerConnection::new(config).unwrap(), socket);
            assert!(stream.read_exact(&mut [0]).is_err());
        });
        let mut socket = TcpStream::connect(addr).unwrap();
        socket.write_all(&[22, 3, 3, 0, 100]).unwrap();
        let started = Instant::now();
        for _ in 0..20 {
            if socket.write_all(&[0]).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        worker.join().unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn canceled_deadline_does_not_close_ready_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        drop(SocketDeadline::new(&server, Duration::from_millis(50)).unwrap());
        std::thread::sleep(Duration::from_millis(100));
        client.write_all(b"x").unwrap();
        let mut byte = [0];
        server.read_exact(&mut byte).unwrap();
        assert_eq!(byte, *b"x");
    }
}
