# Camapro Scope

A planned local-first Android wireless camera platform for desktop: Android Camera2 → direct LAN → Rust/GStreamer → native preview and virtual camera.

**Status: design-only.** No Android project, desktop application, runtime tests, CI or released builds exist yet. The JSON schema is a draft envelope, not a validated complete protocol. Documentation describes the target, not delivered features.

## Product direction

- Android: Kotlin, Compose, Camera2; MediaCodec for a later H.264 path.
- Desktop: Rust + GStreamer; Tauri 2 / React / TypeScript for controls only.
- Linux-first delivery; Windows is a planned adapter, not claimed support.
- Control: versioned JSON over authenticated WebSocket; mDNS is discovery, never trust.
- MJPEG proves the first media path; performance must be measured on real devices.
- No mandatory cloud, account, or third-party relay for ordinary LAN use.

## Names

| Item | Identifier |
|---|---|
| Product / repository | Camapro Scope / `camapro-scope` |
| Android package / desktop app ID | `app.camapro.scope` |
| Planned CLI | `camaproctl` |
| DNS-SD service | `_camaproscope._tcp.local` |

## Read order

1. [Agent entrypoint](AGENTS.md)
2. [Architecture](docs/ARCHITECTURE.md) and [decision ledger](docs/DECISIONS.md)
3. [Protocol](docs/PROTOCOL.md) and [platform boundaries](docs/PLATFORMS.md)
4. [Active roadmap](docs/ROADMAP.md)
5. [Development and evidence](docs/DEVELOPMENT.md)
6. [Implementation handoff](prompts/IMPLEMENT.md)

See [baseline assessment](docs/ASSESSMENT.md) for why the plan changed and [documentation verification](docs/evidence/2026-09-13-replan.md) for actual checks and limits. Start with **G0 risk spikes**, not all milestones at once. There is currently no application run/build command; future commands are labeled as such in development docs.
