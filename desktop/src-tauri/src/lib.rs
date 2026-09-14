pub mod core;
pub mod platform;

use core::commands::CommandDispatcher;
use core::session::SessionController;
use platform::linux::preview::NativePreviewSink;
use platform::linux::virtual_output::{OutputState, VirtualOutputError, DEFAULT_DEVICE};
use std::sync::Mutex;
use tauri::State;

/// Shared dispatcher state behind a mutex, same single-authority pattern as session commands (Invariant D05).
struct AppState(Mutex<CommandDispatcher>);

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let dispatcher =
        CommandDispatcher::new(SessionController::new(6_000), NativePreviewSink::new());
    tauri::Builder::default()
        .manage(AppState(Mutex::new(dispatcher)))
        .invoke_handler(tauri::generate_handler![
            virtual_output_start,
            virtual_output_stop,
            virtual_output_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
