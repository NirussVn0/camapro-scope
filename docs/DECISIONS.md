---
type: Decision Ledger
status: Living Canon
version: 0.2.0
last_updated: 2026-09-19
owner: NirussVn0
authority: docs/DECISIONS.md
---

# Decision ledger

Statuses: **retained** = baseline direction; **proposed** = recommendation requiring evidence/owner approval before implementation; **open** = unresolved; **accepted** = owner-approved decision with approval date recorded.

| ID | Status | Decision / Summary |
|---|---|---|
| **D01** | retained | Local-first architecture: Android sender, Linux-first desktop; Windows planned; zero cloud dependencies. |
| **D02** | retained | Tech stack: Kotlin/Camera2 on mobile; Rust/GStreamer on desktop; Tauri 2 + React controls; MJPEG bootstrap, H.264 later. |
| **D03** | accepted | Authenticated encrypted control and bootstrap media; QR binds peer certificate fingerprint and expiring one-time secret. Uses rustls + pinning on desktop, Android KeyStore + TLS 1.3 on phone, and platform secure storage (keyring-rs / EncryptedSharedPreferences) without plaintext fallback. Accepted 2026-09-13. |
| **D04** | accepted | One phone + one controlling desktop lease. Reconnect returns Ready and requires explicit start; no silent camera reactivation. Accepted 2026-09-13. |
| **D05** | retained | Single Camera2 owner, one desktop command authority, bounded media queues ($\le 2$ frames) and no raw video through JS. |
| **D06** | accepted | Native preview technique on Wayland/Tauri/WebKit: GStreamer `waylandsink` using `GstVideoOverlay` targeting native GTK container surface with a 2-frame downstream leaky queue. Accepted 2026-09-13. |
| **D07** | accepted | Target 1080p30 (fallback 720p30) on declared reference device. Media queue memory budget $\le 16\text{ MiB}$; latency target $< 250\text{ ms}$. Accepted 2026-09-13. |
| **D08** | accepted | Platform floors: Android minSDK 26 / targetSDK 34; Linux reference is CachyOS x86_64, Wayland, GStreamer 1.28+; Windows 11 build 22000+ for MF virtual camera. Accepted 2026-09-13. |
| **D09** | proposed | Future CLI talks to desktop-owned local IPC. Do not add a background daemon now. Decide launch-when-not-running behavior before G6. |
| **D10** | open | H.264 encrypted transport profile, loss recovery, keyframe policy and redistribution/licensing of codecs/runtime. Resolve before G7 distribution. |
| **D11** | accepted | Narrow first Linux camera release to G5; defer profiles/CLI/hotkeys to G6 instead of making them prerequisites for first release. Owner approved 2026-09-13. |

---

## Decision Workflow

For any proposed architectural change, record: requirement, alternatives, evidence, chosen option, affected contracts, migration cost, and owner approval. Do not silently promote a proposal without owner consent. Historical assessments are context, not competing active roadmaps.
