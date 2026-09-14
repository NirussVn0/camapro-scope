//! G3.2 virtual camera output: supervised `gst-launch-1.0` subprocess writing
//! synthetic frames to a v4l2loopback device.
//!
//! Canon: docs/PLATFORMS.md — GStreamer → v4l2sink → existing v4l2loopback.
//! Missing/busy device is reported with the exact remediation string; this code
//! NEVER installs modules or elevates permissions.

use serde::Serialize;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};

pub const DEFAULT_DEVICE: &str = "/dev/video0";

/// Exact host remediation for a missing loopback device. Reported, never auto-run.
pub const DEVICE_REMEDIATION: &str =
    "sudo modprobe v4l2loopback devices=1 video_label=\"Camapro Scope\" card_label=\"Camapro Scope Virtual Camera\"";

/// ponytail: num-buffers is bounded (30fps * 600s = 18000) so a forgotten
/// session self-releases the device after at most ~10 minutes. If real phone
/// frames arrive later, raise/remove this ceiling with a proper session stop.
const NUM_BUFFERS: u32 = 30 * 600;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum VirtualOutputError {
    DeviceMissing { device: String, remediation: String },
    AlreadyRunning,
    SpawnFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OutputState {
    Idle,
    Running,
    /// Child exited on its own (buffers exhausted or crashed); stop() returns to Idle.
    Stopped,
}

/// Handle over a spawned child process (seam for tests: no real process in unit tests).
pub trait ChildHandle: Send {
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>>;
    fn kill(&mut self) -> std::io::Result<()>;
}

/// Process spawner seam (tests inject fakes; production spawns gst-launch-1.0).
pub trait Spawner: Send {
    fn spawn(&self, device_path: &str) -> Result<Box<dyn ChildHandle>, std::io::Error>;
}

struct RealChild {
    child: Child,
}

impl ChildHandle for RealChild {
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }
}

/// Production spawner: `gst-launch-1.0 -q videotestsrc num-buffers=18000 !
/// video/x-raw,width=640,height=480,framerate=30/1 ! v4l2sink device=<path>`
pub struct GstLaunchSpawner;

impl Spawner for GstLaunchSpawner {
    fn spawn(&self, device_path: &str) -> Result<Box<dyn ChildHandle>, std::io::Error> {
        let child = Command::new("gst-launch-1.0")
            .arg("-q")
            .arg("videotestsrc")
            .arg(format!("num-buffers={NUM_BUFFERS}"))
            // gst-launch parses each argv token as one pipeline element;
            // element+props must be separate args.
            .arg("!")
            .arg("video/x-raw,width=640,height=480,framerate=30/1")
            .arg("!")
            .arg("v4l2sink")
            .arg(format!("device={device_path}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Box::new(RealChild { child }))
    }
}

pub struct VirtualOutputController {
    spawner: Box<dyn Spawner>,
    state: OutputState,
    child: Option<Box<dyn ChildHandle>>,
}

impl Default for VirtualOutputController {
    fn default() -> Self {
        Self::new(Box::new(GstLaunchSpawner))
    }
}

impl VirtualOutputController {
    pub fn new(spawner: Box<dyn Spawner>) -> Self {
        Self {
            spawner,
            state: OutputState::Idle,
            child: None,
        }
    }

    pub fn state(&self) -> OutputState {
        self.state
    }

    pub fn start(&mut self, device_path: &str) -> Result<OutputState, VirtualOutputError> {
        if !Path::new(device_path).exists() {
            return Err(VirtualOutputError::DeviceMissing {
                device: device_path.to_string(),
                remediation: DEVICE_REMEDIATION.to_string(),
            });
        }
        self.poll_child();
        if self.state != OutputState::Idle {
            return Err(VirtualOutputError::AlreadyRunning);
        }
        let child = self
            .spawner
            .spawn(device_path)
            .map_err(|e| VirtualOutputError::SpawnFailed(e.to_string()))?;
        self.child = Some(child);
        self.state = OutputState::Running;
        Ok(self.state)
    }

    /// Idempotent from any state; kills the child if still alive, returns to Idle.
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
        self.state = OutputState::Idle;
    }

    /// Polls the child once and normalizes state when it exited on its own.
    pub fn is_running(&mut self) -> bool {
        self.poll_child();
        self.state == OutputState::Running
    }

    /// Polled status for the UI command path.
    pub fn status(&mut self) -> OutputState {
        let _ = self.is_running();
        self.state
    }

    fn poll_child(&mut self) {
        if let Some(child) = &mut self.child {
            if let Ok(Some(_)) = child.try_wait() {
                self.child = None;
                self.state = OutputState::Stopped;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    /// Fake child: returns Ok(None) on the first poll, exited(0) afterwards.
    struct FakeChild {
        polls: u32,
    }

    impl ChildHandle for FakeChild {
        fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
            self.polls += 1;
            if self.polls < 2 {
                Ok(None)
            } else {
                Ok(Some(ExitStatus::from_raw(0)))
            }
        }

        fn kill(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct FakeSpawner;

    impl Spawner for FakeSpawner {
        fn spawn(&self, _device_path: &str) -> Result<Box<dyn ChildHandle>, std::io::Error> {
            Ok(Box::new(FakeChild { polls: 0 }))
        }
    }

    fn controller() -> VirtualOutputController {
        VirtualOutputController::new(Box::new(FakeSpawner))
    }

    fn unique_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(name)
    }

    #[test]
    fn start_with_missing_device_reports_remediation() {
        let device = unique_path("camapro-g32-missing-device");
        let _ = std::fs::remove_file(&device);

        let mut c = controller();
        let err = c.start(device.to_str().unwrap()).unwrap_err();
        match err {
            VirtualOutputError::DeviceMissing {
                device: d,
                remediation,
            } => {
                assert!(d.contains("camapro-g32-missing-device"));
                assert!(remediation.contains("modprobe v4l2loopback"));
            }
            other => panic!("expected DeviceMissing, got {other:?}"),
        }
        assert_eq!(c.state(), OutputState::Idle);
    }

    #[test]
    fn double_start_is_rejected_as_busy() {
        let device = unique_path("camapro-g32-existing-device");
        std::fs::write(&device, b"").unwrap();

        let mut c = controller();
        let first = c.start(device.to_str().unwrap());
        assert_eq!(first, Ok(OutputState::Running));
        assert_eq!(
            c.start(device.to_str().unwrap()),
            Err(VirtualOutputError::AlreadyRunning)
        );

        let _ = std::fs::remove_file(&device);
    }

    #[test]
    fn stop_is_idempotent_from_any_state() {
        let device = unique_path("camapro-g32-stop-device");
        std::fs::write(&device, b"").unwrap();

        let mut c = controller();
        c.stop(); // stop from idle: ok, stays idle
        assert_eq!(c.state(), OutputState::Idle);

        c.start(device.to_str().unwrap()).unwrap();
        c.stop();
        assert_eq!(c.state(), OutputState::Idle);
        c.stop(); // second stop: still ok
        assert_eq!(c.state(), OutputState::Idle);

        let _ = std::fs::remove_file(&device);
    }

    /// Real-process smoke: exercises the actual GstLaunchSpawner pipeline
    /// string against the loopback device. Opt-in: cargo test -- --ignored.
    #[test]
    #[ignore = "spawns gst-launch; run manually with v4l2loopback loaded"]
    fn real_spawner_smoke() {
        let device = DEFAULT_DEVICE;
        if !Path::new(device).exists() {
            panic!("{device} missing — load v4l2loopback first");
        }
        let mut c = VirtualOutputController::new(Box::new(GstLaunchSpawner));
        assert_eq!(c.start(device), Ok(OutputState::Running));
        std::thread::sleep(std::time::Duration::from_millis(1500));
        assert!(c.is_running(), "gst-launch exited immediately");
        c.stop();
        assert_eq!(c.state(), OutputState::Idle);
    }

    #[test]
    fn child_self_exit_flips_is_running_to_false() {
        let device = unique_path("camapro-g32-selfexit-device");
        std::fs::write(&device, b"").unwrap();

        let mut c = controller();
        c.start(device.to_str().unwrap()).unwrap();
        assert!(c.is_running()); // poll 1: still alive
        assert!(!c.is_running()); // poll 2: child exited → self-release
        assert_eq!(c.state(), OutputState::Stopped);

        let _ = std::fs::remove_file(&device);
    }
}
