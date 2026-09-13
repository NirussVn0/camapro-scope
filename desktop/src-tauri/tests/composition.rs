use camapro_scope_lib::core::commands::{CommandDispatcher, SessionCommand};
use camapro_scope_lib::core::session::{SessionController, SessionState};
use camapro_scope_lib::platform::linux::preview::NativePreviewSink;

#[test]
fn command_dispatcher_starts_and_stops_cleanly() {
    let session = SessionController::new(6_000);
    let preview = NativePreviewSink::new();
    let mut dispatcher = CommandDispatcher::new(session, preview);

    // Initial status
    assert_eq!(dispatcher.status(), SessionState::Disconnected);

    // Connect peer
    dispatcher.on_peer_connected("SHA256:phone", 1_000);
    assert_eq!(dispatcher.status(), SessionState::Ready);

    // Dispatch Start
    let start_res = dispatcher.dispatch(SessionCommand::Start { now_ms: 1_100 });
    assert!(start_res.is_ok());
    assert_eq!(dispatcher.status(), SessionState::Streaming);
    assert!(dispatcher.preview().is_active());

    // Dispatch Stop
    let stop_res = dispatcher.dispatch(SessionCommand::Stop);
    assert!(stop_res.is_ok());
    assert_eq!(dispatcher.status(), SessionState::Ready);
    assert!(!dispatcher.preview().is_active());

    // Re-dispatch Stop: idempotent no-op
    let stop_res2 = dispatcher.dispatch(SessionCommand::Stop);
    assert!(stop_res2.is_ok());
    assert_eq!(dispatcher.status(), SessionState::Ready);
}

#[test]
fn repeated_reopen_does_not_leak_stale_generation() {
    let session = SessionController::new(6_000);
    let preview = NativePreviewSink::new();
    let mut dispatcher = CommandDispatcher::new(session, preview);

    dispatcher.on_peer_connected("SHA256:phone", 1_000);
    let gen1 = dispatcher.generation();

    dispatcher
        .dispatch(SessionCommand::Start { now_ms: 1_100 })
        .unwrap();
    dispatcher.dispatch(SessionCommand::Stop).unwrap();

    // Reconnect peer
    dispatcher.on_peer_connected("SHA256:phone", 5_000);
    let gen2 = dispatcher.generation();
    assert!(gen2 > gen1, "generation must advance");

    // Stale event from gen1 must be rejected
    assert!(!dispatcher.accepts_generation_event(gen1));
    assert!(dispatcher.accepts_generation_event(gen2));

    // Second start cycle works cleanly
    dispatcher
        .dispatch(SessionCommand::Start { now_ms: 5_100 })
        .unwrap();
    assert_eq!(dispatcher.status(), SessionState::Streaming);
    assert!(dispatcher.preview().is_active());
}
