use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrPayload {
    pub version: u32,
    pub endpoint_hint: String,
    pub peer_fingerprint_sha256: String,
    pub secret: String,
    pub expires_at_ms: u64,
}

#[derive(Default)]
pub struct EnrollmentValidator {
    consumed_secrets: HashSet<String>,
}

impl EnrollmentValidator {
    pub fn new() -> Self {
        Self {
            consumed_secrets: HashSet::new(),
        }
    }

    pub fn validate(&self, qr: &QrPayload, now_ms: u64) -> Result<(), &'static str> {
        if self.consumed_secrets.contains(&qr.secret) {
            return Err("unauthenticated");
        }
        if now_ms > qr.expires_at_ms {
            return Err("unauthenticated");
        }
        Ok(())
    }

    pub fn consume(&mut self, qr: &QrPayload, now_ms: u64) -> Result<(), &'static str> {
        self.validate(qr, now_ms)?;
        self.consumed_secrets.insert(qr.secret.clone());
        Ok(())
    }
}

#[derive(Default)]
pub struct TrustStore {
    trusted_peers: HashSet<String>,
    revoked_peers: HashSet<String>,
}

impl TrustStore {
    pub fn new() -> Self {
        Self {
            trusted_peers: HashSet::new(),
            revoked_peers: HashSet::new(),
        }
    }

    pub fn add_trusted_peer(&mut self, fingerprint: &str) {
        self.revoked_peers.remove(fingerprint);
        self.trusted_peers.insert(fingerprint.to_string());
    }

    pub fn revoke_peer(&mut self, fingerprint: &str) {
        self.trusted_peers.remove(fingerprint);
        self.revoked_peers.insert(fingerprint.to_string());
    }

    pub fn authenticate_control(&self, fingerprint: &str) -> Result<(), &'static str> {
        if self.revoked_peers.contains(fingerprint) || !self.trusted_peers.contains(fingerprint) {
            return Err("unauthenticated");
        }
        Ok(())
    }

    pub fn authenticate_media(
        &self,
        fingerprint: &str,
        media_token: Option<&str>,
    ) -> Result<(), &'static str> {
        if media_token.is_none() {
            return Err("unauthenticated");
        }
        if self.revoked_peers.contains(fingerprint) || !self.trusted_peers.contains(fingerprint) {
            return Err("unauthenticated");
        }
        Ok(())
    }
}
