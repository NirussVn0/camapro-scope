use crate::core::session::{SessionController, SessionState};
use crate::platform::linux::preview::NativePreviewSink;
use crate::platform::linux::virtual_output::{
    OutputState, VirtualOutputController, VirtualOutputError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionCommand {
    Start { now_ms: u64 },
    Stop,
}

/// Central command dispatcher for desktop UI and future CLI.
/// Invariant D05: One desktop command authority; no parallel control paths.
pub struct CommandDispatcher {
    session: SessionController,
    preview: NativePreviewSink,
    virtual_output: VirtualOutputController,
}

impl CommandDispatcher {
    pub fn new(session: SessionController, preview: NativePreviewSink) -> Self {
        Self {
            session,
            preview,
            virtual_output: VirtualOutputController::default(),
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
}
