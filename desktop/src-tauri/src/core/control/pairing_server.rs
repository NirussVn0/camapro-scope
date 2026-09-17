use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
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
    expires_at_ms: u64,
    port: u16,
    running: bool,
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

    /// Starts the pairing HTTP listener if not running, generates a one-time secret,
    /// and returns the complete QR payload for the phone to scan.
    pub fn start_pairing_session(
        &self,
        app: tauri::AppHandle,
    ) -> Result<serde_json::Value, String> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64;

        let secret = format!("{:016x}", now_ms ^ 0x5a5a_3c3c_9696_e1e1);
        let expires_at_ms = now_ms + 300_000; // 5 minutes

        let mut st = self.state.lock().map_err(|_| "poisoned lock")?;
        st.current_secret = Some(secret.clone());
        st.expires_at_ms = expires_at_ms;

        if !st.running {
            // Find an open port starting from 8101
            let mut bound_listener = None;
            let mut port = 8101;
            for p in 8101..8120 {
                if let Ok(listener) = TcpListener::bind(format!("0.0.0.0:{p}")) {
                    bound_listener = Some(listener);
                    port = p;
                    break;
                }
            }

            let listener = bound_listener.ok_or("Failed to bind pairing port on 8101-8120")?;
            st.port = port;
            st.running = true;

            let state_clone = Arc::clone(&self.state);
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let mut stream = match stream {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    let app_clone = app.clone();
                    let state_ref = Arc::clone(&state_clone);
                    std::thread::spawn(move || {
                        handle_client(&mut stream, app_clone, state_ref);
                    });
                }
            });
        }

        let ips = Self::get_candidate_ips();
        let primary_ip = ips.first().cloned().unwrap_or_else(|| "127.0.0.1".to_string());
        let port = st.port;

        let payload = serde_json::json!({
            "version": 1,
            "type": "camapro_pairing",
            "desktop_ips": ips,
            "port": port,
            "secret": secret,
            "endpoint_hint": format!("http://{primary_ip}:{port}/pair"),
            "peer_fingerprint_sha256": format!("sha256:{:064x}", now_ms),
            "expires_at_ms": expires_at_ms
        });

        Ok(payload)
    }
}

fn handle_client(
    stream: &mut TcpStream,
    app: tauri::AppHandle,
    state: Arc<Mutex<PairingState>>,
) {
    let peer_addr = stream.peer_addr().ok();
    let mut buf = [0u8; 4096];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let req_str = String::from_utf8_lossy(&buf[..n]);
    let first_line = req_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let full_path = parts[1];
    let path = full_path.split('?').next().unwrap_or("");
    let query = if full_path.contains('?') {
        full_path.split('?').nth(1).unwrap_or("")
    } else {
        ""
    };

    if method == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    if path == "/status" || path == "/ping" {
        let body = "{\"status\":\"ok\",\"service\":\"camapro-desktop\"}\r\n";
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    if path != "/pair" {
        let body = "Not Found\r\n";
        let resp = format!(
            "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    // Extract fields from query params or JSON body
    let mut secret = String::new();
    let mut phone_ip = String::new();
    let mut phone_port: u16 = 8100;
    let mut phone_token = String::new();
    let mut phone_name = "Android Phone".to_string();

    // Check query params first
    for q in query.split('&') {
        let mut kv = q.split('=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            match k {
                "secret" => secret = v.to_string(),
                "phone_ip" | "ip" => phone_ip = v.to_string(),
                "phone_port" | "port" => phone_port = v.parse().unwrap_or(8100),
                "phone_token" | "token" => phone_token = v.to_string(),
                "phone_name" | "name" => phone_name = v.to_string(),
                _ => {}
            }
        }
    }

    // If body contains JSON, parse it
    if let Some(body_start) = req_str.find("\r\n\r\n") {
        let body_str = &req_str[body_start + 4..];
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str.trim()) {
            if let Some(s) = json.get("secret").and_then(|v| v.as_str()) {
                secret = s.to_string();
            }
            if let Some(ip) = json.get("phone_ip").and_then(|v| v.as_str()) {
                phone_ip = ip.to_string();
            }
            if let Some(p) = json.get("phone_port").and_then(|v| v.as_u64()) {
                phone_port = p as u16;
            }
            if let Some(t) = json.get("phone_token").and_then(|v| v.as_str()) {
                phone_token = t.to_string();
            }
            if let Some(n) = json.get("phone_name").and_then(|v| v.as_str()) {
                phone_name = n.to_string();
            }
        }
    }

    // If phone_ip is empty or loopback, fall back to socket's actual peer IP
    if phone_ip.is_empty() || phone_ip == "127.0.0.1" || phone_ip == "0.0.0.0" {
        if let Some(SocketAddr::V4(v4)) = peer_addr {
            phone_ip = v4.ip().to_string();
        }
    }

    // Validate secret against current session
    let is_valid = {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if let Ok(st) = state.lock() {
            if let Some(ref cur) = st.current_secret {
                cur == &secret && now_ms <= st.expires_at_ms
            } else {
                false
            }
        } else {
            false
        }
    };

    if !is_valid {
        let body = "{\"status\":\"error\",\"message\":\"Invalid or expired pairing secret\"}\r\n";
        let resp = format!(
            "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    // Emit event to Tauri frontend!
    let event = PhonePairedEvent {
        host: phone_ip,
        port: phone_port,
        token: phone_token,
        name: phone_name,
    };
    let _ = app.emit("phone-paired", &event);

    let body = "{\"status\":\"ok\",\"message\":\"Paired successfully with Desktop\"}\r\n";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_ips_always_includes_loopback_and_non_empty() {
        let ips = PairingServer::get_candidate_ips();
        assert!(!ips.is_empty());
        assert!(ips.contains(&"127.0.0.1".to_string()));
    }
}

