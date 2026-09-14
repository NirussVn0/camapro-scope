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
    // Headless T2/T3 smoke: stream MJPEG from <host:port> with <token>.
    if std::env::args().nth(1).as_deref() == Some("--preview-smoke") {
        let args: Vec<String> = std::env::args().collect();
        let secs: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(15);
        let (addr, token) = match (args.get(2), args.get(3)) {
            (Some(a), Some(t)) => (a.as_str(), t.as_str()),
            _ => {
                eprintln!("usage: --preview-smoke <host:port> <token> [secs]");
                std::process::exit(2);
            }
        };
        std::process::exit(camapro_scope_lib::preview_smoke(addr, token, secs));
    }
    camapro_scope_lib::run()
}
