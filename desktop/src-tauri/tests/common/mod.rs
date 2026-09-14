//! Shared helpers for T2 integration tests: a tiny verified JPEG and
//! multipart/HTTP fake-server builders. Not every test binary uses every
//! helper.
#![allow(dead_code)]

use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc;
use std::time::Duration;

/// Minimal 1x1 grayscale JPEG. Decodability verified on this host:
/// `gst-launch-1.0 filesrc ! jpegparse ! jpegdec ! videoconvert ! filesink`
/// produced exactly 48 bytes of raw frame data (T2 decode-chain proof).
pub const TINY_JPEG: &[u8] = &[
    255, 216, 255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 1, 0, 96, 0, 96, 0, 0, 255, 219, 0, 67, 0,
    8, 6, 6, 7, 6, 5, 8, 7, 7, 7, 9, 9, 8, 10, 12, 20, 13, 12, 11, 11, 12, 25, 18, 19, 15, 20, 29,
    26, 31, 30, 29, 26, 28, 28, 32, 36, 46, 39, 32, 34, 44, 35, 28, 28, 40, 55, 41, 44, 48, 49, 52,
    52, 52, 31, 39, 57, 61, 56, 50, 60, 46, 51, 52, 50, 255, 192, 0, 11, 8, 0, 1, 0, 1, 1, 1, 17,
    0, 255, 196, 0, 20, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 255, 196, 0, 20, 16,
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 218, 0, 8, 1, 1, 0, 0, 63, 0, 84, 223,
    255, 217,
];

/// What the fake server received: request line path + X-Camapro-Token header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenRequest {
    pub path: String,
    pub token: Option<String>,
}

/// Read the request head (up to \r\n\r\n) and extract path + token.
pub fn read_head(stream: &mut TcpStream) -> SeenRequest {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte).unwrap();
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
        if buf.len() > 4096 {
            panic!("request head too large");
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let path = text
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_string();
    let token = text.lines().find_map(|l| {
        l.strip_prefix("X-Camapro-Token: ")
            .map(|t| t.trim().to_string())
    });
    SeenRequest { path, token }
}

pub fn multipart_body(frames: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..frames {
        out.extend_from_slice(b"--frame\r\nContent-Type: image/jpeg\r\n");
        out.extend_from_slice(format!("Content-Length: {}\r\n\r\n", TINY_JPEG.len()).as_bytes());
        out.extend_from_slice(TINY_JPEG);
        out.extend_from_slice(b"\r\n");
    }
    out
}

pub fn http200_multipart(body: &[u8]) -> Vec<u8> {
    let mut out =
        b"HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=frame\r\n\r\n"
            .to_vec();
    out.extend_from_slice(body);
    out
}

/// Accept one connection, capture its head, run `respond` (which owns the
/// response), keep the stream open until the handler returns.
pub fn spawn_fake<F>(requests: mpsc::Sender<SeenRequest>, respond: F) -> std::net::SocketAddr
where
    F: FnOnce(&mut TcpStream) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let seen = read_head(&mut stream);
        let _ = requests.send(seen);
        respond(&mut stream);
    });
    addr
}

/// Serve canned 200 + multipart body for ~300ms after responding, then close.
pub fn canned_server(blob: Vec<u8>) -> (std::net::SocketAddr, mpsc::Receiver<SeenRequest>) {
    let (tx, rx) = mpsc::channel();
    let addr = spawn_fake(tx, move |stream| {
        stream.write_all(&blob).unwrap();
        std::thread::sleep(Duration::from_millis(300));
    });
    (addr, rx)
}
