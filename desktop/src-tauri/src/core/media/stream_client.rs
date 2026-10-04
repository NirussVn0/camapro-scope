//! T2 blocking MJPEG stream client (Rust std only — zero new deps).
//!
//! Owns one reader thread over a single `TcpStream`: sends
//! `GET /stream` + `X-Camapro-Token`, validates the 200 + multipart
//! content-type head, feeds body bytes through [`MjpegStreamParser`] and
//! pushes exact frame bytes onto an `mpsc::Receiver`. A clean server EOF
//! closes the channel; `stop()` signals shutdown and joins the thread.

use crossbeam_channel::{self as channel, Receiver, Sender, TrySendError};
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

use super::mjpeg_parser::MjpegStreamParser;
use crate::core::control::lan_tls::{self, TlsStream};

/// Parser buffer cap: a 4 MiB JPEG head can never legitimately appear;
/// overflow aborts the stream instead of growing memory.
const MAX_BUFFER_BYTES: usize = 4 * 1024 * 1024;
/// Connect + response-head budget. The phone is LAN-local; 5s is generous.
const HEAD_TIMEOUT: Duration = Duration::from_secs(5);
/// Poll granularity while waiting for the head, so stop() stays responsive.
const READ_POLL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    /// TCP connect failed (refused / unreachable / bad address).
    Connect(String),
    /// Status line was not 200, or 200 without a multipart content type.
    HttpStatus(u16),
    /// Server closed the connection cleanly (EOF). The frame channel is
    /// also disconnected when this happens.
    ServerEnd,
}

/// Running client handle; `stop()` is idempotent.
pub struct StreamClient {
    stop: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
    socket: std::net::TcpStream,
}

impl std::fmt::Debug for StreamClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamClient")
            .field("stopping", &self.stop.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl StreamClient {
    /// Connect, validate the head (blocking, bounded by HEAD_TIMEOUT), then
    /// spawn the reader thread. Returns the handle and the frame receiver.
    pub fn start(
        addr: String,
        token: String,
    ) -> Result<(StreamClient, Receiver<Vec<u8>>), StreamError> {
        let mut stream = lan_tls::connect(&addr, &token).map_err(StreamError::Connect)?;
        let head_deadline =
            crate::core::control::socket_deadline::SocketDeadline::new(&stream.sock, HEAD_TIMEOUT)
                .map_err(|e| StreamError::Connect(e.to_string()))?;
        stream
            .sock
            .set_read_timeout(Some(READ_POLL))
            .map_err(|e| StreamError::Connect(e.to_string()))?;

        let request = format!(
            "GET /stream HTTP/1.1\r\nHost: {addr}\r\nX-Camapro-Token: {token}\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|e| StreamError::Connect(e.to_string()))?;

        let head = read_head(&mut stream)?;
        validate_head(&head)?;
        drop(head_deadline); // Validated media readiness; do not time-limit the stream.

        let stop = Arc::new(AtomicBool::new(false));
        let socket = stream
            .sock
            .try_clone()
            .map_err(|e| StreamError::Connect(e.to_string()))?;
        let stop_flag = Arc::clone(&stop);
        let (tx, rx) = channel::bounded::<Vec<u8>>(2);
        let drop_oldest = rx.clone();
        let reader = std::thread::spawn(move || {
            read_frames(stream, tx, drop_oldest, stop_flag);
        });
        Ok((
            StreamClient {
                stop,
                reader: Some(reader),
                socket,
            },
            rx,
        ))
    }

    /// Signal shutdown, close the socket via thread-exit, join. Idempotent.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.socket.shutdown(std::net::Shutdown::Both);
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
    }

    pub fn is_finished(&self) -> bool {
        self.reader.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

impl Drop for StreamClient {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Read bytes until \r\n\r\n (response head), honoring the stop-free
/// HEAD_TIMEOUT budget via short read polls.
fn read_head(stream: &mut impl Read) -> Result<Vec<u8>, StreamError> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    let deadline = Instant::now() + HEAD_TIMEOUT;
    loop {
        if Instant::now() >= deadline {
            return Err(StreamError::Connect("head_timeout".into()));
        }
        match stream.read(&mut byte) {
            Ok(0) => return Err(StreamError::ServerEnd),
            Ok(_) => {
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n\r\n") {
                    return Ok(buf);
                }
                if buf.len() > 8192 {
                    return Err(StreamError::HttpStatus(0));
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                if Instant::now() > deadline {
                    return Err(StreamError::Connect("head_timeout".into()));
                }
            }
            Err(e) => return Err(StreamError::Connect(e.to_string())),
        }
    }
}

fn validate_head(head: &[u8]) -> Result<(), StreamError> {
    let text = String::from_utf8_lossy(head).to_ascii_lowercase();
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or(StreamError::HttpStatus(0))?;
    if status != 200 {
        return Err(StreamError::HttpStatus(status));
    }
    if !text.contains("content-type: multipart/") {
        return Err(StreamError::HttpStatus(200));
    }
    Ok(())
}

fn read_frames(
    mut stream: TlsStream,
    tx: Sender<Vec<u8>>,
    drop_oldest: Receiver<Vec<u8>>,
    stop: Arc<AtomicBool>,
) {
    let mut parser = MjpegStreamParser::new(MAX_BUFFER_BYTES);
    let mut chunk = [0u8; 8192];
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match stream.read(&mut chunk) {
            Ok(0) => return, // clean server EOF → tx dropped → channel ends
            Ok(n) => {
                match parser.feed(&chunk[..n]) {
                    Ok(frames) => {
                        for mut f in frames {
                            loop {
                                if stop.load(Ordering::SeqCst) {
                                    return;
                                }
                                match tx.try_send(f) {
                                    Ok(()) => break,
                                    Err(TrySendError::Full(frame)) => {
                                        let _ = drop_oldest.try_recv();
                                        f = frame;
                                    }
                                    Err(TrySendError::Disconnected(_)) => return,
                                }
                            }
                        }
                    }
                    Err(_) => return, // parser overflow: abort, do not grow
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {}
            Err(_) => return,
        }
    }
}
