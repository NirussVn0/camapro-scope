# Docs changelog

## v0.3.0 — 2026-09-27

- **Live Camera Controls (G4)**: Real-time Camera2 dynamic capture request updates on Android (EV compensation, manual ISO sensitivity, manual shutter time, manual focus diopters) synchronized with Tauri live sliders over authenticated `POST /control`.
- **Standalone CLI `camaproctl` (G6)**: Dedicated headless CLI tool for checking phone status, streaming preview via Wayland GStreamer, virtual camera testing, camera control commands, and atomic profile CRUD. Packaged into Linux desktop distribution.
- **H.264 Hardware Streaming (G7)**: Android `MediaCodec` low-latency hardware video encoder (`H264Encoder.kt`) streaming Annex-B NAL units via `GET /stream.h264`, with desktop GStreamer H.264 decode pipeline (`h264parse ! avdec_h264`).
- **Local Skills & Docs Governance**: Relocated operational skills to `.agents/skills/` and established authoritative documentation governance index in `docs/README.md`.
- **Packaging & JDK Auto-detection**: Enhanced `build-installer.sh` to auto-detect portable JDK 21 LTS and include `camaproctl` in user-space installation.

## v0.2.0 — 2026-09-17

- **Desktop UI**: Redesigned UI faithful to brief (navbar, sidebars, StageFrame, brand icon, color palette per DESIGN.md).
- **QR Pairing**: Desktop QR code visual rendering (inline SVG, zero dependencies) and payload generation.
- **Android QR Scanner**: CameraX + ML Kit QR code scanner for desktop pairing.
- **Android Mobile UI**: T4 mobile UI (black disconnected state, floating bottom sheet, status indicator dots).
- **Controls & Profiles**: G4 camera control protocol prep (`camera_set`, `CameraEngine.setControl`) and G6 profiles infrastructure (typed schema + atomic store).
- **Release Automation**: Enhanced `build-installer.sh` to automatically build both Linux desktop portable installer and Android APK, generate SHA256 checksums, and support GitHub release publication via `gh release create`.

## Design replan — 2026-09-13

- Made design-only/runtime-unverified status explicit; preserved the draft JSON envelope unchanged.
- Replaced P0–P10 with one dependency-ordered G0–G8 roadmap, risk spikes, precise task paths and hardware acceptance gates.
- Defined ownership, native preview risk, bounded-pipeline requirements, complete cleanup/recovery paths and one future CLI command authority.
- Proposed early encrypted/authenticated control + media and explicit reconnect consent; recorded approval/evidence gates rather than claiming implementation.
- Added decision ledger, baseline assessment, AGENTS.md and four repository-local workflow skills.
- Reduced implementation prompt duplication; separated current document checks from future application commands.
- No application implementation, system changes, commit, push or release included.

## v0.1 (prior baseline)

- Standardized product naming as **Camapro Scope**.
- Changed product definition from Linux-only webcam to cross-platform desktop camera platform.
