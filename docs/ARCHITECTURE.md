---
type: Architecture Specification
status: Active (v0.2.0 Core Implemented)
version: 0.2.0
last_updated: 2026-09-19
owner: NirussVn0
authority: docs/ARCHITECTURE.md
---

# Architecture

## 1. Status and Scope

Camapro Scope turns an Android phone into a high-quality, controllable desktop webcam on a local network without cloud dependency. 

As of **v0.2.0 (2026-09-19)**, the core application runtime is implemented and tested:
* **Android Sender:** Camera2 ISP capture, Android Foreground Service, embedded MJPEG HTTP server, and CameraX/ML Kit QR code scanner.
* **Linux Desktop:** Tauri 2 (Rust core + React 18 UI), GStreamer Waylandsink native preview, V4L2 virtual camera sink (`/dev/video*`), and atomic profiles.
* **Out of scope for MVP:** Audio, cloud accounts, multi-viewer streaming, recording, macOS, and AI filters.

---

## 2. Runtime Ownership and Boundaries

```text
Android Phone (Sender)                    Linux Desktop (Receiver / Controller)
┌──────────────────────────────────────┐  ┌─────────────────────────────────────┐
│ MainActivity / QrScanActivity        │  │ React UI (Navbar, StageFrame, Panel)│
│               │                      │  │               │                     │
│               ▼                      │  │               ▼ (Tauri IPC)         │
│ SessionManager (Foreground Service)  │  │ SessionController / CommandRegistry │
│   ├─ CameraEngine (exclusive Camera2)│  │   ├─ StreamClient (bounded queue)   │
│   ├─ MjpegHttpServer (HTTP /stream)  │  │   ├─ GstPreview (Waylandsink)       │
│   └─ TrustStore (Keys & Pairing)     │  │   ├─ VirtualOutput (V4L2 Loopback)  │
└──────────────────┬───────────────────┘  │   └─ ProfileStore (atomic JSON)     │
                   │ Authenticated LAN    └─────────────────────────────────────┘
                   ▼ (Tailscale / Wi-Fi / USB ADB)
```

### Strict Ownership Invariants
| Owner | Responsibility | Forbidden Actions |
|---|---|---|
| **Android SessionManager** | Controller lease, watchdog, foreground service lifecycle. | Directly opening Camera2 outside `CameraEngine`. |
| **CameraEngine** | Camera2 device/surfaces, capability validation, atomic sensor state. | Managing desktop network tokens or credentials. |
| **Desktop SessionController**| Session generations, command serialization, auto-reconnect. | Direct OS output manipulation. |
| **StreamClient / Media** | Bounded frame parsing, single decode pipeline, metrics. | Retaining secrets or modifying UI state. |
| **Platform Adapters** | Native Wayland preview surface, V4L2 output spawner, keyrings. | Introducing camera business logic. |
| **React UI** | Controls display, status indicators, QR rendering. | **Handling raw video frames** (zero-copy native preview only). |

---

## 3. Media Pipeline Invariants

* **Bounded Queues:** In-flight media queues hold at most **2 frames**; the oldest frame is dropped immediately when consumers lag to prevent RAM growth. Total media memory is capped $\le 16\text{ MiB}$.
* **Single Decode Stage:** The MJPEG stream is decoded once on the desktop and split between native preview (`waylandsink`) and virtual camera (`v4l2sink`). Slow preview never blocks virtual camera output.
* **Zero JavaScript Frame Bridge:** Video frames are rendered directly into native GTK/Wayland surfaces via GStreamer `GstVideoOverlay`. React receives only FPS, status, and telemetry.
* **Independent Mirroring:** Preview mirroring is purely a display transform and never alters the output frame sent to virtual camera consumers.

---

## 4. Session State Machine (Accepted D04 Policy)

```text
  [Disconnected] ──(connect)──> [Connecting] ──(trusted)──> [Ready]
        ▲                             │                        │
        │                       (untrusted)                    ▼
        │                             ▼                  [Streaming]
        │                         [Pairing]                    │
        │                             │ (approved)             ▼
        └────────(cancel/fail)────────┴───────────────> [Stopping]
```

| State | Transition Event | Next State & Required Cleanup |
|---|---|---|
| **Disconnected** | User connects via LAN / Tailscale / USB | $\rightarrow$ `Connecting`; no camera capture active. |
| **Connecting** | Pinned peer verified / Unknown peer / Error | $\rightarrow$ `Ready` (if trusted) / `Pairing` (if unknown) / `Disconnected`. |
| **Pairing** | User confirms QR code match / Expiry | $\rightarrow$ `Ready` on mutual approval / `Disconnected` on cancel/timeout. |
| **Ready** | Start stream command / Disconnect | $\rightarrow$ `Streaming` / `Disconnected`. |
| **Streaming** | Stop command / Heartbeat loss / Wi-Fi drop | $\rightarrow$ `Stopping` (confirmed) / `Reconnecting` (transient drop). |
| **Stopping** | Confirmed release or timeout | $\rightarrow$ `Ready` (if connected) or `Disconnected`. Teardown sinks immediately. |
| **Reconnecting** | Same peer re-authenticates / Budget exhausted | $\rightarrow$ `Ready` (no silent capture restart) / `Disconnected`. |

### Heartbeat & Watchdog Rules
* Desktop sends `ping` every **2.0 seconds**.
* If phone receives no valid heartbeat for **6.0 seconds**, watchdog automatically terminates Camera2 capture.
* Reconnect creates a new monotonically incremented generation ID; stale generation events or frames are dropped immediately.

---

## 5. Current Codebase Paths

```text
android/app/src/main/java/app/camapro/scope/
  camera/       # Camera2Source, CameraEngine, CameraSource interface
  network/      # NetworkHelper (LAN/Tailscale/USB IP), QrEnrollment, TrustStore
  service/      # CameraStreamService (Foreground Service)
  session/      # ControllerLease, SessionManager, SessionState
  transport/    # MjpegHttpServer, MjpegTransport
  MainActivity.kt, CameraActivity.kt, QrScanActivity.kt, ScopeUi.kt
desktop/
  src/          # React 18 + Tailwind UI (StageFrame, Panel, hooks, QR)
  src-tauri/src/
    core/       # commands, control (pairing/server), media (stream_client, virtual_camera), profiles, session
    platform/   # platform adapters: linux (gst_preview, preview, virtual_output)
protocol/       # Frozen v1 schema (control-message.schema.json) + 43 fixtures
.agents/skills/ # Reusable project-local workflow skills
scripts/        # Benchmark & measure scripts (g5-measure.sh)
build-installer.sh # Automated portable Linux tarball + Android APK packager
```
