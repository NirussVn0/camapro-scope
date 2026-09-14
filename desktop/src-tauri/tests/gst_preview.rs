//! T2 integration tests for the GStreamer preview controller.
//! SinkMode::Fake spawns a real `gst-launch-1.0 ... ! fakesink` pipeline —
//! headless-safe (no display needed), proven working on this host.

mod common;

use std::path::PathBuf;
use std::time::Duration;

use camapro_scope_lib::platform::linux::gst_preview::{
    GstPreviewController, GstPreviewError, SinkMode,
};
use common::TINY_JPEG;

fn unique_fifo(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("camapro-t2-{name}"))
}

#[test]
fn start_creates_fifo_stop_removes_it() {
    let fifo = unique_fifo("start-stop");
    let _ = std::fs::remove_file(&fifo);

    let mut ctrl = GstPreviewController::default();
    ctrl.start(fifo.to_str().unwrap(), SinkMode::Fake).unwrap();
    assert!(fifo.exists(), "start must mkfifo");
    ctrl.stop();
    assert!(!fifo.exists(), "stop must remove the fifo");
    assert!(!ctrl.active());
}

#[test]
fn double_start_is_rejected_as_busy() {
    let fifo = unique_fifo("double-start");
    let _ = std::fs::remove_file(&fifo);

    let mut ctrl = GstPreviewController::default();
    ctrl.start(fifo.to_str().unwrap(), SinkMode::Fake).unwrap();
    let err = ctrl
        .start(fifo.to_str().unwrap(), SinkMode::Fake)
        .unwrap_err();
    assert!(
        matches!(err, GstPreviewError::AlreadyRunning),
        "got {err:?}"
    );
    ctrl.stop();
}

#[test]
fn stop_is_idempotent_from_any_state() {
    let mut ctrl = GstPreviewController::default();
    ctrl.stop(); // from idle: ok
    assert!(!ctrl.active());

    let fifo = unique_fifo("stop-idem");
    let _ = std::fs::remove_file(&fifo);
    ctrl.start(fifo.to_str().unwrap(), SinkMode::Fake).unwrap();
    ctrl.stop();
    ctrl.stop();
    assert!(!ctrl.active());
}

#[test]
fn write_without_pipeline_drops_frame_without_panic() {
    let mut ctrl = GstPreviewController::default();
    let accepted = ctrl.write(TINY_JPEG);
    assert!(!accepted, "no pipeline -> frame dropped, no panic");
    ctrl.stop();
}

/// Feed 3 tiny valid JPEGs through the real fakesink pipeline: bytes must be
/// consumed (child alive & frames registered within 3s), then stop releases.
#[test]
fn fake_sink_pipeline_consumes_three_frames_and_stop_releases() {
    let fifo = unique_fifo("drain");
    let _ = std::fs::remove_file(&fifo);

    let mut ctrl = GstPreviewController::default();
    ctrl.start(fifo.to_str().unwrap(), SinkMode::Fake).unwrap();

    // filesrc opens the fifo lazily; give the pipeline a moment to preroll.
    std::thread::sleep(Duration::from_millis(500));
    for _ in 0..3 {
        ctrl.write(TINY_JPEG);
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while ctrl.frames_written() < 3 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(ctrl.frames_written(), 3, "all bytes written to fifo");
    assert!(
        ctrl.active(),
        "gst fakesink pipeline died — fifo bytes not consumed"
    );

    ctrl.stop();
    assert!(!ctrl.active());
    assert!(!fifo.exists());
}
