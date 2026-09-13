use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Disconnected,
    Ready,
    Streaming,
    Error,
}

pub struct SessionController {
    state: SessionState,
    timeout_ms: u64,
    last_heartbeat_ms: u64,
    active_peer: Option<String>,
    connection_generation: u64,
}

impl SessionController {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            state: SessionState::Disconnected,
            timeout_ms,
            last_heartbeat_ms: 0,
            active_peer: None,
            connection_generation: 0,
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn connection_generation(&self) -> u64 {
        self.connection_generation
    }

    pub fn on_connected(&mut self, peer_fingerprint: &str, now_ms: u64) {
        self.connection_generation += 1;
        self.active_peer = Some(peer_fingerprint.to_string());
        self.last_heartbeat_ms = now_ms;
        // Invariant D04: Reconnect returns Ready, never silently reactivates Streaming
        self.state = SessionState::Ready;
    }

    pub fn start_stream(&mut self, now_ms: u64) -> Result<(), &'static str> {
        if self.state != SessionState::Ready {
            return Err("invalid_state");
        }
        self.last_heartbeat_ms = now_ms;
        self.state = SessionState::Streaming;
        Ok(())
    }

    pub fn stop_stream(&mut self) -> Result<(), &'static str> {
        if self.state == SessionState::Streaming {
            self.state = SessionState::Ready;
        }
        Ok(())
    }

    pub fn on_heartbeat(&mut self, now_ms: u64) {
        self.last_heartbeat_ms = now_ms;
    }

    pub fn check_watchdog(&mut self, now_ms: u64) {
        if self.state == SessionState::Disconnected {
            return;
        }
        if now_ms.saturating_sub(self.last_heartbeat_ms) > self.timeout_ms {
            self.disconnect();
        }
    }

    pub fn disconnect(&mut self) {
        self.state = SessionState::Disconnected;
        self.active_peer = None;
    }

    pub fn accepts_generation_event(&self, event_generation: u64) -> bool {
        event_generation == self.connection_generation
    }
}
