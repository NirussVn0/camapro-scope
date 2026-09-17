pub mod core;
pub mod platform;

use core::commands::{CameraSetPayload, CommandDispatcher};
use core::control::PairingServer;
use core::session::SessionController;
use platform::linux::preview::NativePreviewSink;
use platform::linux::virtual_output::{OutputState, VirtualOutputError, DEFAULT_DEVICE};
use std::sync::Mutex;
use tauri::State;

/// Shared dispatcher state behind a mutex, same single-authority pattern as session commands (Invariant D05).
struct AppState(Mutex<CommandDispatcher>);

struct PairingServerState(PairingServer);

#[tauri::command]
fn virtual_output_start(
    state: State<AppState>,
    device_path: Option<String>,
) -> Result<OutputState, String> {
    let mut dispatcher = state.0.lock().map_err(|_| "state poisoned")?;
    let path = device_path.unwrap_or_else(|| DEFAULT_DEVICE.to_string());
    dispatcher
        .virtual_output_start(&path)
        .map_err(|e| match e {
            VirtualOutputError::DeviceMissing { device, remediation } => {
                format!("Virtual camera device {device} not found. Remediation (run manually):\n{remediation}")
            }
            VirtualOutputError::AlreadyRunning => "Virtual output already running".to_string(),
            VirtualOutputError::SpawnFailed(detail) => {
                format!("Failed to spawn gst-launch-1.0: {detail}")
            }
        })
}

#[tauri::command]
fn virtual_output_stop(state: State<AppState>) -> Result<OutputState, String> {
    let mut dispatcher = state.0.lock().map_err(|_| "state poisoned")?;
    dispatcher.virtual_output_stop();
    Ok(dispatcher.virtual_output_status())
}

#[tauri::command]
fn virtual_output_status(state: State<AppState>) -> Result<OutputState, String> {
    let mut dispatcher = state.0.lock().map_err(|_| "state poisoned")?;
    Ok(dispatcher.virtual_output_status())
}

#[tauri::command]
fn preview_start(
    state: State<AppState>,
    host: String,
    port: u16,
    token: String,
) -> Result<(), String> {
    let mut dispatcher = state.0.lock().map_err(|_| "state poisoned")?;
    dispatcher
        .preview_start(&host, port, &token)
        .map_err(|e| format!("{e:?}"))
}

#[tauri::command]
fn preview_stop(state: State<AppState>) -> Result<(), String> {
    let mut dispatcher = state.0.lock().map_err(|_| "state poisoned")?;
    dispatcher.preview_stop();
    Ok(())
}

#[tauri::command]
fn preview_status(state: State<AppState>) -> Result<serde_json::Value, String> {
    let dispatcher = state.0.lock().map_err(|_| "state poisoned")?;
    Ok(serde_json::json!({
        "active": dispatcher.preview_active(),
        "frames": dispatcher.preview_frames(),
    }))
}

#[tauri::command]
fn check_phone_status(host: String, port: u16) -> Result<serde_json::Value, String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let addr = format!("{host}:{port}");
    let socket_addr = addr
        .parse()
        .map_err(|e| format!("Invalid address {addr}: {e}"))?;

    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_millis(2000))
        .map_err(|e| format!("Cannot reach phone at {addr}: {e}"))?;

    stream.set_read_timeout(Some(Duration::from_millis(2000))).ok();
    stream.set_write_timeout(Some(Duration::from_millis(2000))).ok();

    let req = format!("GET /status HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).map_err(|e| e.to_string())?;

    let mut resp = String::new();
    stream.read_to_string(&mut resp).map_err(|e| e.to_string())?;

    if resp.contains("200 OK") {
        Ok(serde_json::json!({ "online": true, "host": host, "port": port }))
    } else {
        Err(format!("Phone returned unexpected response: {resp}"))
    }
}

/// G4 structural prep: accepts typed camera.set payload per protocol schema.
/// Returns Ok but does nothing until WSS transport + Android handler exist.
/// ponytail: inert stub; wire when physical phone available.
#[tauri::command]
fn camera_set(_state: State<AppState>, payload: CameraSetPayload) -> Result<(), String> {
    // Validate payload shape (serde already did); log for future wiring.
    eprintln!(
        "camera.set received (inert): camera={} rev={} changes={}",
        payload.camera_id, payload.capability_revision, payload.changes
    );
    Ok(())
}

/// Generate real 2-way pairing session with local IP discovery and HTTP callback.
#[tauri::command]
fn generate_pairing_qr(
    app: tauri::AppHandle,
    state: State<PairingServerState>,
) -> Result<serde_json::Value, String> {
    state.0.start_pairing_session(app)
}

/// Headless T2 preview smoke: stream from `host:port` with `token`, pump
/// frames through the waylandsink pipeline for `secs`, exit 0 only if at
/// least one frame flowed. Prints frame count + fps.
pub fn preview_smoke(addr: &str, token: &str, secs: u64) -> i32 {
    let mut dispatcher =
        CommandDispatcher::new(SessionController::new(6_000), NativePreviewSink::new());
    let (host, port) = match addr.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(8100)),
        None => {
            eprintln!("usage: --preview-smoke <host:port> <token> <secs>");
            return 2;
        }
    };
    if let Err(e) = dispatcher.preview_start(host, port, token) {
        eprintln!("preview_start failed: {e:?}");
        return 1;
    }
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_secs(secs) {
        if dispatcher.preview_frames() > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let frames = dispatcher.preview_frames();
    let secs_f = started.elapsed().as_secs_f64().max(0.001);
    let fps = frames as f64 / secs_f;
    dispatcher.preview_stop();
    println!("preview frames={frames} fps={fps:.1}");
    if frames >= 1 {
        0
    } else {
        eprintln!("no frames flowed within {secs}s");
        1
    }
}

/// Headless run of the production virtual-output path: spawn gst-launch,
/// keep it running `secs` seconds, stop, release. Exit 0 = success.
pub fn virtual_output_smoke(secs: u64) -> i32 {
    use platform::linux::virtual_output::{GstLaunchSpawner, OutputState, VirtualOutputController};
    let mut ctrl = VirtualOutputController::new(Box::new(GstLaunchSpawner));
    match ctrl.start(DEFAULT_DEVICE) {
        Ok(OutputState::Running) => {}
        Ok(other) => {
            eprintln!("unexpected state after start: {other:?}");
            return 1;
        }
        Err(e) => {
            eprintln!("start failed: {e:?}");
            return 1;
        }
    }
    println!("virtual output running on {DEFAULT_DEVICE} for {secs}s");
    std::thread::sleep(std::time::Duration::from_secs(secs));
    if !ctrl.is_running() {
        eprintln!("gst-launch exited before the smoke window ended");
        return 1;
    }
    ctrl.stop();
    println!("virtual output stopped cleanly");
    0
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let dispatcher =
        CommandDispatcher::new(SessionController::new(6_000), NativePreviewSink::new());
    let pairing_server = PairingServer::new();
    tauri::Builder::default()
        .manage(AppState(Mutex::new(dispatcher)))
        .manage(PairingServerState(pairing_server))
        .invoke_handler(tauri::generate_handler![
            virtual_output_start,
            virtual_output_stop,
            virtual_output_status,
            preview_start,
            preview_stop,
            preview_status,
            camera_set,
            generate_pairing_qr,
            check_phone_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
