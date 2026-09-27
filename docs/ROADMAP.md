---
type: Execution Roadmap
status: Active (v0.3.0 Release)
version: 0.3.0
last_updated: 2026-09-27
owner: NirussVn0
authority: docs/ROADMAP.md
---

# Active roadmap

## Milestone Dependency

```text
G0 risks/decisions → G1 contract/bootstrap → G2 safe first pixel
 → G3 Linux output → G4 controls/recovery → G5 Linux release proof
 → G6 convenience
G5 → G7 production codec (H.264)
G0 Windows feasibility + G1 portability + G5 → G8 Windows delivery
```

---

## Gates Overview & Status

### G0 — Retire Expensive Uncertainty
* **Status:** **COMPLETE**
* **Deliverables:** Architectural decisions D03–D08 and D11 accepted. Verified Wayland `waylandsink` integration model, Camera2 JPEG baseline, QR trust model, and platform floor definitions (Android minSdk 26, CachyOS Wayland, GStreamer 1.28+).

### G1 — Executable Contract & Minimal Build Foundation
* **Status:** **COMPLETE**
* **Deliverables:** Discriminated `control-message.schema.json` v1 frozen; 43 valid, invalid, and semantic test fixtures; Python contract runner; cross-runtime parity tests passing in Kotlin (`ProtocolContractTest.kt`) and Rust (`protocol_contract.rs`); GitHub Actions CI operational.

### G2 — Safe First Pixel (Android $\rightarrow$ Linux Desktop)
* **Status:** **Core Complete (G2.1–G2.4); Physical Test Pending (G2.5)**
* **Deliverables:**
  * Android: `CameraEngine.kt`, `SessionManager.kt`, `Camera2Source.kt`, `MjpegHttpServer.kt`.
  * Desktop: `StreamClient.rs`, `GstPreview.rs` (`waylandsink`), bounded frame queues ($\le 2$ frames), watchdog lease cleanup.
* **Pending (G2.5):** 10-minute continuous run on physical phone to verify real thermal/memory trends.

### G3 — Linux Virtual Output (V4L2 Loopback)
* **Status:** **Core Complete (G3.1–G3.2); Browser Consumer Pending**
* **Deliverables:**
  * `virtual_camera.rs` sink, `virtual_output.rs` supervisor managing `gst-launch` pipeline.
  * Auto-detection of `/dev/video*` (e.g. `/dev/video11`), fake-sink tests, and headless smoke mode.
* **Pending:** Resolving Chrome Linux MMAP buffer negotiation on `/dev/video*`; verifying concurrent OBS Studio + browser capture.

### G4 — Live Controls & Recovery
* **Status:** **COMPLETE**
* **Deliverables:**
  * Typed `camera_set` commands and Android `setControl` interface.
  * Live camera control sliders (EV, ISO, shutter, focus) in desktop UI with real-time feedback.
  * `POST /control` authenticated wire endpoint on `MjpegHttpServer` with fail-closed validation.
  * Dynamic repeating capture request updates in `Camera2Source`.
  * 2-way network pairing handshake with mutual confirmation, ML Kit QR code scanner on mobile (`QrScanActivity.kt`), zero-dependency QR code SVG rendering on desktop, and ADB USB reverse tethering support.

### G5 — Linux Release Candidate (Physical Qualification)
* **Status:** **Awaiting Physical Phone Test**
* **Deliverables:**
  * Test script pre-written: `scripts/g5-measure.sh`.
  * Automated package installer: `build-installer.sh` (produces standalone Linux tarball and Android APK; v0.2.0 released).
* **Exit Criteria:** 30-minute sustained 1080p30 (or declared 720p30) run measuring latency $< 250\text{ ms}$, FPS stability, and memory/thermal profiles.

### G6 — Convenience After Reliability
* **Status:** **Core CLI Complete**
* **Deliverables:**
  * Typed schema and atomic profile persistence in `desktop/src-tauri/src/core/profiles.rs`.
  * Dedicated `camaproctl` CLI binary (`status`, `preview`, `v4l2`, `control`, `profile list/get/save/delete`).
* **Pending:** Desktop system tray icon and global hotkey bindings.

### G7 — Production Codec (H.264 via MediaCodec)
* **Status:** **Core Complete (Hardware Benchmark Pending)**
* **Deliverables:**
  * Android: `H264Encoder.kt` hardware encoder using `MediaCodec` (MIME `video/avc`, direct Camera2 Surface target, CBR, low-latency, Annex-B NAL unit emission).
  * Streaming: `GET /stream.h264` authenticated route on `MjpegHttpServer.kt`.
  * Desktop: `GstPreviewController` codec switching support (`CodecMode::H264` via `h264parse ! avdec_h264`).
* **Pending:** Physical phone benchmark measuring $< 120\text{ ms}$ latency at 1080p60 on physical device.

### G8 — Windows Delivery
* **Status:** **Planned**
* **Scope:** Windows Media Foundation virtual camera driver/sink, native DirectX preview adapter behind portable core.
