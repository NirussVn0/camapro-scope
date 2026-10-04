# Direct LAN TLS backend handoff — 2026-10-01

Baseline: clean `62bb814`; backend-only corrective D03/D04 slice, followed by
user-provided independent-review corrections. This backend owner performed no
commits, system installations, frontend edits, or host credential writes.
Concurrent frontend changes are preserved and remain with their owner.

## Delivered path and interfaces

- Desktop QR contains a real rcgen certificate SHA-256 DER fingerprint, a CSPRNG
  256-bit secret, HTTPS callback endpoint, and five-minute expiry. Runtime expiry
  is monotonic; consumption is mutex-protected and single-use. A failed endpoint
  probe also burns the secret: generate a fresh QR to retry.
- Phone scans QR, verifies its shape/expiry, and pins the HTTPS callback. A QR
  scanned while idle is pending until the user presses phone **Start**. Pairing
  never calls capture start. The TLS listener exists before enrollment begins.
- Phone listener requires TLS 1.3, the enrolled desktop client certificate, and
  `X-Camapro-Token` for status, control, MJPEG and H.264. URL tokens are rejected.
  Phone key is native AndroidKeyStore; desktop pin uses EncryptedSharedPreferences.
  Trust replacement closes sockets and replaces the TLS session cache. Stopped
  endpoints cannot be reopened by delayed enrollment.
- Review correction: enrollment uses a provisional TLS context pinned **only** to
  the proposed desktop certificate and permits only authenticated `/status`.
  Previous persisted trust remains untouched until phone commit. Phone persistence
  requires initial callback HTTP 200, the current ticket/endpoint under the same
  lock used by phone Stop, and an unexpired QR. Failure/expiry **before commit**
  restores the previous context; Stop invalidates the ticket and closes the
  endpoint. An older attempt cannot commit or roll back a newer attempt.
- Phone's random 256-bit token now persists alongside trust in encrypted
  preferences. Recreation reads the same token; corrupt data or failed secure
  writes fail closed. Existing installations without a stored token require one
  new QR enrollment to establish the persistent token.
- Final ordering correction: initial `POST /pair` burns QR enrollment use, probes
  the provisional phone, and returns 200 **without storing a peer or emitting**.
  At most one in-memory completion ticket retains the original secret, bound to
  socket peer IP and the exact phone pin/port/token/name. It expires after at most
  30 seconds and never outlives the original QR. A new QR invalidates it.
- Android commits the approved pin, replaces the TLS listener with the committed
  context, and only then sends private pinned-HTTPS `POST /pair/confirm` with the
  same body. Rust validates the completion binding and performs a **fresh mutual
  TLS** probe. Authenticated `/status` must have exactly one
  `X-Camapro-Ready: true` header; provisional, missing or duplicate markers fail
  closed. This marker means committed **listener** readiness, not capture-free
  D04 Ready. Only this confirmed probe can persist the peer and emit
  `phone-paired`; no listener replacement occurs after confirmation success.
- Matching completion attempts are single-use: success burns the capability,
  and matching probe/storage failure also burns it. Wrong bindings do not burn
  another phone's pending completion. Replays, expired tickets and query-string
  capability requests are rejected without success. No secrets are logged or
  placed in URLs.
- Post-commit confirmation failure/expiry or a lost final reply is ambiguous:
  Android retains the already-approved pin, does **not** show pairing success,
  and asks for a fresh QR. It never rolls back after commit, which could otherwise
  revoke trust that Rust already confirmed. Stop/superseding tickets prevent
  late confirmation success locally. Before-commit failure still restores old
  trust; after-commit failure is not represented as a completed desktop pairing.
- Desktop identity and phone
  credentials persist exclusively through keyring-rs Secret Service. Storage
  failure fails closed. Rust centrally resolves endpoint pins for status, preview,
  control and CLI status; JavaScript supplies no certificate fingerprint.
- Existing Tauri signatures and `phone-paired {host, port, token, name}` are
  unchanged. `host` is the callback socket's phone address. The private Android
  callback JSON now requires `phone_fingerprint_sha256`; `phone_ip` is removed.
  QR `endpoint_hint` is HTTPS; old HTTP/fake-fingerprint QR payloads are rejected.
- Preview source is Rust TLS -> bounded two-frame drop-oldest channel -> FIFO ->
  GStreamer. `gst_preview` performs no network fetch. `virtual_output` currently
  uses `videotestsrc`, not phone media; there is no subprocess HTTP bypass.
  Phone permits one media writer while keeping control/status available through
  a bounded two-worker/eight-waiting-socket executor.
- Review correction: server sockets have an absolute five-second accept -> TLS
  handshake -> complete request budget. Independent deadline owners close sockets
  even when individual reads keep succeeding. Android queue wait is included;
  rejected/finished requests cancel their timers. Authenticated stream readiness
  cancels the media request timer. Rust outbound handshake, probe/control and
  media-response head reads also have absolute socket-close budgets; preview
  cancels the response-head guard after validated readiness.
- Reader or pump termination is reaped by `preview_status` or the next start,
  releasing the session and reporting `active: false`. Stop shuts down a cloned
  socket before joining, so partial TLS records cannot trap the reader. FIFO
  reopen is now nonblocking, and the pump detects child exit even when frames
  continue arriving. `active` describes only local desktop preview.

## Initial-slice verification (historical, before independent-review corrections)

Commands are relative to the stated working directory. Exit status is explicit.

| Directory | Command | Exit / result |
|---|---|---|
| repository root, before implementation | `python -m unittest discover -s tests -p test_lan_security.py -v` | **1**: three intended failures (predictable QR identity, URL-token auth, pairing capture auto-start) |
| `desktop/src-tauri`, baseline | `cargo test --locked` | **0** |
| `desktop/src-tauri`, dependency resolution | `cargo check` | **101**, fixed a new Result/Option compile error; generated dependency lock |
| `desktop/src-tauri`, final | `cargo test --locked` | **0**: 56 passed, two hardware smoke tests ignored |
| `desktop/src-tauri`, final | `cargo fmt -- --check` | **0** |
| `desktop/src-tauri`, final | `cargo clippy --locked --all-targets -- -D warnings` | **0** |
| `android`, default Java 26 | `./gradlew testDebugUnitTest assembleDebug` | **1**, Gradle/Kotlin toolchain rejected Java 26.0.2 |
| `android`, system Java 21 runtime | `JAVA_HOME=/usr/lib/jvm/java-21-openjdk ./gradlew testDebugUnitTest assembleDebug lintDebug` | **1**, runtime lacks JAVA_COMPILER |
| `android`, final bundled JDK | `JAVA_HOME=/home/nirussvn0/Android/jdk/jdk-21.0.12.1+1 ./gradlew testDebugUnitTest assembleDebug lintDebug` | **0**: 63 JVM tests passed, APK built, lint passed |
| repository root, final | `python -m unittest discover -s tests -p test_lan_security.py -v` | **0**: three passed |
| repository root | `git diff --check` | **0** |

Intermediate Android compilation errors from the new concurrent socket owner
were corrected before the final successful gates. No dependency/toolchain
blocker remains when using the bundled JDK. Android security-crypto emits
deprecation warnings; the implementation follows the accepted D03 storage choice.

Native tests exercise real Rust TLS sockets: exact pin acceptance, wrong-pin
rejection before credentials, TLS 1.2/plaintext rejection, mutual TLS valid/wrong/
missing client identity, unenrolled endpoint/token rejection, and bounded media.
Existing control and preview/fakesink integration tests now run over TLS.
Android JVM tests cover header-only auth on both codecs and status, concurrent
status/control during media, fail-closed plaintext LAN bind, explicit-start
source regression, stopped-endpoint restart rejection, random tokens and digest
parity. They do not execute AndroidKeyStore or real phone TLS.

## First review-correction verification (historical, before final ordering fix)

| Directory | Exact command | Exit / result |
|---|---|---|
| `desktop/src-tauri`, RED before EOF fix | `cargo test --locked --test preview_e2e server_end_marks_preview_inactive_and_allows_restart` | **101**, intended failure: EOF left preview active |
| `android`, RED before absolute deadline fix | `JAVA_HOME=/home/nirussvn0/Android/jdk/jdk-21.0.12.1+1 ./gradlew testDebugUnitTest --tests '*MjpegHttpServerTest*'` | **1**, intended failure: trickled request monopolized worker |
| `desktop/src-tauri`, final | `cargo test --locked` | **0**, 60 passed, two hardware smoke tests ignored |
| `desktop/src-tauri`, final | `cargo build --locked` | **0** |
| `desktop/src-tauri`, final | `cargo clippy --locked --all-targets -- -D warnings` | **0** |
| `android`, final | `JAVA_HOME=/home/nirussvn0/Android/jdk/jdk-21.0.12.1+1 ./gradlew testDebugUnitTest assembleDebug lintDebug` | **0**, 70 JVM tests passed, APK rebuilt, lint passed |
| repository root, final | `python -m unittest discover -s tests -p test_lan_security.py -v` | **0**, four passed |
| repository root | `git diff --check` | **0** |
| `desktop/src-tauri` | `cargo fmt -- --check` | **1**, restored baseline-only formatting defects in `profiles.rs`, `gst_preview.rs`, `virtual_output.rs`, `tests/gst_preview.rs`; not a new logic/test failure |

The first post-deadline Android run exited **1** because the test mistook a TLS
close alert for an open connection. The test now drains the alert to EOF and
requires the incomplete handshake/request to survive until the absolute budget.
These tests exercise trickled TLS handshake, headers and control body;
authenticated media still delivers a frame after 5.5 seconds, proving cancellation.
Rust tests exercise deadline cancellation, a trickled incomplete TLS handshake,
EOF cleanup/restart, and Stop during a trickled partial encrypted record.
Five storage/state tests cover failed/expired QR, late success after Stop,
superseded tickets, successful commit, recreation with the same token and secure
storage failure. These use an in-memory secure-storage adapter seam, not real
encryption or a phone; native encrypted preferences still require device proof.

Changed-module formatting check passed (**0**) with this exact command in
`desktop/src-tauri`:

```sh
rustfmt --check --edition 2021 --config skip_children=true src/core/control/socket_deadline.rs src/core/control/lan_tls.rs src/core/control/pairing_server.rs src/core/control/mod.rs src/core/media/stream_client.rs src/core/commands/session_commands.rs src/lib.rs src/bin/camaproctl.rs tests/preview_e2e.rs tests/stream_client.rs tests/common/mod.rs tests/lan_tls.rs tests/composition.rs
```

The four baseline formatting failures were independently reproduced at repository
root (**exit 1 each**), without changing worktree files:

```sh
git show 62bb814:desktop/src-tauri/src/core/profiles.rs | rustfmt --check --edition 2021
git show 62bb814:desktop/src-tauri/src/platform/linux/gst_preview.rs | rustfmt --check --edition 2021
git show 62bb814:desktop/src-tauri/src/platform/linux/virtual_output.rs | rustfmt --check --edition 2021
git show 62bb814:desktop/src-tauri/tests/gst_preview.rs | rustfmt --check --edition 2021
```

## Final confirmation-order verification (current)

| Directory | Exact command | Exit / result |
|---|---|---|
| repository root, RED | `python -m unittest discover -s tests -p test_lan_security.py -v` | **1**, new ordering regression failed because post-commit confirmation was absent; four existing checks passed |
| `desktop/src-tauri`, final | `cargo test --locked` | **0**, 67 passed, two hardware smoke tests ignored |
| `desktop/src-tauri`, final | `cargo clippy --locked --all-targets -- -D warnings` | **0** |
| `android`, final | `JAVA_HOME=/home/nirussvn0/Android/jdk/jdk-21.0.12.1+1 ./gradlew testDebugUnitTest assembleDebug lintDebug` | **0**, 71 JVM tests passed, APK rebuilt, lint passed |
| repository root, final | `python -m unittest discover -s tests -p test_lan_security.py -v` | **0**, five passed |
| repository root | `git diff --check` | **0** |

Six new production-state-machine tests check that prepare cannot invoke
store/emission, confirmation requires a fresh readiness probe before finalization,
one-time replay behavior, peer IP/pin/port/token/secret binding, expiry/no pending/
failed-ready rejection, original-QR expiry bounds, new-QR invalidation and exact
private route parsing. Store/emission hooks are injected test seams, not actual
keyring or Tauri event integration. A real Rust TLS socket test additionally
checks final-probe rejection of provisional/missing/duplicate readiness and
acceptance of the committed marker. Android tests check confirmation only for a
current committed ticket, Stop/supersession invalidation and retention of approved
trust after an ambiguous confirmation outcome. Source regressions check phone
confirmation occurs after commit **and** listener replacement.

Changed final-fix Rust files also passed (**exit 0**) formatting in
`desktop/src-tauri`:

```sh
rustfmt --check --edition 2021 --config skip_children=true src/core/control/pairing_server.rs src/core/control/lan_tls.rs tests/lan_tls.rs
```

No public JSON schema, Tauri command signature, or `phone-paired` IPC field was
changed by the final fix. Only private backend pairing routes/status headers were
extended. Physical-phone two-phase integration is still unverified; the existing
capture/lease HIGH risk below remains open.

Artifacts:
- `android/app/build/outputs/apk/debug/app-debug.apk`
- `android/app/build/test-results/testDebugUnitTest/TEST-*.xml`
- `android/app/build/reports/lint-results-debug.html`

## Exact changed paths

```text
android/app/build.gradle.kts
android/app/src/main/java/app/camapro/scope/CameraActivity.kt
android/app/src/main/java/app/camapro/scope/network/LanTls.kt
android/app/src/main/java/app/camapro/scope/network/LanCredentials.kt
android/app/src/main/java/app/camapro/scope/network/NetworkHelper.kt
android/app/src/main/java/app/camapro/scope/transport/MjpegHttpServer.kt
android/app/src/test/java/app/camapro/scope/network/LanTlsTest.kt
android/app/src/test/java/app/camapro/scope/network/LanCredentialsTest.kt
android/app/src/test/java/app/camapro/scope/network/NetworkHelperTest.kt
android/app/src/test/java/app/camapro/scope/transport/MjpegHttpServerTest.kt
desktop/src-tauri/Cargo.toml
desktop/src-tauri/Cargo.lock
desktop/src-tauri/src/bin/camaproctl.rs
desktop/src-tauri/src/core/commands/session_commands.rs
desktop/src-tauri/src/core/control/mod.rs
desktop/src-tauri/src/core/control/lan_tls.rs
desktop/src-tauri/src/core/control/pairing_server.rs
desktop/src-tauri/src/core/control/socket_deadline.rs
desktop/src-tauri/src/core/media/stream_client.rs
desktop/src-tauri/src/lib.rs
desktop/src-tauri/src/platform/linux/gst_preview.rs
desktop/src-tauri/tests/common/mod.rs
desktop/src-tauri/tests/composition.rs
desktop/src-tauri/tests/preview_e2e.rs
desktop/src-tauri/tests/lan_tls.rs
desktop/src-tauri/tests/stream_client.rs
tests/test_lan_security.py
tests/direct-lan-tls-evidence.md
```

Formatting-only changes to `profiles.rs`, `virtual_output.rs`, and
`tests/gst_preview.rs` were removed completely. `gst_preview.rs` retains only
the functional nonblocking-FIFO-reopen fix and its corrected ownership comment;
its original formatting was preserved.

## Acceptance limits / lead follow-up

- Physical phone AndroidKeyStore/JSSE-to-rustls mutual TLS, QR camera handoff,
  Camera2 delivery and LAN cleanup have not been exercised. Host Secret Service
  persistent storage is not exercised by tests (test enrollments are in memory).
  These are hardware/runtime acceptance requirements, not synthetic PASS claims.
- TLS 1.3 requires a phone provider supporting it (normally Android 10/API 29+).
  Older supported-installation-floor phones fail closed; no TLS/cleartext
  downgrade was added. IPv4 direct LAN is the implemented callback profile.
- **Unresolved HIGH — production capture/lease ownership:** phone **Start** calls
  `CameraActivity.startStreamingInternal`, opens Camera2 and starts capture
  immediately, before/during pairing. Desktop Disconnect/`preview_stop` shuts
  down only its reader, pump and GStreamer child; it sends no authenticated
  `session.stop`. `CameraActivity.processControlMessage` handles only
  `camera.set`, and the tested `SessionManager` watchdog is not the owner of this
  production Activity capture path. Disconnect, EOF or `active: false` therefore
  **does not prove that the phone stopped capturing**. The foreground service
  also does not own/confirm Activity camera teardown. Full D04 capture-free Ready,
  remote stop and watchdog cleanup remain unimplemented on this path; no Ready /
  idle-camera / remote-release / full-G2-G4 acceptance claim is made here.
- `virtual_output` remains a test-pattern producer; phone media fan-out is not
  delivered by this transport correction.
- The initial nested-review invocation was blocked by the harness. The user then
  supplied independent-review findings, addressed in this correction. No nested
  agents were invoked for the correction; lead owns post-fix review/acceptance.
- Frontend owner must retain explicit **Connect**, remove auto-start on
  `phone-paired`, and remove the desktop-IP-as-phone picker. No IPC pin field is
  needed. Canon/evidence updates under `docs/` remain with its owner.
