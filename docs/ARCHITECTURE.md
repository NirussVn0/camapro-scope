# Architecture

## Status and scope

This is the target architecture; no runtime is implemented. Decision statuses and unresolved choices live in [DECISIONS.md](DECISIONS.md). Linux is the first acceptance platform; Windows remains planned.

Goal: turn an Android phone into a controllable desktop camera on a local network, without a mandatory cloud service. The proposed first release (D04/D11) covers one phone and one controlling desktop, native preview, Linux virtual output, basic capability-driven controls, explicit pairing, cleanup and recovery. Multiple viewers, Internet streaming, cloud accounts, audio, macOS, recording, AI effects and plugins are out of scope. Profiles/CLI/hotkeys follow a reliable camera slice; H.264 follows measurement.

## Runtime ownership

```text
Android Compose UI ─> SessionManager (foreground-service lifecycle)
                         ├─ CameraEngine (only Camera2 owner)
                         ├─ control server + pairing/auth boundary
                         └─ media producer (bounded queue)
                                   │ authenticated direct LAN
Desktop control client ─> SessionController / CommandRegistry
                                   ├─ MediaEngine (one decode pipeline)
                                   │    ├─ native preview adapter
                                   │    └─ virtual-camera adapter
                                   └─ capabilities, state, credential store
React / tray / future CLI ─> same desktop command authority
```

| Owner | Responsibility | Must not own |
|---|---|---|
| Android SessionManager | controller lease, stream lifecycle, watchdog, foreground service | direct Camera2 access outside CameraEngine |
| CameraEngine | device/session/reader/surfaces, capability validation, atomic sensor changes | desktop effects or credentials |
| Desktop SessionController | connection generation, command serialization, reconnection, effective-state projection | OS output internals |
| MediaEngine | receive/decode once, bounded fan-out, metrics and teardown | React state or secret persistence |
| Platform adapters | native surface, virtual output, secure storage, optional shortcuts/IPC | camera business policy |
| React | controls, capability rendering, status/errors | raw video buffers, camera lifecycle authority |

Keep the Rust core independent of Tauri. Initially use modules rather than a large crate graph. A future CLI uses permission-restricted local IPC to the existing desktop authority; it must not instantiate another session or open another phone connection. Headless operation is a later explicit decision, not an implied daemon requirement.

## Dependency direction and planned paths

Create paths only when their slice needs them:

```text
android/app/src/main/java/app/camapro/scope/
  camera/       # CameraEngine, capabilities, controls
  session/      # service, lease, lifecycle
  network/      # discovery, pairing, control
  transport/    # bounded MJPEG; later H.264
desktop/        # desktop application
  src/          # React controls
  src-tauri/src/
    core/       # protocol, session, commands, media
    platform/   # linux, later windows; native preview and output
protocol/       # draft schema; G1 adds typed schemas and shared fixtures
skills/         # reusable project-local workflows
```

Core depends on small ports for network, clock, media, output and credentials; adapters implement ports and the composition root selects them. Do not make an interface for every class. No native handles or platform-specific paths in wire messages.

## Media contract

Bootstrap: Camera2 → JPEG acquisition/compression → bounded queue → authenticated MJPEG → GStreamer → native preview + optional virtual output. G0 must measure whether JPEG capture supports the target on an actual phone. If not, compare YUV→JPEG cost and a bounded early H.264 spike; amend the decision rather than pretending MJPEG always sustains 1080p30.

Production candidate: Camera2 → MediaCodec Surface → H.264 → RTP with an approved encrypted media profile → GStreamer decode. Plain RTP is not automatically production-safe. Transport replacement must not change camera controls or command semantics.

- Capture handoff and each decoded-frame consumer queue: at most two frames; drop oldest. Compressed inter-frame queues need codec-aware recovery/keyframe requests, not arbitrary dropping.
- Every other buffer (socket, multipart parser, jitter, decoder, appsrc/appsink) needs a measured byte/time bound in G0/G1. Do not equate a two-frame application queue with bounded total memory.
- Slow or closed preview must not block virtual output. Output failure becomes explicit degraded status; session.stop always stops phone capture and tears down local consumers.
- Share one decode stage. Preview uses a native surface/sink adapter; G0 must prove Wayland window lifetime, resize and teardown. No JPEG/base64/raw-frame fallback through React or Tauri IPC.
- Validate negotiated size, format, timestamps, rotation and mirroring. Mirror preview independently from output; no accidental double rotation or stretched aspect ratio.
- Desktop effects remain after decode; phone exposure/focus/lens controls remain before encode.

## Camera behavior

Capabilities are authoritative per selected camera and refresh after lens changes. Query FPS/resolution combinations, sensor orientation, logical/physical cameras, AE step and limits, ISO/shutter ranges, manual focus, OIS and processing modes. Camera IDs are opaque device identifiers; “main/wide/tele” are capability-derived aliases, not assumptions.

AE uses EV compensation in reported integer steps. Manual exposure sets AE off plus ISO and shutter together, constrained by FPS/frame duration. Unsupported controls remain disabled with a reason; requested state and confirmed applied state are distinct.

Android requires a user-visible camera foreground-service path appropriate to the selected SDK/device policy. Permission denial/revocation, app backgrounding, screen-off, notification stop, camera-in-use and service death must have explicit cleanup behavior. No automatic capture on reconnect without the resume policy below.

## Trust and session lifecycle

mDNS advertises only non-secret metadata; manual endpoint entry remains available. A discovered device is untrusted. Pairing, authenticated media access and watchdog cleanup precede any ordinary LAN camera release. Detailed trust and timeout semantics are in [PROTOCOL.md](PROTOCOL.md); cryptographic implementation remains gated by D03.

The following lifecycle is the proposed D04 policy, subject to G0 approval before G1 freeze. One controlling desktop lease per phone. Separate connection state from media/output status:

| State | Event | Next state / required action |
|---|---|---|
| Disconnected | discover or manual connect | Discovering or Connecting; no capture |
| Discovering | candidate selected / cancel | Connecting / Disconnected |
| Connecting | unpaired identity / trusted identity / timeout | Pairing / Ready after authenticated hello / Disconnected with error |
| Pairing | approval / reject, expiry, cancel | Ready after auth / Disconnected; delete ephemeral secrets |
| Ready | start / disconnect | Starting / Disconnected |
| Starting | first valid frame / failure or cancellation | Streaming / Stopping if remote start was sent; otherwise Ready if connected or Disconnected; tear down partial local resources |
| Streaming | stop / connection loss | Stopping / Reconnecting; stop local sinks and expire remote lease |
| Stopping | confirmed release / timeout | Ready if connected and release confirmed, otherwise Disconnected; on timeout tear down locally, abandon the lease and cease its heartbeats so phone capture expires |
| Reconnecting | authenticated same peer + old-generation capture released / retry budget exhausted / cancel | Ready (no automatic capture) / Disconnected / Disconnected |
| Any active state | permission loss, fatal media error, peer revoked | clean up owned resources, clear pending commands; Ready only if still authorized and usable, otherwise Disconnected |

Any exit from capture due to a local failure must request remote stop when reachable. Ready is allowed only after confirmed remote release; if confirmation times out, abandon the lease and stop renewing it, even if the control socket still appears healthy. Never keep capture alive by sending background heartbeats after failed cleanup.

Discovery is optional; saved peers can connect directly. Errors are typed state metadata, not a terminal `Error` state with no exit. Each async completion carries a connection generation; ignore stale completions after stop/reconnect. Start and stop are idempotent. Phone capture stops on lease expiry even when the desktop cannot deliver a stop message. Native output failure can leave a preview-only stream only with visible degraded status and an explicit stop action.

## Command authority

UI, tray and later CLI/hotkeys dispatch intent to one registry, then to the same session/controller logic. Camera start/stop, lens switches and profile operations serialize. Slider coalescing happens before assigning wire IDs, only for the same control/camera/generation. Desktop crop/zoom is a local effect, never an invented sensor capability.

Do not retry non-idempotent toggle after a timeout. Prefer an explicit desired start/stop command. Profile transactions and persistence are deferred; their atomicity/rollback must be specified before implementation.

## Priority

Feasibility + cleanup + trust → first authenticated pixel → virtual output → reliable controls and recovery → measured latency → CLI/profiles → H.264/Windows → polish/effects. Keep the existing stack unless a measured spike disproves it; no rewrite or extra agent runtime is justified by the current repository.
