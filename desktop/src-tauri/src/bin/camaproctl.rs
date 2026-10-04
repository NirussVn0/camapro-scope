//! Dedicated CLI binary for Camapro Scope (Gate G6).
//! Provides scripting and headless operations: status, preview, v4l2,
//! camera control, and profile management.

use camapro_scope_lib::core::commands::CameraSetPayload;
use camapro_scope_lib::core::profiles::{CameraProfile, ProfileStore, Resolution};
use std::env;
use std::process::exit;

fn print_usage() {
    eprintln!(
        r#"camaproctl — Camapro Scope command-line interface (v0.3.0)

USAGE:
    camaproctl <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    status                          Check connection status to Android phone
        [--host <HOST>]             Target host (default: 127.0.0.1)
        [--port <PORT>]             Target port (default: 8100)

    preview <HOST:PORT> <TOKEN>     Stream and preview frames headlessly
        [--secs <SECONDS>]          Duration in seconds (default: 15)

    v4l2                            Test V4L2 virtual camera loopback output
        [--secs <SECONDS>]          Duration in seconds (default: 10)

    control <HOST:PORT> <TOKEN>     Send camera control commands
        [--ev <STEPS>]              Exposure compensation steps (-4 to +4)
        [--iso <VALUE>]             Manual ISO sensitivity (e.g. 100, 400, 800)
        [--shutter <NANOS>]         Manual shutter time in nanoseconds
        [--focus <DIOPTERS>]        Manual focus distance (float diopters)

    profile <ACTION> [ARGS]         Manage camera profiles
        list                        List all saved profiles
        get <NAME>                  Display profile JSON
        save <NAME>                 Save profile atomically
            --width <W>             Width in pixels (e.g. 1920)
            --height <H>            Height in pixels (e.g. 1080)
            --fps <FPS>             Frames per second (e.g. 30, 60)
            [--camera <ID>]         Camera ID (default: "0")
        delete <NAME>               Delete profile by name

OPTIONS:
    -h, --help                      Print this help message
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        exit(1);
    }

    match args[1].as_str() {
        "-h" | "--help" | "help" => {
            print_usage();
            exit(0);
        }

        "status" => {
            let mut host = "127.0.0.1".to_string();
            let mut port = 8100u16;
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--host" && i + 1 < args.len() {
                    host = args[i + 1].clone();
                    i += 2;
                } else if args[i] == "--port" && i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(8100);
                    i += 2;
                } else {
                    i += 1;
                }
            }

            let addr = format!("{host}:{port}");
            match camapro_scope_lib::core::control::lan_tls::status(&addr) {
                Ok(()) => println!("status: online (authenticated TLS)"),
                Err(e) => {
                    eprintln!("Phone status failed: {e}");
                    exit(1);
                }
            }
        }

        "preview" => {
            if args.len() < 4 {
                eprintln!("usage: camaproctl preview <HOST:PORT> <TOKEN> [--secs <SECS>]");
                exit(1);
            }
            let addr = &args[2];
            let token = &args[3];
            let mut secs = 15u64;
            if args.len() >= 6 && args[4] == "--secs" {
                secs = args[5].parse().unwrap_or(15);
            }
            exit(camapro_scope_lib::preview_smoke(addr, token, secs));
        }

        "v4l2" => {
            let mut secs = 10u64;
            if args.len() >= 4 && args[2] == "--secs" {
                secs = args[3].parse().unwrap_or(10);
            }
            exit(camapro_scope_lib::virtual_output_smoke(secs));
        }

        "control" => {
            if args.len() < 4 {
                eprintln!("usage: camaproctl control <HOST:PORT> <TOKEN> [--ev <STEPS>] [--iso <ISO>] [--shutter <NANOS>] [--focus <DIOPTERS>]");
                exit(1);
            }
            let addr = &args[2];
            let token = &args[3];
            let (host, port) = match addr.rsplit_once(':') {
                Some((h, p)) => (h, p.parse::<u16>().unwrap_or(8100)),
                None => (addr.as_str(), 8100),
            };

            let mut changes = serde_json::Map::new();
            let mut i = 4;
            while i < args.len() {
                match args[i].as_str() {
                    "--ev" if i + 1 < args.len() => {
                        let val: i32 = args[i + 1].parse().unwrap_or(0);
                        changes.insert(
                            "exposureCompensationSteps".to_string(),
                            serde_json::json!(val),
                        );
                        i += 2;
                    }
                    "--iso" if i + 1 < args.len() => {
                        let val: u32 = args[i + 1].parse().unwrap_or(100);
                        changes.insert("iso".to_string(), serde_json::json!(val));
                        changes.insert("aeEnabled".to_string(), serde_json::json!(false));
                        i += 2;
                    }
                    "--shutter" if i + 1 < args.len() => {
                        let val: u64 = args[i + 1].parse().unwrap_or(33333333);
                        changes.insert("shutterNanos".to_string(), serde_json::json!(val));
                        changes.insert("aeEnabled".to_string(), serde_json::json!(false));
                        i += 2;
                    }
                    "--focus" if i + 1 < args.len() => {
                        let val: f64 = args[i + 1].parse().unwrap_or(0.0);
                        changes.insert("focusDistanceDiopters".to_string(), serde_json::json!(val));
                        i += 2;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }

            if changes.is_empty() {
                eprintln!("No controls specified! Provide at least one of --ev, --iso, --shutter, --focus");
                exit(1);
            }

            let mut dispatcher = camapro_scope_lib::core::commands::CommandDispatcher::new(
                camapro_scope_lib::core::session::SessionController::new(6_000),
                camapro_scope_lib::platform::linux::preview::NativePreviewSink::new(),
            );

            let payload = CameraSetPayload {
                camera_id: "0".to_string(),
                capability_revision: 1,
                changes: serde_json::Value::Object(changes),
            };

            match dispatcher.camera_set(host, port, token, payload) {
                Ok(res) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&res).unwrap_or_else(|_| res.to_string())
                    );
                    exit(0);
                }
                Err(e) => {
                    eprintln!("Failed to set controls: {e}");
                    exit(1);
                }
            }
        }

        "profile" => {
            if args.len() < 3 {
                eprintln!("usage: camaproctl profile <list|get|save|delete> [ARGS]");
                exit(1);
            }
            let store = ProfileStore::new(ProfileStore::default_dir());

            match args[2].as_str() {
                "list" => match store.list() {
                    Ok(names) => {
                        for name in names {
                            println!("{name}");
                        }
                    }
                    Err(e) => {
                        eprintln!("Error listing profiles: {e}");
                        exit(1);
                    }
                },
                "get" => {
                    if args.len() < 4 {
                        eprintln!("usage: camaproctl profile get <NAME>");
                        exit(1);
                    }
                    let name = &args[3];
                    match store.load(name) {
                        Ok(p) => {
                            println!("{}", serde_json::to_string_pretty(&p).unwrap());
                        }
                        Err(e) => {
                            eprintln!("Error reading profile '{name}': {e}");
                            exit(1);
                        }
                    }
                }
                "save" => {
                    if args.len() < 4 {
                        eprintln!("usage: camaproctl profile save <NAME> --width <W> --height <H> --fps <FPS> [--camera <ID>]");
                        exit(1);
                    }
                    let name = args[3].clone();
                    let mut width = 1920u32;
                    let mut height = 1080u32;
                    let mut fps = 30u32;
                    let mut camera_id = "0".to_string();

                    let mut i = 4;
                    while i < args.len() {
                        match args[i].as_str() {
                            "--width" if i + 1 < args.len() => {
                                width = args[i + 1].parse().unwrap_or(1920);
                                i += 2;
                            }
                            "--height" if i + 1 < args.len() => {
                                height = args[i + 1].parse().unwrap_or(1080);
                                i += 2;
                            }
                            "--fps" if i + 1 < args.len() => {
                                fps = args[i + 1].parse().unwrap_or(30);
                                i += 2;
                            }
                            "--camera" if i + 1 < args.len() => {
                                camera_id = args[i + 1].clone();
                                i += 2;
                            }
                            _ => {
                                i += 1;
                            }
                        }
                    }

                    let profile = CameraProfile {
                        name: name.clone(),
                        resolution: Resolution { width, height },
                        fps,
                        camera_id,
                        controls: Default::default(),
                    };

                    match store.save(&profile) {
                        Ok(()) => {
                            println!("✓ Profile '{name}' saved successfully.");
                        }
                        Err(e) => {
                            eprintln!("Error saving profile: {e}");
                            exit(1);
                        }
                    }
                }
                "delete" => {
                    if args.len() < 4 {
                        eprintln!("usage: camaproctl profile delete <NAME>");
                        exit(1);
                    }
                    let name = &args[3];
                    match store.delete(name) {
                        Ok(()) => {
                            println!("✓ Profile '{name}' deleted.");
                        }
                        Err(e) => {
                            eprintln!("Error deleting profile: {e}");
                            exit(1);
                        }
                    }
                }
                unknown => {
                    eprintln!("Unknown profile action: {unknown}");
                    exit(1);
                }
            }
        }

        unknown => {
            eprintln!("Unknown subcommand: {unknown}");
            print_usage();
            exit(1);
        }
    }
}
