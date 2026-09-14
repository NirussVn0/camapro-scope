//! T2 preview pipeline controller: supervised `gst-launch-1.0` reading
//! concatenated JPEG frames from a FIFO. Mirrors `virtual_output.rs`
//! (subprocess controller pattern); the decode chain was chosen by
//! MEASUREMENT on this host (T2 report):
//!
//! `filesrc location=<fifo> ! jpegparse ! jpegdec ! videoconvert !
//!  video/x-raw,format=I420,framerate=30/1 ! <sink>`
//!
//! Why filesrc-on-fifo and not the brief's `fdsrc`+`cat` sketch: measured,
//! every fdsrc variant failed (caps negotiation / preroll errors), while
//! filesrc on a FIFO delivered 3/3 decoded buffers. `parsejpeg` from the
//! brief does not exist here; the element is `jpegparse`.
//!
//! Invariant D06: raw frames never pass through React IPC; the pipeline owns
//! decode+display, the UI only talks to this controller.

use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

/// Where the decoded video goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkMode {
    /// Real window (needs a Wayland session).
    Wayland,
    /// Headless/CI-safe sink that still runs the full decode chain.
    Fake,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GstPreviewError {
    AlreadyRunning,
    SpawnFailed(String),
}

impl std::fmt::Display for GstPreviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GstPreviewError::AlreadyRunning => write!(f, "preview pipeline already running"),
            GstPreviewError::SpawnFailed(d) => write!(f, "failed to spawn gst-launch-1.0: {d}"),
        }
    }
}

/// How long one frame may take to hand to the fifo before it is dropped.
const WRITE_TIMEOUT: Duration = Duration::from_millis(200);

/// Controller over one preview session. `start`/`stop` are the session
/// boundary; `write` is called from the pump loop with each JPEG frame.
///
/// Frame flow (measured on this host): the child's filesrc opens the fifo
/// read end within ms of spawn; the controller then keeps ONE persistent
/// write fd open so the reader never sees EOF between frames, and if the
/// child dies every write/reopen fails with EPIPE instead of hanging.
// ponytail: mkfifo spawned via coreutils to honor the zero-deps constraint;
// swap to libc::mkfifo the moment a libc dep is approved.
pub struct GstPreviewController {
    child: Option<Child>,
    fifo_path: Option<PathBuf>,
    writer: Option<File>,
    running: Arc<AtomicBool>,
    frames_written: Arc<Mutex<u64>>,
}

impl Default for GstPreviewController {
    fn default() -> Self {
        Self::new()
    }
}

impl GstPreviewController {
    pub fn new() -> Self {
        Self {
            child: None,
            fifo_path: None,
            writer: None,
            running: Arc::new(AtomicBool::new(false)),
            frames_written: Arc::new(Mutex::new(0)),
        }
    }

    /// Create the fifo and spawn the decode pipeline on it. Blocks until
    /// the child's filesrc has actually opened the read end (proven by a
    /// successful O_NONBLOCK writer-open), so a pipeline that dies on
    /// startup surfaces as `SpawnFailed` instead of a silent black fifo.
    pub fn start(&mut self, fifo_path: &str, sink: SinkMode) -> Result<(), GstPreviewError> {
        if self.active() {
            return Err(GstPreviewError::AlreadyRunning);
        }
        let fifo = Path::new(fifo_path);
        let _ = std::fs::remove_file(fifo);
        let mkfifo = Command::new("mkfifo")
            .arg(fifo)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| GstPreviewError::SpawnFailed(format!("mkfifo: {e}")))?;
        if !mkfifo.success() {
            return Err(GstPreviewError::SpawnFailed(
                "mkfifo exited nonzero".to_string(),
            ));
        }

        let mut child = match spawn_gst(fifo_path, sink) {
            Ok(child) => child,
            Err(e) => {
                let _ = std::fs::remove_file(fifo);
                return Err(GstPreviewError::SpawnFailed(e.to_string()));
            }
        };

        // O_NONBLOCK open returns ENXIO instantly while no reader exists;
        // success therefore proves filesrc attached to the fifo.
        let deadline = Instant::now() + Duration::from_secs(2);
        let writer = loop {
            match open_writer_nonblocking(fifo) {
                Ok(f) => break f,
                Err(_) => {
                    if let Ok(Some(_)) = child.try_wait() {
                        let _ = std::fs::remove_file(fifo);
                        return Err(GstPreviewError::SpawnFailed(
                            "gst-launch exited before consuming the fifo".to_string(),
                        ));
                    }
                    if Instant::now() > deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        let _ = std::fs::remove_file(fifo);
                        return Err(GstPreviewError::SpawnFailed(
                            "pipeline never opened the fifo".to_string(),
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        };

        self.child = Some(child);
        self.fifo_path = Some(fifo.to_path_buf());
        self.writer = Some(writer);
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Hand one frame to the pipeline. Returns false when the frame is
    /// dropped (no pipeline, failed open, or a write that stalls beyond the
    /// cap). A dropped frame is a preview hiccup, never a fatal error.
    // ponytail: a dead child unblocks via EPIPE (all readers gone), but a
    // live child with a full 64 KiB pipe buffer can block the pump until
    // stop(). Upgrade path: O_NONBLOCK + poll once libc is approved.
    pub fn write(&mut self, frame: &[u8]) -> bool {
        if !self.active() {
            return false;
        }
        let path = match &self.fifo_path {
            Some(p) => p.clone(),
            None => return false,
        };
        if self.writer.is_none() {
            self.writer = OpenOptions::new().write(true).open(&path).ok();
        }
        let file = match &mut self.writer {
            Some(w) => w,
            None => return false,
        };
        if write_capped(file, frame, WRITE_TIMEOUT) {
            *self.frames_written.lock().unwrap() += 1;
            true
        } else {
            self.writer = None; // broken/stalled: next write reopens
            false
        }
    }

    /// Frames handed to the fifo since the last start (test observable).
    pub fn frames_written(&self) -> u64 {
        *self.frames_written.lock().unwrap()
    }

    /// True while the spawned child is alive (polls once).
    pub fn active(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            if let Ok(Some(_)) = child.try_wait() {
                self.child = None;
                self.writer = None;
                self.running.store(false, Ordering::SeqCst);
            }
        }
        self.running.load(Ordering::SeqCst)
    }

    /// Kill the child, close the writer, remove the fifo. Idempotent.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.writer = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(path) = self.fifo_path.take() {
            let _ = std::fs::remove_file(&path);
        }
    }
}

impl Drop for GstPreviewController {
    fn drop(&mut self) {
        self.stop();
    }
}

fn spawn_gst(fifo_path: &str, sink: SinkMode) -> std::io::Result<Child> {
    let sink_element = match sink {
        SinkMode::Wayland => "waylandsink",
        SinkMode::Fake => "fakesink",
    };
    Command::new("gst-launch-1.0")
        .arg("-q")
        .arg("filesrc")
        .arg(format!("location={fifo_path}"))
        .arg("!")
        .arg("jpegparse")
        .arg("!")
        .arg("jpegdec")
        .arg("!")
        .arg("videoconvert")
        .arg("!")
        // Measured: jpegdec emits per-frame variable caps; the fully-fixed
        // I420 filter keeps negotiation stable across the whole stream.
        .arg("video/x-raw,format=I420,framerate=30/1")
        .arg("!")
        .arg(sink_element)
        .arg("sync=false")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

/// O_NONBLOCK fifo write-end open: instant ENXIO while no reader exists,
/// so success proves the child attached. O_NONBLOCK also makes writes
/// EAGAIN instead of blocking forever on a full pipe.
fn open_writer_nonblocking(fifo: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .custom_flags(0o4000) // O_NONBLOCK
        .open(fifo)
}

fn write_capped(file: &mut File, frame: &[u8], timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let mut buf = frame;
    loop {
        if buf.is_empty() {
            return true;
        }
        match file.write(buf) {
            Ok(0) => return false,
            Ok(n) => buf = &buf[n..],
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return false,
        }
        if Instant::now() > deadline {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_mentions_cause() {
        assert!(GstPreviewError::AlreadyRunning
            .to_string()
            .contains("running"));
        assert!(GstPreviewError::SpawnFailed("x".into())
            .to_string()
            .contains("gst-launch"));
    }

    /// Real waylandsink smoke mirroring the G3.2 `#[ignore]` pattern:
    /// `cargo test -- --ignored` with a Wayland session present.
    #[test]
    #[ignore = "spawns gst-launch with waylandsink; needs a Wayland session"]
    fn real_waylandsink_smoke() {
        let fifo = std::env::temp_dir().join("camapro-t2-wayland-smoke");
        let _ = std::fs::remove_file(&fifo);
        let mut ctrl = GstPreviewController::new();
        ctrl.start(fifo.to_str().unwrap(), SinkMode::Wayland)
            .expect("start waylandsink pipeline");
        std::thread::sleep(Duration::from_millis(700));

        // 1x1 JPEG, decodability proven in the T2 decode-chain measurement.
        let jpeg: &[u8] = &[
            255, 216, 255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 1, 0, 96, 0, 96, 0, 0, 255, 219, 0,
            67, 0, 8, 6, 6, 7, 6, 5, 8, 7, 7, 7, 9, 9, 8, 10, 12, 20, 13, 12, 11, 11, 12, 25, 18,
            19, 15, 20, 29, 26, 31, 30, 29, 26, 28, 28, 32, 36, 46, 39, 32, 34, 44, 35, 28, 28, 40,
            55, 41, 44, 48, 49, 52, 52, 52, 31, 39, 57, 61, 56, 50, 60, 46, 51, 52, 50, 255, 192,
            0, 11, 8, 0, 1, 0, 1, 1, 1, 17, 0, 255, 196, 0, 20, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 9, 255, 196, 0, 20, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 255, 218, 0, 8, 1, 1, 0, 0, 63, 0, 84, 223, 255, 217,
        ];
        for _ in 0..3 {
            assert!(ctrl.write(jpeg), "frame must reach the fifo");
        }
        std::thread::sleep(Duration::from_millis(1000));
        assert!(ctrl.active(), "waylandsink pipeline died mid-stream");
        assert_eq!(ctrl.frames_written(), 3);

        ctrl.stop();
        assert!(!ctrl.active());
        assert!(!fifo.exists());
    }
}
