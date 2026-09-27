---
type: Platform Specification
status: Active (v0.2.0 Linux & Android Implemented)
version: 0.2.0
last_updated: 2026-09-19
owner: NirussVn0
authority: docs/PLATFORMS.md
---

# Platform boundaries and support evidence

Linux (Wayland) and Android (minSdk 26) are the primary active platforms with v0.2.0 core implementations. Windows remains planned for G8.

---

## 1. Platform Support Matrix

| Platform Surface | Target Technology | Status | Key Requirements & Verification |
|---|---|---|---|
| **Android Sender** | Kotlin, Camera2, Foreground Service | **Implemented** | minSdk 26, targetSdk 34, JDK 21. Real-device thermal/battery endurance (G5). |
| **Linux Desktop** | Tauri 2, Rust, GStreamer Waylandsink | **Implemented** | CachyOS / Arch Linux x86_64, Wayland compositor, GStreamer 1.28+. |
| **Linux Virtual Camera**| `v4l2loopback`, `v4l2sink` (`/dev/video*`) | **Implemented** | Auto-detects loopback device (`/dev/video11`); requires user-provisioned kernel module. |
| **Windows Desktop/Sink**| Media Foundation Virtual Camera | **Planned (G8)**| Windows 11 build 22000+, MF virtual camera API, portable core. |
| **CLI / Global Hotkeys**| Wayland compositor binding / IPC | **Planned (G6)**| Interacts with desktop session controller; no duplicate daemon. |

---

## 2. Platform Details

### Android (Sender)
* **SDK Levels:** `minSdk = 26` (Android 8.0 Oreo), `targetSdk = 34` (Android 14).
* **Capture Engine:** Exclusive Camera2 ownership in `CameraEngine.kt`. Surfaces managed via `Camera2Source.kt`.
* **Foreground Service:** Camera streaming runs inside an Android Foreground Service (`CameraStreamService.kt`) with type `camera` to survive app backgrounding and screen-off.
* **Network & Pairing:** Local IP resolution supports Wi-Fi, Tailscale VPN, and USB ADB reverse tethering (`adb reverse tcp:8080 tcp:8080`).

### Linux Desktop & Virtual Output
* **Reference OS:** CachyOS / Arch Linux x86_64, running Wayland compositor.
* **Native Preview:** Embedded directly into the Tauri GTK window surface using GStreamer `waylandsink` and `GstVideoOverlay` (`gst_preview.rs`). No raw video data passes through React or Tauri IPC.
* **Virtual Camera (`v4l2loopback`):**
  * Spawns a supervised `gst-launch` pipeline (`virtual_output.rs`) targeting user-provisioned `/dev/video*` nodes.
  * Auto-detects virtual camera nodes (e.g. `/dev/video11`).
  * If the device node is missing or lacks permissions, returns clear remediation instructions. Never silently installs kernel modules or elevates host permissions.
* **Packaging:** Self-contained portable tarball generated via `./build-installer.sh --desktop-only`.

### Windows Delivery (G8)
* Candidate architecture: Windows Media Foundation (MF) virtual camera driver / custom media source.
* Implemented behind platform-independent core abstractions in `desktop/src-tauri/src/platform/`. Linux-only dependencies (`gstreamer`, `v4l2`) must remain strictly behind target gates.
