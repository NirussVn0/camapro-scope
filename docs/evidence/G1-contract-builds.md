# G1 Executable contract and minimal build foundation — evidence

Gate/task and verdict:
- G1.1 PASS (red baseline recorded), G1.2 PASS, G1.3-desktop PASS, G1.4-rust PASS, G1.5 authored (first real run occurs on GitHub CI)
- G1.3-android BLOCKED (no Android SDK; wrapper jar cannot be generated without a Gradle binary)
- G1.4-kotlin PARTIAL: the same 5-test corpus passes under kotlinc 2.1.0 + JUnit4 on JDK 21, but not yet under `./gradlew testDebugUnitTest`
Date, inspected commit and dirty/untracked scope: 2026-09-13, base `master` (all G1 outputs are new untracked files; no prior tracked application source existed)
Changed paths / artifact checksums:
- `protocol/control-message.schema.json` (rewritten to discriminated variants), `protocol/fixtures/**` (39 fixtures + manifest), `protocol/tests/test_contract.py`, `protocol/requirements-test.txt`, `protocol/README.md`
- `android/{settings.gradle.kts,build.gradle.kts,gradle/wrapper/gradle-wrapper.properties,app/build.gradle.kts,app/src/main/AndroidManifest.xml,app/src/main/java/app/camapro/scope/MainActivity.kt,app/src/test/java/app/camapro/scope/protocol/*}`
- `desktop/{package.json,pnpm-lock.yaml,pnpm-workspace.yaml,tsconfig.json,vite.config.ts,index.html,src/**}`, `desktop/src-tauri/{Cargo.toml,Cargo.lock,build.rs,tauri.conf.json,icons/*.png,src/lib.rs,src/main.rs,tests/protocol_contract.rs}`
- `.github/workflows/ci.yml`
Environment: CachyOS x86_64, Linux 7.2.2-1-cachyos, niri/Wayland; Python 3.14.7 venv (`protocol/.venv`) with jsonschema 4.26.0; rustc/cargo 1.97.1; Node 22.23.1, pnpm 11.3.0; OpenJDK 21 (`java-21-openjdk`) + standalone kotlinc 2.1.0 (downloaded to `/tmp/opencode/kotlin-sdk`, outside repo); no Android SDK (`ANDROID_HOME` unset), no devices attached.
Decisions and prerequisites: D03/D04/D06/D07/D08 accepted (DECISIONS.md); G0-environment recorded. minSdk 26 / targetSDK 34 / compileSdk 34 per D08. Kotlin 2.1.0 requires JDK ≤25 for its CLI; default JDK 26 fails (`IllegalArgumentException: 26.0.2` in bundled IntelliJ core) — JDK 21 used, matching AGP expectations.

Commands: exact cwd + command + exit status + log/artifact path:
- repo root | `protocol/.venv/bin/python -m unittest discover -s protocol/tests` (BEFORE schema freeze) | exit 1 — 11 failures, all assertion failures naming the negative cases the permissive draft accepted (unknown_top_level_field, event_with_request_id, request_requires_id, id bounds incl. probe 9007199254740992, empty changes, unknown control key, non-integer EV steps, error code/response-ok invariants, oversized vector) — intended G1.1 red
- repo root | same command (AFTER freeze) | exit 0 — "Ran 9 tests ... OK"
- `desktop` | `pnpm install --frozen-lockfile --ignore-scripts` | exit 0; `pnpm build` | exit 0 → `desktop/dist/assets/index-RKYR0vTB.js` (222.83 kB) + index.html; `pnpm typecheck` runs as part of build (tsc) exit 0
- `desktop/src-tauri` | `cargo build` | exit 0 → `target/debug/camapro-scope`; `cargo fmt --all -- --check` exit 0; `cargo clippy --all-targets --locked -- -D warnings` exit 0; `cargo test --locked` exit 0 → `test result: ok. 5 passed` (protocol_contract)
- `android/app` | `/usr/lib/jvm/java-21-openjdk/bin/java -cp <kotlinc classes + junit4 + org.json 20240303 + kotlin-stdlib> org.junit.runner.JUnitCore app.camapro.scope.protocol.ProtocolContractTest` | exit 0 → `OK (5 tests)`
- repo root | `python -m json.tool protocol/control-message.schema.json > /dev/null` | exit 0
Scenario: cross-runtime parity on the shared corpus: Python reference suite, Rust consumer (jsonschema crate 0.45 validating against the frozen schema file itself), Kotlin consumer (hand-written envelope validator + semantic rules). Large-value check: max safe integer id validates in both consumers; +1 rejected. Unit checks: EV steps integer==2, shutterNanos max==33333333 asserted identically in Rust/Kotlin.
Measurements: method, thresholds fixed before test, actual values: fixture counts fixed via manifest before implementation — 11 valid, 19 invalid, 9 semantic = 39 declared expectations, each with expected outcome; consumer floors (>=10/>=15/>=8) encoded as assertions so silent fixture deletion fails.
Fault cases and recovery/resource cleanup: red-phase failure list captured above; `response.session.stop.json` (a response-shaped stop ack) was removed during freeze because the variant model forbids `payload`+`result` mixing — response carries the ack instead; no stale daemon left running (cargo/pnpm/kotlinc all foreground; ADB untouched this session).
Evidence type: build + contract (simulated semantics; no camera hardware involved).
Independent reviewer findings and lead verification: pending independent read-only review of G1 (roadmap requires it at gate exit); lead re-ran all commands after final edits (results above).
Limitations and unresolved support rows:
- G1.3-android: `./gradlew assembleDebug` NOT run — no Android SDK installed and installing one needs explicit user authorization; `gradle-wrapper.jar` intentionally absent (must be generated by Gradle 8.11.1 itself, sha256 pinned in gradle-wrapper.properties from services.gradle.org checksum f397b287...151c6).
- G1.4-kotlin parity proven under raw JVM only; `lintDebug testDebugUnitTest` require the SDK.
- `pnpm tauri build` packaging not executed (requires network-heavy bundler deps; frontend bundle + cargo native binary proven separately). Placeholder solid-color icons committed for generate_context!.
- Desktop JS lint/test scripts intentionally absent until the first UI slice (G2.4) adds them; CI job runs typecheck+build instead.
- G1 gate NOT closed: closure requires Android APK build + gradle-run parity once an SDK is authorized.
