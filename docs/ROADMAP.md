# Active roadmap

This is the only active execution roadmap. All application gates below are **not started**. A docs refresh does not complete G0. Decision labels refer to [DECISIONS.md](DECISIONS.md); verification rules are in [DEVELOPMENT.md](DEVELOPMENT.md).

## Dependency order

```text
G0 risks/decisions → G1 contract/bootstrap → G2 safe first pixel
 → G3 Linux output → G4 controls/recovery → G5 Linux release proof
 → G6 convenience
G5 → G7 production codec
G0 Windows feasibility + G1 portability + G5 → G8 Windows delivery
```

Do not call Linux daily-usable before G5. Under proposed D11, G6–G8 may be reprioritized by the owner after G5 and do not block the first Linux camera release. If D11 is not approved, retain G6 as a prerequisite for the agreed MVP. Effects/AI remain a backlog, not active work.

## G0 — retire expensive uncertainty

**Outcome:** evidence-backed decisions before full application scaffolding. **Status:** not started.

Read architecture, D03/D04/D06/D07/D08 and media-platform skill. Work in bounded spikes; no production service or host-module installation.

| Task | Concrete output | Acceptance / stop condition |
|---|---|---|
| G0.1 | `docs/evidence/G0-environment.md` | record available tools + versions, reference phone/ADB authorization, OS/compositor, kernel/output prerequisites; missing hardware is BLOCKED, not assumed |
| G0.2 | `docs/evidence/G0-camera.md`, minimal Android spike only if SDK/device available | measure Camera2 JPEG modes at 720p30 then 1080p30, frame gaps, CPU, thermal/memory; compare YUV→JPEG only if needed; record negotiated vs delivered FPS |
| G0.3 | `docs/evidence/G0-preview.md`, minimal native sink spike | synthetic GStreamer source visible in intended native surface; resize/close/reopen, hidden window, bounded queues on actual Wayland; external window does not close integrated-preview risk |
| G0.4 | `docs/evidence/G0-trust.md` and D03 update | approve QR identity binding, TLS/pinning libraries, secure store, revocation, expiry and local-network policy; no real-camera LAN release while unresolved |
| G0.5 | update D04/D07/D08/D11 | owner confirms resume policy, first-release scope, reference mode/phone, numeric memory/latency budgets and Android/Linux floors; capture Windows API feasibility constraints without making Windows delivery decisions a blocker for Linux |

**Exit:** measurements and applicable decisions explicitly accepted; independent review. If the preview or JPEG stack fails, present a bounded alternative and cost before changing D02. Do not create a complete Android/Desktop module forest to answer these questions.

## G1 — executable contract and minimal build foundation

**Outcome:** both sides agree before parallel implementation. Depends on G0. **Status:** code tasks complete — G1.1/G1.2/G1.3 (Android APK + desktop)/G1.4 (Rust + Gradle-run Kotlin parity)/G1.5 authored, all verified (evidence: [G1-contract-builds.md](evidence/G1-contract-builds.md)); gate exit awaits independent read-only review, then lead rerun.

One contract writer first; Android and desktop scaffolding may proceed independently only after schemas/fixtures are agreed.

1. **G1.1 Contract negatives.** Create `protocol/fixtures/valid/`, `protocol/fixtures/invalid/`, `protocol/fixtures/semantic/` and `protocol/tests/test_contract.py`. Start with one valid/invalid pair per message plus ID, auth, duplicate/generation and bounds cases from protocol docs. Prove the permissive existing schema incorrectly accepts negative envelope cases before replacing it. Temporal/capability fixtures belong to semantic validators, not JSON Schema alone.
2. **G1.2 Contract freeze.** Modify `protocol/control-message.schema.json` into discriminated message variants; add `protocol/requirements-test.txt` with pinned validator and `protocol/README.md` documenting `python -m unittest discover -s protocol/tests -v`. Define directions, payloads, units, request completion, errors, limits and cache policy. Test failures must be assertion/contract failures, not missing imports. All fixtures must have a declared expected result.
3. **G1.3 Build skeletons.** Create minimal `android/settings.gradle.kts`, `android/build.gradle.kts`, Gradle wrapper, `android/app/build.gradle.kts`, `android/app/src/main/AndroidManifest.xml`; and `desktop/package.json`, `desktop/pnpm-lock.yaml`, `desktop/src/main.tsx`, `desktop/src-tauri/Cargo.toml`, Cargo lockfile, Tauri config/build entrypoint. Pin compatible tool versions from G0; add only required entrypoints/config, not placeholder future subsystems.
4. **G1.4 Consumer parity.** Add `android/app/src/test/java/app/camapro/scope/protocol/ProtocolContractTest.kt` and `desktop/src-tauri/tests/protocol_contract.rs`. Run the same valid/invalid/semantic corpus in both, testing large numeric values and exact unit interpretation. Add only minimal protocol source needed to make tests pass.
5. **G1.5 Deterministic gates.** Create `.github/workflows/ci.yml` after local commands exist: protocol fixtures, Android lint/unit/build, Rust format/clippy/test, TS typecheck/test/build, desktop native build and Windows portable-core compile where feasible. Lockfile-based installs; no privilege or camera hardware in ordinary CI. Missing required tools/tests must fail, not skip to green.

**Exit:** commands produce actual artifacts; fixtures pass on both runtimes; no application/hardware claims from scaffolding. Review contract before implementations branch apart.

## G2 — safe first pixel, one real phone to Linux

**Outcome:** explicitly start/stop an authenticated MJPEG stream with native desktop preview. Depends on G1 and approved D03/D04. **Status:** not started.

| Task | Planned paths | Red → green acceptance |
|---|---|---|
| G2.1 Android ownership | `android/app/src/main/java/app/camapro/scope/camera/CameraEngine.kt`, `session/SessionManager.kt`; matching tests under `android/app/src/test/java/app/camapro/scope/` | fake camera proves start/stop idempotency, partial-start failure, denied permissions and deterministic close; device confirms real Camera2 release |
| G2.2 Trust + lease | Android `network/`, desktop `src-tauri/src/core/control/` and `session/`; matching unit/integration tests | reject wrong identity, expired/replayed QR and unauthenticated requests; fake monotonic clock proves expiry stops producer; real disconnect confirms cleanup |
| G2.3 Bounded MJPEG | Android `transport/MjpegTransport.kt`, desktop `src-tauri/src/core/media/`; parser/queue tests | malformed/truncated/oversized parts bounded; deliberately slow consumer drops stale frames rather than growing queue |
| G2.4 Composition | desktop `src-tauri/src/platform/linux/preview.rs`, `src-tauri/src/core/commands/`, `src-tauri/src/lib.rs`, `src/` controls | minimal shared command dispatch for start/stop from the first UI; single decode and native preview; repeat close/reopen with no stale generation or JS frame bridge |
| G2.5 Real run | `docs/evidence/G2-first-pixel.md` | declared mode, actual phone/network, 10-minute run, FPS/frame age/memory trend, stop and desktop-death release; first-pixel target 720p30, not blanket 1080p30 certification |

**Exit:** actual camera frame in integrated Linux desktop, authenticated stream, lease cleanup verified and limitations documented. Synthetic-only test is useful but cannot close G2.

## G3 — Linux virtual output

**Outcome:** same decoded stream reaches OBS and browser camera capture. **Status:** not started.

- Create `desktop/src-tauri/src/platform/linux/virtual_camera.rs`; test output port independently with a fake sink before attaching user-provisioned v4l2loopback.
- Negotiate size/FPS/pixel format and color/rotation/aspect semantics; test missing/busy device and permissions without auto-elevation.
- Prove slow/closed preview does not stall output; output errors are visible, stopping never leaves sender capture running.
- Record `docs/evidence/G3-linux-output.md`: exact device node/format, OBS and browser versions, visible actual frames, restart and two-consumer behavior. Listing a camera name alone is not sufficient.

**Exit:** both consumers display current real-camera frames and teardown/restart succeeds. No kernel modifications without separate user approval.

## G4 — controls and recovery

**Outcome:** trustworthy capability-driven live controls and recovery. **Status:** not started.

- Extend the G2 command dispatch in `desktop/src-tauri/src/core/commands/` with live camera controls; do not introduce a second UI command path. Add Android camera control validation.
- Tests first: EV rational steps, opaque lens IDs, focus/manual support, AE-off ISO+shutter transaction, unsupported values, camera capability refresh, requested vs applied state.
- Add discovery/manual endpoint path, persisted trusted peer reference, secure credential storage and revoke action; first pairing/lease already exists in G2, not postponed here.
- Deterministic state tests cover every architecture transition: start canceled by stop, delayed events, heartbeat expiry, retry budget, permission loss, failed sink, pairing cancellation and process death. Reconnect returns Ready; the user starts again.
- Record `docs/evidence/G4-controls-recovery.md` with real-device controls and Wi-Fi interruption/restore; fake-clock tests do not substitute for device cleanup.

**Exit:** implemented controls reflect actual device support and confirmed applied state; no stuck camera or stale reconnection.

## G5 — Linux release candidate

**Outcome:** measured, installable and honestly scoped Linux MVP. **Status:** not started.

- Record `docs/evidence/G5-linux-release.md` using the evidence template in development docs.
- 30-minute declared-reference 1080p30 target run; measure negotiated/delivered FPS, drops, end-to-end latency, process RSS trend, thermal state and battery drain. Freeze pass thresholds before running, not after seeing results.
- If 1080p30 is not sustainable, report failure and request reduced support mode or reprioritized H.264; do not silently lower acceptance. `<250 ms` remains an unverified goal until measured with a stated method.
- Test clean-machine package install/start/pair/capture/stop/restart/uninstall; secrets redacted, credentials secure, required runtime dependencies documented.
- Independent read-only review plus lead rerun; every mandatory hardware row PASS or release BLOCKED.

**Exit:** declared Linux/phone combination is verified. Other distributions/devices remain unverified, not “supported by architecture.”

## G6 — convenience after reliability

Implement profiles/settings with atomic persistence and failure recovery in `desktop/src-tauri/src/core/profiles/`; define profile-apply validation/rollback first. Approve D09 and a versioned local IPC contract before adding `camaproctl`, tray and shortcut adapters. Test that CLI/UI affect the same session, that IPC rejects other users, and that a CLI command never starts a second controller. Establish exact CLI grammar and exposure units; do not copy example commands from old docs as a frozen API. Evidence: `docs/evidence/G6-convenience.md`. **Status:** not started.

## G7 — production codec

Resolve D10, then add Camera2→MediaCodec→approved protected H.264 transport and desktop receive/decode behind existing media ports. Test loss/reorder, bounded jitter, keyframe recovery, reconnect, fallback negotiation and hardware/software decoder behavior. MJPEG fallback must be explicit and equally authenticated. Measure against G5 on the same device/network; 1080p60 / <120 ms is a stretch goal, not acceptance already achieved. Evidence: `docs/evidence/G7-h264.md`. **Status:** not started.

## G8 — Windows delivery

Resolve Windows minimum OS/API and package requirements from G0/D08. Implement native preview/output/secure-store/IPC adapters behind portable core. Verify real Windows installation, privacy/registration, frames in two consumers, stop/restart/uninstall. Same protocol fixtures and Android APK. Linux build does not close this gate. Evidence: `docs/evidence/G8-windows.md`. **Status:** not started.

## Task closure

For each coding task: behavioral failing test → observe intended failure → minimal implementation → focused pass → relevant full gates → evidence → independent major-gate review. Commits are separate coherent changes only when authorized. Record status and evidence link here only after verifying; never tick requirements because a document was written.
