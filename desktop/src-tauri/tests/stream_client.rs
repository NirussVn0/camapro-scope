//! T2 unit tests: MJPEG stream client against in-process TcpListener fakes.

mod common;

use std::sync::mpsc;
use std::time::Duration;

use common::{canned_server, http200_multipart, multipart_body, spawn_fake, TINY_JPEG};

use camapro_scope_lib::core::media::stream_client::StreamClient;
use camapro_scope_lib::core::media::stream_client::StreamError;

#[test]
fn sends_get_stream_with_token_and_yields_exact_frame_bytes() {
    let (addr, requests) = canned_server(http200_multipart(&multipart_body(3)));

    let (mut client, frames) = StreamClient::start(addr.to_string(), "tok".into()).unwrap();
    let first = frames
        .recv_timeout(Duration::from_secs(2))
        .expect("frame 1 within 2s");
    assert_eq!(first, TINY_JPEG);
    let second = frames
        .recv_timeout(Duration::from_secs(2))
        .expect("frame 2 within 2s");
    assert_eq!(second, TINY_JPEG);
    client.stop();

    let seen = requests.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(seen.path, "/stream");
    assert_eq!(seen.token.as_deref(), Some("tok"));
}

#[test]
fn server_401_maps_to_http_status_error() {
    let (addr, _requests) =
        canned_server(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n".to_vec());

    let err = StreamClient::start(addr.to_string(), "tok".into()).unwrap_err();
    match err {
        StreamError::HttpStatus(401) => {}
        other => panic!("expected HttpStatus(401), got {other:?}"),
    }
}

#[test]
fn non_multipart_content_type_is_http_status_error() {
    let (addr, _requests) =
        canned_server(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n".to_vec());

    let err = StreamClient::start(addr.to_string(), "tok".into()).unwrap_err();
    assert!(
        matches!(err, StreamError::HttpStatus(200)),
        "expected HttpStatus(200) for bad content-type, got {err:?}"
    );
}

#[test]
fn server_close_ends_stream_cleanly() {
    let blob = http200_multipart(&multipart_body(1));
    let (tx, _requests) = mpsc::channel();
    let addr = spawn_fake(tx, move |stream| {
        std::io::Write::write_all(stream, &blob).unwrap();
        // drop → close: client must see a clean end, not an error frame
    });

    let (mut client, frames) = StreamClient::start(addr.to_string(), "tok".into()).unwrap();
    assert_eq!(
        frames.recv_timeout(Duration::from_secs(2)),
        Ok(TINY_JPEG.to_vec())
    );
    // Channel disconnects when the reader thread ends after server EOF.
    assert!(frames.recv_timeout(Duration::from_secs(2)).is_err());
    client.stop(); // must join, not hang
}

#[test]
fn stop_before_any_frame_is_clean_and_idempotent() {
    // 200 + multipart head, then silence: frames never arrive.
    let (tx, _requests) = mpsc::channel();
    let addr = spawn_fake(tx, move |stream| {
        std::io::Write::write_all(
            stream,
            b"HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=frame\r\n\r\n",
        )
        .unwrap();
        std::thread::sleep(Duration::from_secs(2)); // hold open, no body
    });

    let (mut client, _frames) = StreamClient::start(addr.to_string(), "tok".into()).unwrap();
    client.stop();
    client.stop(); // idempotent
}

#[test]
fn connection_refused_maps_to_connect_error() {
    // Bind then drop to get a recently-free (refusing) port.
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = probe.local_addr().unwrap();
    drop(probe);

    let err = StreamClient::start(addr.to_string(), "tok".into()).unwrap_err();
    assert!(matches!(err, StreamError::Connect(_)), "got {err:?}");
}

#[test]
fn slow_consumer_retains_only_two_newest_frames() {
    let mut body = Vec::new();
    for value in 0u8..100 {
        body.extend_from_slice(b"--frame\r\nContent-Type: image/jpeg\r\nContent-Length: 1\r\n\r\n");
        body.extend_from_slice(&[value, b'\r', b'\n']);
    }
    let (addr, _) = canned_server(http200_multipart(&body));
    let (mut client, frames) = StreamClient::start(addr.to_string(), "tok".into()).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(frames.len(), 2);
    assert_eq!(frames.recv().unwrap(), vec![98]);
    assert_eq!(frames.recv().unwrap(), vec![99]);
    client.stop();
}

#[test]
fn stop_interrupts_trickled_partial_tls_record_without_join_stall() {
    use std::io::Write;
    let (tx, _) = mpsc::channel();
    let addr = spawn_fake(tx, |stream| {
        stream.write_all(&http200_multipart(&[])).unwrap();
        stream.flush().unwrap();
        // Record is deliberately incomplete; per-read polling alone can be renewed.
        stream.sock.write_all(&[23, 3, 3, 0, 100]).unwrap();
        for _ in 0..40 {
            if stream.sock.write_all(&[0]).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
    let (mut client, _) = StreamClient::start(addr.to_string(), "tok".into()).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let started = std::time::Instant::now();
    client.stop();
    assert!(client.is_finished());
    assert!(started.elapsed() < Duration::from_secs(1));
}
