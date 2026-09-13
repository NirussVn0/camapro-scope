# Decision ledger

Statuses: **retained** = baseline direction; **proposed** = recommendation requiring evidence/owner approval before implementation; **open** = unresolved; **accepted** = owner-approved decision with evidence and approval date recorded. No new decision is accepted by this replan. This ledger does not claim implementation.

| ID | Status | Decision / gate |
|---|---|---|
| D01 | retained | Local-first, Android sender, Linux-first desktop; Windows planned; no cloud dependency. |
| D02 | retained | Kotlin/Camera2; Rust/GStreamer; Tauri 2 + React controls only; MJPEG bootstrap, H.264 later. G0 can reopen feasibility. |
| D03 | accepted | Authenticated encrypted control and bootstrap media; QR binds peer key/certificate identity (SHA-256 fingerprint) and expiring one-time secret. Uses rustls + pinning on desktop, Android KeyStore + TLS 1.3 on phone, and platform secure storage (keyring-rs / EncryptedSharedPreferences) without plaintext fallback. Evaluated in `docs/evidence/G0-trust.md`; accepted 2026-09-13. |
| D04 | accepted | One phone + one controlling desktop lease. Reconnect returns Ready and requires explicit start; no silent camera reactivation. Confirmed for G1 freeze; accepted 2026-09-13. |
| D05 | retained | Single Camera2 owner, one desktop command authority, bounded media queues and no raw video through JS. |
| D06 | accepted | Native preview technique on Wayland/Tauri/WebKit: GStreamer `waylandsink` using `GstVideoOverlay` targeting native GTK container surface with a 2-frame downstream leaky queue. Evaluated in `docs/evidence/G0-preview.md`; accepted 2026-09-13. |
| D07 | accepted | Benchmark 720p30 first; 1080p30 remains release target on a declared reference device. Memory budget <= 16 MiB total in-flight media queue; < 250 ms latency target; accepted 2026-09-13. |
| D08 | accepted | Floors defined: Android minSDK 26 / targetSDK 34; Linux reference is CachyOS x86_64, Wayland (`niri`), GStreamer 1.28+; Windows 11 build 22000+ for MF virtual camera. Recorded in `docs/evidence/G0-environment.md`; accepted 2026-09-13. |
| D09 | proposed | Future CLI talks to desktop-owned local IPC. Do not add a daemon/headless mode now. Decide launch-when-not-running behavior before G6. |
| D10 | open | H.264 encrypted transport profile, loss recovery, keyframe policy and redistribution/licensing of codecs/runtime. Resolve before G7 distribution. |
| D11 | accepted | Narrow first Linux camera release to G5; defer profiles/CLI/hotkeys to G6 instead of making them prerequisites for first release. Owner approved 2026-09-13. |

## Decision workflow

For a changed decision record: requirement, alternatives, evidence, chosen option, status, affected contracts, migration cost and owner approval. Do not silently promote a proposal because a worker needs a default. G0 may run bounded local/synthetic feasibility experiments while approvals remain open; G1 contract freeze and real-LAN release stop at unresolved applicable decisions.

Skills and prompts route work; they cannot introduce product decisions or overrule this ledger. Historical assessments and `.hermes/plans/` are context, not competing active roadmaps.
