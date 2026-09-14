//! T2 end-to-end: dispatcher preview_start → stream client → pump → gst
//! fakesink pipeline, against an in-process fake MJPEG server.

mod common;

use std::io::Write;
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

use camapro_scope_lib::core::commands::CommandDispatcher;
use camapro_scope_lib::core::session::SessionController;
use camapro_scope_lib::platform::linux::gst_preview::SinkMode;
use camapro_scope_lib::platform::linux::preview::NativePreviewSink;

use common::{http200_multipart, multipart_body, spawn_fake, TINY_JPEG};

/// Fake phone: serves N frames, one per 100ms, then holds the socket.
fn phone_server(frames: usize) -> std::net::SocketAddr {
    let (tx, _rx) = mpsc::channel();
    spawn_fake(tx, move |stream| {
        stream
            .write_all(&http200_multipart(&multipart_body(0)))
            .unwrap();
        let body = multipart_body(1);
        for _ in 0..frames {
            std::thread::sleep(Duration::from_millis(100));
            stream.write_all(&body).unwrap();
        }
        std::thread::sleep(Duration::from_secs(5));
    })
}

#[test]
fn preview_session_streams_frames_to_pipeline_and_stops_cleanly() {
    let addr = phone_server(10);
    let mut dispatcher =
        CommandDispatcher::new(SessionController::new(6_000), NativePreviewSink::new());

    dispatcher
        .preview_start_with_sink("127.0.0.1", addr.port(), "tok", SinkMode::Fake)
        .expect("preview start");
    assert!(dispatcher.preview_active());

    // ≥1 frame must flow through client → pump → gst within 5s.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while dispatcher.preview_frames() == 0 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(dispatcher.preview_frames() >= 1, "no frames flowed");

    dispatcher.preview_stop();
    assert!(!dispatcher.preview_active());
    dispatcher.preview_stop(); // idempotent
}

#[test]
fn preview_start_twice_is_rejected() {
    let addr = phone_server(3);
    let mut dispatcher =
        CommandDispatcher::new(SessionController::new(6_000), NativePreviewSink::new());

    dispatcher
        .preview_start_with_sink("127.0.0.1", addr.port(), "tok", SinkMode::Fake)
        .unwrap();
    let second =
        dispatcher.preview_start_with_sink("127.0.0.1", addr.port(), "tok", SinkMode::Fake);
    assert!(
        matches!(
            second,
            Err(camapro_scope_lib::core::commands::PreviewError::AlreadyRunning)
        ),
        "got {second:?}"
    );
    dispatcher.preview_stop();
}

#[test]
fn preview_start_with_dead_server_maps_connect_error() {
    let probe = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = probe.local_addr().unwrap();
    drop(probe);

    let mut dispatcher =
        CommandDispatcher::new(SessionController::new(6_000), NativePreviewSink::new());
    let err = dispatcher.preview_start_with_sink("127.0.0.1", addr.port(), "tok", SinkMode::Fake);
    assert!(
        matches!(
            err,
            Err(camapro_scope_lib::core::commands::PreviewError::Connect(_))
        ),
        "got {err:?}"
    );
    assert!(!dispatcher.preview_active());
}

/// The same valid-JPEG proof used by the gst tests, pinned here so the
/// pipeline contract (concatenated JPEGs decode) is visible at the glue level.
#[test]
fn tiny_jpeg_is_a_real_jpeg_pair() {
    assert_eq!(TINY_JPEG.len(), 160);
    assert_eq!(&TINY_JPEG[..2], &[0xFF, 0xD8]); // SOI
    assert_eq!(&TINY_JPEG[158..], &[0xFF, 0xD9]); // EOI
}
