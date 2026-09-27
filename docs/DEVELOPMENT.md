---
type: Development & Verification Guide
status: Active
version: 0.2.0
last_updated: 2026-09-19
owner: NirussVn0
authority: docs/DEVELOPMENT.md
---

# Development and verification

## 1. Quick Verification Commands

From repository root, run each command within its specified workspace:

| Scope | Command | Purpose |
|---|---|---|
| **All-in-One Packaging** | `./build-installer.sh --no-rebuild` | Builds portable Linux tarball + Android APK. |
| **Protocol Fixtures** | `python -m unittest discover -s protocol/tests -v` | Validates 43 schema and semantic test fixtures. |
| **Android Lint & Tests** | `cd android && ./gradlew lintDebug testDebugUnitTest` | Runs Android JVM tests (CameraEngine, SessionManager). |
| **Android Build** | `cd android && ./gradlew assembleDebug` | Produces debug APK in `android/app/build/outputs/apk/`. |
| **Desktop Rust Suite** | `cd desktop/src-tauri && cargo test --locked` | Runs all 42 core/contract & platform integration tests. |
| **Desktop Rust Lint** | `cd desktop/src-tauri && cargo fmt -- --check && cargo clippy` | Ensures strict Rust formatting and zero clippy warnings. |
| **Desktop Web UI** | `cd desktop && pnpm typecheck && pnpm test && pnpm build` | TypeScript typecheck, unit tests, and production Vite bundle. |
| **Desktop Native App** | `cd desktop && pnpm tauri build` | Full Linux desktop executable packaging. |

---

## 2. Implementation Loop (Verified Slice)

Every implementation task follows these rules:
1. **Scope Check:** Identify the single gate/task in [ROADMAP.md](ROADMAP.md) and check component invariants in [ARCHITECTURE.md](ARCHITECTURE.md).
2. **Red-Green Test Cycle:**
   * Write an observable failing behavioral test before changing production code.
   * Verify intended failure (not a syntax/import error).
   * Implement the minimum coherent code to achieve green.
3. **Run Full Subsystem Gates:** Run the relevant commands above. A frontend build does NOT validate Tauri native code; JVM tests do NOT validate real Camera2 hardware.
4. **Cleanliness:** No unreviewed host changes, kernel module forced installations, or unauthenticated open network ports.

---

## 3. Acceptance Matrix

| Layer | Automated Test Evidence | Integration / Hardware Proof |
|---|---|---|
| **Protocol** | Negative schema fixtures, Kotlin/Rust parity, unit boundings. | Cross-runtime exchange between Android & Desktop. |
| **Session** | State machine transition matrix with fake monotonic clock. | Wi-Fi drop, notification kill, permission revoke, process death. |
| **Trust** | Expired/reused QR secret rejection, wrong identity rejection. | Mutual QR pairing handshake over LAN, Tailscale, or USB ADB. |
| **Media** | Bounded frame queues (max 2 frames), corrupt part recovery. | Real Camera2 capture delivered to native Waylandsink preview. |
| **Platform** | Simulated sink tests, missing device error detection. | Frame capture in OBS Studio and web browsers. |
| **UI** | State-driven panels (Disconnected, Ready, Streaming). | No raw video frames passed through React/JS; zero-lag controls. |

---

## 4. Performance Measurement Standards (G5 Gate)

For release qualification (Gate G5):
* **Target:** 1080p30 (fallback 720p30) sustained over 30 minutes with $\le 16\text{ MiB}$ media queue.
* **Latency Method:** Use an external high-frame-rate recording of a common digital timer/reference, or clock-calibrated timestamp delta. Simple uncalibrated clock subtraction across devices is invalid.
* **Telemetry Required:** Delivered FPS distribution, drop counts, queue high-water mark, process RSS curve, Android battery drain, and thermal throttling status.
* **Automation:** Pre-written measurement script is available at `scripts/g5-measure.sh`.