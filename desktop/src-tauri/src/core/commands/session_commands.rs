use crate::core::media::stream_client::{StreamClient, StreamError};
use crate::core::session::{SessionController, SessionState};
use crate::platform::linux::gst_preview::{GstPreviewController, SinkMode};
use crate::platform::linux::preview::NativePreviewSink;
use crate::platform::linux::virtual_output::{
    OutputState, VirtualOutputController, VirtualOutputError,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionCommand {
    Start { now_ms: u64 },
    Stop,
}

/// G4 structural prep: typed camera control payload per protocol schema.
/// Inert until Android side implements camera.set handler over WSS.
/// ponytail: no wire transport yet; add when physical phone available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraSetPayload {
    #[serde(rename = "cameraId")]
    pub camera_id: String,
    #[serde(rename = "capabilityRevision")]
    pub capability_revision: u32,
    pub changes: serde_json::Value,
}

/// Central command dispatcher for desktop UI and future CLI.
/// Invariant D05: One desktop command authority; no parallel control paths.
pub struct CommandDispatcher {
    session: SessionController,
    preview: NativePreviewSink,
    virtual_output: VirtualOutputController,
    preview_session: Option<PreviewSession>,
    frames_seen: Arc<AtomicU64>,
}

/// Live MJPEG preview session: stream client (reader thread) + pump thread
/// that hands each frame to the gst pipeline's fifo. `preview_stop` stops
/// the client (channel disconnect), joins the pump, and the pump's exit
/// tears down the gst child + fifo.
struct PreviewSession {
    client: Arc<Mutex<StreamClient>>,
    pump: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PreviewError {
    AlreadyRunning,
    Connect(String),
    HttpStatus(u16),
    ServerEnd,
    SpawnFailed(String),
}

fn map_stream_err(e: StreamError) -> PreviewError {
    match e {
        StreamError::Connect(d) => PreviewError::Connect(d),
        StreamError::HttpStatus(s) => PreviewError::HttpStatus(s),
        StreamError::ServerEnd => PreviewError::ServerEnd,
    }
}

impl CommandDispatcher {
    pub fn new(session: SessionController, preview: NativePreviewSink) -> Self {
        Self {
            session,
            preview,
            virtual_output: VirtualOutputController::default(),
            preview_session: None,
            frames_seen: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn status(&self) -> SessionState {
        self.session.state()
    }

    pub fn generation(&self) -> u64 {
        self.session.connection_generation()
    }

    pub fn preview(&self) -> &NativePreviewSink {
        &self.preview
    }

    pub fn virtual_output_start(
        &mut self,
        device_path: &str,
    ) -> Result<OutputState, VirtualOutputError> {
        self.virtual_output.start(device_path)
    }

    pub fn virtual_output_stop(&mut self) {
        self.virtual_output.stop();
    }

    pub fn virtual_output_status(&mut self) -> OutputState {
        self.virtual_output.status()
    }

    pub fn on_peer_connected(&mut self, peer: &str, now_ms: u64) {
        self.session.on_connected(peer, now_ms);
        self.preview.stop();
    }

    pub fn accepts_generation_event(&self, gen: u64) -> bool {
        self.session.accepts_generation_event(gen)
    }

    pub fn dispatch(&mut self, cmd: SessionCommand) -> Result<(), &'static str> {
        match cmd {
            SessionCommand::Start { now_ms } => {
                self.session.start_stream(now_ms)?;
                self.preview.start()?;
                Ok(())
            }
            SessionCommand::Stop => {
                self.preview.stop();
                self.session.stop_stream()?;
                Ok(())
            }
        }
    }

    /// Start the T2 preview: MJPEG stream client → pump → gst pipeline
    /// (production Wayland sink).
    pub fn preview_start(
        &mut self,
        host: &str,
        port: u16,
        token: &str,
    ) -> Result<(), PreviewError> {
        self.preview_start_with_sink(host, port, token, SinkMode::Wayland)
    }

    pub fn preview_start_with_sink(
        &mut self,
        host: &str,
        port: u16,
        token: &str,
        sink: SinkMode,
    ) -> Result<(), PreviewError> {
        if self.preview_session.is_some() {
            return Err(PreviewError::AlreadyRunning);
        }
        let addr = format!("{host}:{port}");
        let (client, frames) =
            StreamClient::start(addr, token.to_string()).map_err(map_stream_err)?;
        let client = Arc::new(Mutex::new(client));

        // Unique per session (pid + counter): parallel tests/processes never
        // share a path. A stale file from a crashed run is removed by
        // GstPreviewController::start before mkfifo.
        static SESSION_SEQ: AtomicU64 = AtomicU64::new(0);
        let seq = SESSION_SEQ.fetch_add(1, Ordering::SeqCst);
        let fifo = std::env::temp_dir().join(format!(
            "camapro-preview-{}-{}.fifo",
            std::process::id(),
            seq
        ));
        let mut gst = GstPreviewController::new();
        gst.start(fifo.to_str().unwrap(), sink)
            .map_err(|e| PreviewError::SpawnFailed(e.to_string()))?;

        self.frames_seen.store(0, Ordering::SeqCst);
        let counter = Arc::clone(&self.frames_seen);
        let pump = std::thread::spawn(move || {
            loop {
                match frames.recv_timeout(Duration::from_millis(200)) {
                    Ok(frame) => {
                        counter.fetch_add(1, Ordering::SeqCst);
                        let _ = gst.write(&frame);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    // stream client ended (server closed or stop()) → teardown
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            gst.stop();
        });
        self.preview_session = Some(PreviewSession {
            client,
            pump: Some(pump),
        });
        Ok(())
    }

    /// Stop the preview session. Idempotent (no-op from idle).
    pub fn preview_stop(&mut self) {
        if let Some(mut session) = self.preview_session.take() {
            // stop() joins the reader thread → sender dropped → pump breaks.
            if let Ok(mut client) = session.client.lock() {
                client.stop();
            }
            if let Some(pump) = session.pump.take() {
                let _ = pump.join();
            }
        }
    }

    /// Frames received from the phone since the last preview_start.
    pub fn preview_frames(&self) -> u64 {
        self.frames_seen.load(Ordering::SeqCst)
    }

    pub fn preview_active(&self) -> bool {
        self.preview_session.is_some()
    }

    /// Dispatch live camera control changes to the phone via POST /control.
    /// Invariant D05: One desktop command authority; mutual typed response.
    pub fn camera_set(
        &mut self,
        host: &str,
        port: u16,
        token: &str,
        payload: CameraSetPayload,
    ) -> Result<serde_json::Value, String> {
        use std::io::{Read, Write};
        use std::net::TcpStream;
        use std::time::Duration;

        let addr = format!("{host}:{port}");
        let socket_addr = addr
            .parse()
            .map_err(|e| format!("Invalid address {addr}: {e}"))?;

        let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_millis(3000))
            .map_err(|e| format!("Cannot reach phone at {addr}: {e}"))?;

        stream.set_read_timeout(Some(Duration::from_millis(3000))).ok();
        stream.set_write_timeout(Some(Duration::from_millis(3000))).ok();

        let req_body = serde_json::json!({
            "v": 1,
            "id": self.generation() + 1,
            "type": "camera.set",
            "payload": payload
        });
        let body_str = req_body.to_string();

        let req = format!(
            "POST /control HTTP/1.1\r\n\
             Host: {host}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             X-Camapro-Token: {token}\r\n\
             Connection: close\r\n\r\n\
             {body_str}",
            body_str.len()
        );

        stream.write_all(req.as_bytes()).map_err(|e| e.to_string())?;

        let mut resp = String::new();
        stream.read_to_string(&mut resp).map_err(|e| e.to_string())?;

        if let Some(idx) = resp.find("\r\n\r\n") {
            let (headers, body_part) = resp.split_at(idx + 4);
            if headers.starts_with("HTTP/1.1 200") {
                let parsed: serde_json::Value = serde_json::from_str(body_part.trim())
                    .map_err(|e| format!("Invalid JSON response: {e}"))?;
                if parsed.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                    Ok(parsed)
                } else {
                    Err(format!("Phone returned error: {parsed}"))
                }
            } else {
                Err(format!("Phone returned error status: {headers}"))
            }
        } else {
            Err(format!("Invalid HTTP response: {resp}"))
        }
    }
}
