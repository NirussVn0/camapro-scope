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

#[test]
fn camera_set_dispatches_http_post_and_parses_response() {
    use camapro_scope_lib::core::commands::CameraSetPayload;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 2048];
        let n = stream.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);
        assert!(req.starts_with("POST /control HTTP/1.1"));
        assert!(req.contains("X-Camapro-Token: test-token"));
        assert!(req.contains("\"type\":\"camera.set\""));

        let res_body = r#"{"v":1,"id":1,"type":"response","ok":true,"result":{"applied":{"exposureCompensationSteps":2}}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n\
             {}",
            res_body.len(),
            res_body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    let session = SessionController::new(6_000);
    let preview = NativePreviewSink::new();
    let mut dispatcher = CommandDispatcher::new(session, preview);

    let payload = CameraSetPayload {
        camera_id: "0".to_string(),
        capability_revision: 1,
        changes: serde_json::json!({ "exposureCompensationSteps": 2 }),
    };

    let res = dispatcher.camera_set("127.0.0.1", port, "test-token", payload);
    assert!(res.is_ok(), "camera_set failed: {:?}", res.err());
    let res_json = res.unwrap();
    assert_eq!(res_json["ok"], true);
    assert_eq!(res_json["result"]["applied"]["exposureCompensationSteps"], 2);

    server_thread.join().unwrap();
}

