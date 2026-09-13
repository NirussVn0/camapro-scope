# G2 Safe first pixel — evidence

Gate/task and verdict:
- G2.1 (Android ownership): PASS
- G2.2 (Trust + lease): PASS
- G2.3 (Bounded MJPEG): PASS
- G2.4 (Composition): PASS
- G2.5 (Real hardware run): BLOCKED (no physical Android phone currently attached via ADB)

Date, inspected commit and dirty/untracked scope:
- Date: 2026-09-14
- Inspected commits: `d4f56ec`, `5a83c60`, `dd90c5a`, `33cc72a`
- Untracked scope: pristine; all unit/integration tests committed and green.

Changed paths / artifact checksums:
- `android/app/src/main/java/app/camapro/scope/camera/{CameraSource.kt, CameraEngine.kt}`
- `android/app/src/main/java/app/camapro/scope/session/{SessionState.kt, ControllerLease.kt, SessionManager.kt}`
- `android/app/src/main/java/app/camapro/scope/network/{QrEnrollment.kt, TrustStore.kt}`
- `android/app/src/main/java/app/camapro/scope/transport/MjpegTransport.kt`
- `android/app/src/test/java/app/camapro/scope/camera/CameraEngineTest.kt` (6 tests)
- `android/app/src/test/java/app/camapro/scope/session/SessionManagerTest.kt` (8 tests)
- `android/app/src/test/java/app/camapro/scope/network/{QrEnrollmentTest.kt, TrustStoreTest.kt}` (7 tests)
- `android/app/src/test/java/app/camapro/scope/transport/MjpegTransportTest.kt` (4 tests)
- `desktop/src/App.tsx` (React UI controls)
- `desktop/src-tauri/src/core/{commands, control, media, session}/**`
- `desktop/src-tauri/src/platform/linux/preview.rs`
- `desktop/src-tauri/tests/{trust_and_lease.rs, bounded_mjpeg.rs, composition.rs}` (10 new tests)

Environment:
- OS: CachyOS x86_64, Linux kernel 7.2.2-1-cachyos, Wayland (niri)
- Rust: 1.97.1 (cargo/rustc)
- Java/Android: Temurin JDK 21.0.12.1+1, Android SDK 34 (`~/Android/Sdk`)
- Python: 3.14.7 venv (`protocol/.venv`)
- Node / pnpm: Node 22.23.1, pnpm 11.3.0
- Attached devices: None (`adb devices` reports empty)

Commands: exact cwd + command + exit status:
1. `protocol/` | `protocol/.venv/bin/python -m unittest discover -s protocol/tests -v` | exit 0 (9/9 pass)
2. `android/` | `JAVA_HOME=~/Android/jdk/jdk-21.0.12.1+1 ANDROID_HOME=~/Android/Sdk ./gradlew lintDebug testDebugUnitTest assembleDebug` | exit 0 (30 unit tests pass, 0 lint errors, debug APK built)
3. `desktop/` | `pnpm typecheck && pnpm build` | exit 0 (TypeScript compile clean, Vite bundle built)
4. `desktop/src-tauri/` | `cargo fmt --all -- --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked` | exit 0 (15 tests pass, 0 clippy warnings)

Scenario & Acceptance Proof:
- G2.1: Fake camera proves start/stop idempotency, partial start failure, denied permissions, and deterministic closure.
- G2.2: Enrollment manager and trust store reject expired and replayed QR secrets (fail-closed). Rust session controller monotonic watchdog proves heartbeat expiration stops producer. Reconnect bumps generation and enforces Ready state without silent reactivation (D04).
- G2.3: Bounded frame queue proves max capacity of 2 frames with drop-oldest behavior under slow consumers, and bounded MJPEG parser safely handles partial chunks and rejects oversized payloads.
- G2.4: Command dispatcher coordinates session lifecycle and native preview sink with generation isolation. React UI receives state and commands without passing raw video buffers across IPC (D05, D06).

Limitations and unresolved support rows:
- G2.5 hardware verification requires an actual Android phone attached via ADB/LAN for a sustained 10-minute live capture test. Marked BLOCKED until hardware is attached.
