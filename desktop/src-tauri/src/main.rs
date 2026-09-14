#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Headless smoke mode (G3 acceptance): exercises the exact production
    // start path (VirtualOutputController → GstLaunchSpawner → /dev/video0)
    // without needing a display or UI click.
    if std::env::args().nth(1).as_deref() == Some("--virtual-output-smoke") {
        let secs: u64 = std::env::args()
            .nth(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        std::process::exit(camapro_scope_lib::virtual_output_smoke(secs));
    }
    camapro_scope_lib::run()
}
