use super::lan_tls::{self, Identity, Peer};
use std::io::{Read, Write};
use std::net::{TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::Emitter;

/// Phone pairing payload received from the phone when scanning desktop QR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhonePairedEvent {
    pub host: String,
    pub port: u16,
    pub token: String,
    pub name: String,
}

#[derive(Default)]
struct PairingState {
    current_secret: Option<String>,
    deadline: Option<Instant>,
    port: u16,
    running: bool,
    pending: Option<PendingEnrollment>,
}

struct PendingEnrollment {
    host: String,
    enrollment: Enrollment,
    deadline: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PairingRoute {
    Prepare,
    Confirm,
}

impl PairingState {
    fn issue(&mut self, secret: String, now: Instant) {
        self.current_secret = Some(secret);
        self.pending = None;
        self.deadline = Some(now + Duration::from_secs(300));
    }
    fn process(
        &mut self,
        route: PairingRoute,
        req: Enrollment,
        host: String,
        now: Instant,
        probe: impl FnOnce(&str, &Peer, bool) -> Result<(), String>,
        finish: impl FnOnce(PhonePairedEvent, Peer) -> Result<(), String>,
    ) -> Result<(), String> {
        let deadline = match route {
            PairingRoute::Prepare => {
                let expires = self.deadline.ok_or("No active QR")?;
                if !self.consume(&req.secret, now) {
                    return Err("Invalid or consumed QR".into());
                }
                expires.min(now + Duration::from_secs(30))
            }
            PairingRoute::Confirm => {
                let pending = self.pending.as_ref().ok_or("No pending enrollment")?;
                if now >= pending.deadline {
                    self.pending = None;
                    return Err("Completion expired".into());
                }
                if pending.host != host || pending.enrollment != req {
                    return Err("Completion binding mismatch".into());
                }
                // One matching completion attempt; failure also burns it, never replay.
                self.pending.take().ok_or("No pending enrollment")?.deadline
            }
        };
        let addr = format!("{host}:{}", req.phone_port);
        let peer = Peer {
            pin: req.phone_fingerprint_sha256.clone(),
            token: req.phone_token.clone(),
        };
        probe(&addr, &peer, route == PairingRoute::Confirm)?;
        if Instant::now() >= deadline {
            return Err("Enrollment expired during probe".into());
        }
        match route {
            PairingRoute::Prepare => {
                self.pending = Some(PendingEnrollment {
                    host,
                    enrollment: req,
                    deadline,
                });
                Ok(()) // Initial HTTP 200 authorizes phone commit; not desktop success.
            }
            PairingRoute::Confirm => finish(
                PhonePairedEvent {
                    host,
                    port: req.phone_port,
                    token: req.phone_token,
                    name: req.phone_name,
                },
                peer,
            ),
        }
    }

    fn consume(&mut self, secret: &str, now: Instant) -> bool {
        if self.deadline.is_some_and(|deadline| now < deadline)
            && self.current_secret.as_deref() == Some(secret)
        {
            self.current_secret = None;
            self.deadline = None;
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Default)]
pub struct PairingServer {
    state: Arc<Mutex<PairingState>>,
}

impl PairingServer {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(PairingState::default())),
        }
    }

    /// Discovers all candidate IPv4 addresses for the desktop (LAN, Tailscale, Wi-Fi).
    pub fn get_candidate_ips() -> Vec<String> {
        let mut ips = Vec::new();

        // 1. Check network interfaces via `ip -4 -o addr show`
        if let Ok(output) = std::process::Command::new("ip")
            .args(["-4", "-o", "addr", "show"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(pos) = parts.iter().position(|&x| x == "inet") {
                        if let Some(cidr) = parts.get(pos + 1) {
                            let ip = cidr.split('/').next().unwrap_or("");
                            if !ip.is_empty()
                                && ip != "127.0.0.1"
                                && !ip.starts_with("172.17.")
                                && !ip.starts_with("172.18.")
                                && !ips.contains(&ip.to_string())
                            {
                                ips.push(ip.to_string());
                            }
                        }
                    }
                }
            }
        }

        // 2. Outbound route discovery via UDP
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    let ip = addr.ip().to_string();
                    if ip != "0.0.0.0" && ip != "127.0.0.1" && !ips.contains(&ip) {
                        ips.push(ip);
                    }
                }
            }
        }

        // 3. Always include 127.0.0.1 for local/ADB loopback
        if !ips.contains(&"127.0.0.1".to_string()) {
            ips.push("127.0.0.1".to_string());
        }

        ips
    }

    /// TLS callback enrollment. QR binds the desktop certificate and a one-time secret.
    pub fn start_pairing_session(
        &self,
        app: tauri::AppHandle,
    ) -> Result<serde_json::Value, String> {
        let identity = Identity::load()?;
        let tls_config = identity.server_config()?;
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64;
        let secret = lan_tls::random_secret()?;
        let mut st = self.state.lock().map_err(|_| "poisoned lock")?;
        if !st.running {
            let listener = (8101..8120)
                .find_map(|port| TcpListener::bind(("0.0.0.0", port)).ok())
                .ok_or("Failed to bind pairing port")?;
            st.port = listener.local_addr().map_err(|e| e.to_string())?.port();
            st.running = true;
            let state = Arc::clone(&self.state);
            let identity_clone = identity.clone();
            std::thread::spawn(move || {
                // Bounded single enrollment worker; a client cannot spawn arbitrary threads.
                for socket in listener.incoming().flatten() {
                    let Ok(_deadline) = super::socket_deadline::SocketDeadline::new(
                        &socket,
                        Duration::from_secs(5),
                    ) else {
                        continue;
                    };
                    let _ = socket.set_read_timeout(Some(Duration::from_secs(3)));
                    let _ = socket.set_write_timeout(Some(Duration::from_secs(3)));
                    if let Ok(conn) = rustls::ServerConnection::new(Arc::clone(&tls_config)) {
                        let peer_ip = socket.peer_addr().ok().map(|a| a.ip());
                        let mut stream = rustls::StreamOwned::new(conn, socket);
                        let outcome = read_enrollment(&mut stream).and_then(|(route, req)| {
                            let mut guard = state
                                .lock()
                                .map_err(|_| "Trust state poisoned".to_string())?;
                            // The callback socket identifies the phone's reachable LAN interface.
                            let host = peer_ip.ok_or("Missing peer address")?.to_string();
                            guard.process(
                                route,
                                req,
                                host,
                                Instant::now(),
                                |addr, peer, ready| {
                                    if ready {
                                        lan_tls::probe_ready(addr, peer, &identity_clone)
                                    } else {
                                        lan_tls::probe(addr, peer, &identity_clone)
                                    }
                                },
                                |event, peer| {
                                    let addr = format!("{}:{}", event.host, event.port);
                                    lan_tls::remember_peer(
                                        &addr,
                                        peer,
                                        identity_clone.clone(),
                                        true,
                                    )?;
                                    app.emit("phone-paired", &event).map_err(|e| e.to_string())
                                },
                            )
                        });
                        let (status, body) = if outcome.is_ok() {
                            ("200 OK", "{\"status\":\"ok\"}")
                        } else {
                            ("401 Unauthorized", "{\"status\":\"error\",\"message\":\"Enrollment rejected; start phone and scan a fresh QR\"}")
                        };
                        let _ = stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
                        stream.conn.send_close_notify();
                        let _ = stream.flush();
                    }
                }
            });
        }
        st.issue(secret.clone(), Instant::now());
        let ips = Self::get_candidate_ips();
        let primary_ip = ips.first().cloned().unwrap_or_else(|| "127.0.0.1".into());
        let port = st.port;
        Ok(serde_json::json!({
            "version": 1, "type": "camapro_pairing", "desktop_ips": ips, "port": port,
            "secret": secret, "endpoint_hint": format!("https://{primary_ip}:{port}/pair"),
            "peer_fingerprint_sha256": lan_tls::fingerprint(&identity.cert),
            "expires_at_ms": now_ms + 300_000
        }))
    }
}

#[derive(Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Enrollment {
    secret: String,
    phone_port: u16,
    phone_token: String,
    phone_name: String,
    phone_fingerprint_sha256: String,
}

fn read_enrollment(stream: &mut impl Read) -> Result<(PairingRoute, Enrollment), String> {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= 8192 {
            return Err("Header too large".into());
        }
        let mut byte = [0];
        stream.read_exact(&mut byte).map_err(|e| e.to_string())?;
        head.push(byte[0]);
    }
    let head = std::str::from_utf8(&head).map_err(|_| "Invalid header")?;
    let route = match head.lines().next() {
        Some("POST /pair HTTP/1.1") => PairingRoute::Prepare,
        Some("POST /pair/confirm HTTP/1.1") => PairingRoute::Confirm,
        _ => return Err("POST JSON required".into()),
    };
    let mut lengths = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"));
    let length: usize = lengths
        .next()
        .ok_or("Missing Content-Length")?
        .1
        .trim()
        .parse()
        .map_err(|_| "Invalid length")?;
    if lengths.next().is_some()
        || length == 0
        || length > 4096
        || head
            .lines()
            .any(|l| l.to_ascii_lowercase().starts_with("transfer-encoding:"))
    {
        return Err("Invalid body framing".into());
    }
    let mut body = vec![0; length];
    stream.read_exact(&mut body).map_err(|e| e.to_string())?;
    let req: Enrollment = serde_json::from_slice(&body).map_err(|_| "Invalid enrollment JSON")?;
    if req.phone_port == 0
        || req.secret.len() != 64
        || !lan_tls::valid_pin(&req.phone_fingerprint_sha256)
        || req.phone_token.len() != 64
        || !req.phone_token.bytes().all(|b| b.is_ascii_hexdigit())
        || req.phone_name.len() > 128
    {
        return Err("Invalid enrollment fields".into());
    }
    Ok((route, req))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn enrollment() -> Enrollment {
        Enrollment {
            secret: "ab".repeat(32),
            phone_port: 8100,
            phone_token: "cd".repeat(32),
            phone_name: "Phone".into(),
            phone_fingerprint_sha256: format!("sha256:{}", "ef".repeat(32)),
        }
    }

    fn prepared(now: Instant) -> PairingState {
        let req = enrollment();
        let mut state = PairingState {
            current_secret: Some(req.secret.clone()),
            deadline: Some(now + Duration::from_secs(300)),
            ..Default::default()
        };
        state
            .process(
                PairingRoute::Prepare,
                req,
                "192.168.1.2".into(),
                now,
                |_, _, ready| {
                    assert!(!ready);
                    Ok(())
                },
                |_, _| panic!("Initial HTTP 200 must not persist or emit success"),
            )
            .unwrap();
        state
    }

    #[test]
    fn prepare_never_finalizes_confirm_requires_fresh_ready_probe_and_is_single_use() {
        let now = Instant::now();
        let mut state = prepared(now);
        assert!(state.current_secret.is_none());
        assert_eq!(
            state.pending.as_ref().unwrap().deadline,
            now + Duration::from_secs(30)
        );
        assert!(state
            .process(
                PairingRoute::Prepare,
                enrollment(),
                "192.168.1.2".into(),
                now,
                |_, _, _| panic!("QR replay must fail before probe"),
                |_, _| panic!("Replay must not finalize")
            )
            .is_err());
        let calls = std::cell::Cell::new(0);
        state
            .process(
                PairingRoute::Confirm,
                enrollment(),
                "192.168.1.2".into(),
                now,
                |addr, peer, ready| {
                    assert_eq!(addr, "192.168.1.2:8100");
                    assert_eq!(peer.pin, enrollment().phone_fingerprint_sha256);
                    assert!(ready);
                    calls.set(1);
                    Ok(())
                },
                |event, _| {
                    assert_eq!(calls.get(), 1);
                    assert_eq!(event.host, "192.168.1.2");
                    calls.set(2);
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(calls.get(), 2);
        assert!(state.pending.is_none());
        assert!(state
            .process(
                PairingRoute::Confirm,
                enrollment(),
                "192.168.1.2".into(),
                now,
                |_, _, _| panic!("Completion replay"),
                |_, _| panic!("Duplicate success")
            )
            .is_err());
    }

    #[test]
    fn completion_binds_socket_peer_pin_port_token_and_original_secret() {
        let now = Instant::now();
        let mut state = prepared(now);
        for field in 0..5 {
            let mut req = enrollment();
            let mut host = "192.168.1.2".to_string();
            match field {
                0 => host = "192.168.1.3".into(),
                1 => req.phone_fingerprint_sha256 = format!("sha256:{}", "11".repeat(32)),
                2 => req.phone_port += 1,
                3 => req.phone_token = "22".repeat(32),
                _ => req.secret = "33".repeat(32),
            }
            assert!(state
                .process(
                    PairingRoute::Confirm,
                    req,
                    host,
                    now,
                    |_, _, _| panic!("Wrong binding must fail before probe"),
                    |_, _| panic!("Wrong binding must not finalize")
                )
                .is_err());
            assert!(state.pending.is_some());
        }
    }

    #[test]
    fn expired_missing_and_not_ready_completion_cannot_finalize() {
        let now = Instant::now();
        let mut state = prepared(now);
        assert!(state
            .process(
                PairingRoute::Confirm,
                enrollment(),
                "192.168.1.2".into(),
                now + Duration::from_secs(30),
                |_, _, _| panic!("Expired completion"),
                |_, _| panic!("Expired success")
            )
            .is_err());
        assert!(state.pending.is_none());
        state = prepared(now);
        assert!(state
            .process(
                PairingRoute::Confirm,
                enrollment(),
                "192.168.1.2".into(),
                now,
                |_, _, ready| {
                    assert!(ready);
                    Err("Phone is still provisional".into())
                },
                |_, _| panic!("No success before ready")
            )
            .is_err());
        assert!(state.pending.is_none());
        assert!(state
            .process(
                PairingRoute::Confirm,
                enrollment(),
                "192.168.1.2".into(),
                now,
                |_, _, _| panic!("No pending capability"),
                |_, _| panic!("No pending success")
            )
            .is_err());
    }

    #[test]
    fn completion_never_extends_original_qr_expiry() {
        let now = Instant::now();
        let mut state = PairingState {
            current_secret: Some(enrollment().secret),
            deadline: Some(now + Duration::from_secs(1)),
            ..Default::default()
        };
        state
            .process(
                PairingRoute::Prepare,
                enrollment(),
                "192.168.1.2".into(),
                now,
                |_, _, _| Ok(()),
                |_, _| panic!("Prepare cannot finalize"),
            )
            .unwrap();
        assert_eq!(
            state.pending.as_ref().unwrap().deadline,
            now + Duration::from_secs(1)
        );
    }

    #[test]
    fn fresh_qr_invalidates_old_pending_completion() {
        let now = Instant::now();
        let mut state = prepared(now);
        state.issue("44".repeat(32), now);
        assert!(state.pending.is_none());
        assert!(state
            .process(
                PairingRoute::Confirm,
                enrollment(),
                "192.168.1.2".into(),
                now,
                |_, _, _| panic!("Old confirmation cannot probe"),
                |_, _| panic!("Old success")
            )
            .is_err());
    }

    #[test]
    fn parser_accepts_only_exact_private_prepare_and_confirm_routes() {
        let req = enrollment();
        let body = serde_json::json!({"secret": req.secret, "phone_port": req.phone_port, "phone_token": req.phone_token, "phone_name": req.phone_name, "phone_fingerprint_sha256": req.phone_fingerprint_sha256}).to_string();
        for path in ["/pair", "/pair/confirm"] {
            let message = format!(
                "POST {path} HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            assert!(read_enrollment(&mut message.as_bytes()).is_ok());
        }
        let message = format!(
            "POST /pair/confirm?secret=bad HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        assert!(read_enrollment(&mut message.as_bytes()).is_err());
    }
    #[test]
    fn enrollment_secret_is_single_use_and_expiring() {
        let now = Instant::now();
        let mut state = PairingState {
            current_secret: Some("secret".into()),
            deadline: Some(now + Duration::from_secs(1)),
            ..Default::default()
        };
        assert!(!state.consume("wrong", now));
        assert!(state.consume("secret", now));
        assert!(!state.consume("secret", now));
        state.current_secret = Some("secret".into());
        state.deadline = Some(now);
        assert!(!state.consume("secret", now));
    }
    #[test]
    fn enrollment_parser_rejects_query_and_oversize_and_truncated_body() {
        for input in [
            "GET /pair?secret=x HTTP/1.1\r\n\r\n",
            "POST /pair HTTP/1.1\r\nContent-Length: 4097\r\n\r\n",
            "POST /pair HTTP/1.1\r\nContent-Length: 2\r\n\r\n{",
        ] {
            assert!(read_enrollment(&mut input.as_bytes()).is_err());
        }
    }
    #[test]
    fn candidate_ips_always_includes_loopback_and_non_empty() {
        let ips = PairingServer::get_candidate_ips();
        assert!(!ips.is_empty());
        assert!(ips.contains(&"127.0.0.1".to_string()));
    }
}
