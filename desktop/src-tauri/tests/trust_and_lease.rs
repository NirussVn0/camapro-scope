use camapro_scope_lib::core::control::{EnrollmentValidator, QrPayload, TrustStore};
use camapro_scope_lib::core::session::{SessionController, SessionState};

#[test]
fn qr_enrollment_validates_and_rejects_expired() {
    let validator = EnrollmentValidator::new();
    let qr = QrPayload {
        version: 1,
        endpoint_hint: "192.168.1.50:8443".to_string(),
        peer_fingerprint_sha256: "SHA256:abcd".to_string(),
        secret: "secret-token".to_string(),
        expires_at_ms: 10_000,
    };

    // Before expiry
    assert!(validator.validate(&qr, 5_000).is_ok());

    // After expiry: fail-closed with unauthenticated
    let err = validator.validate(&qr, 10_001).unwrap_err();
    assert_eq!(err, "unauthenticated");
}

#[test]
fn trust_store_revokes_peer_and_rejects_unauthenticated_media() {
    let mut store = TrustStore::new();
    store.add_trusted_peer("SHA256:peer1");

    assert!(store.authenticate_control("SHA256:peer1").is_ok());

    // Revoke peer
    store.revoke_peer("SHA256:peer1");
    assert_eq!(
        store.authenticate_control("SHA256:peer1").unwrap_err(),
        "unauthenticated"
    );

    // Unauthenticated media
    assert_eq!(
        store.authenticate_media("SHA256:peer1", None).unwrap_err(),
        "unauthenticated"
    );
}

#[test]
fn session_controller_watchdog_expires_and_stops_streaming() {
    let mut session = SessionController::new(6_000); // 6s timeout
    session.on_connected("SHA256:peer1", 1_000);
    assert_eq!(session.state(), SessionState::Ready);

    // Explicit start
    session.start_stream(1_100).expect("start must succeed");
    assert_eq!(session.state(), SessionState::Streaming);

    // Heartbeat within 6s
    session.on_heartbeat(4_000);
    session.check_watchdog(5_000);
    assert_eq!(session.state(), SessionState::Streaming);

    // Expiry after > 6s from last heartbeat (4_000 + 6_000 = 10_000)
    session.check_watchdog(10_001);
    assert_eq!(session.state(), SessionState::Disconnected);
}

#[test]
fn reconnect_increments_generation_and_returns_ready_never_streaming() {
    let mut session = SessionController::new(6_000);
    session.on_connected("SHA256:peer1", 1_000);
    let gen1 = session.connection_generation();

    session.start_stream(1_100).unwrap();
    assert_eq!(session.state(), SessionState::Streaming);

    // Disconnect
    session.disconnect();
    assert_eq!(session.state(), SessionState::Disconnected);

    // Reconnect
    session.on_connected("SHA256:peer1", 5_000);
    let gen2 = session.connection_generation();
    assert!(gen2 > gen1, "generation must increment on reconnect");

    // Invariant D04: Reconnect returns Ready, never silently reactivates Streaming
    assert_eq!(session.state(), SessionState::Ready);

    // Stale event from gen1 must be ignored
    assert!(!session.accepts_generation_event(gen1));
    assert!(session.accepts_generation_event(gen2));
}
