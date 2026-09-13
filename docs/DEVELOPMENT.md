# Development and verification

## Current runnable checks

There is no application source/toolchain configuration yet. Only JSON syntax and document/skill structural checks can run at this baseline. From repository root:

```bash
python -m json.tool protocol/control-message.schema.json > /dev/null
git status --short --branch
```

`git diff --check` does not inspect untracked files. At the initial audit all authored files were untracked; use a captured baseline plus direct file validation for review, rather than reporting an empty Git diff as clean validation. JSON syntax success is not protocol semantics success.

## Delivery loop

1. Select one gate/task in [ROADMAP.md](ROADMAP.md); load relevant skills from [AGENTS.md](../AGENTS.md).
2. Check prerequisite decisions/evidence and existing workspace changes. Define observable behavior and test before implementation.
3. Observe the test fail for its intended behavior, implement the minimum, rerun focused tests and relevant full gates.
4. Exercise the real integration. Use fakes for deterministic lifecycle tests, but label them; never present fake video as Camera2/device proof.
5. Update affected canon and evidence; get independent review at each major gate. Lead verifies outputs, then changes roadmap status. No autonomous commit/push/release.

## Future application commands (not runnable until G1 creates them)

G1 must pin tools/dependencies and establish these scripts. Run each from its specified directory and record actual exit status:

| Directory | Planned command | Checks |
|---|---|---|
| repository root | `python -m unittest discover -s protocol/tests -v` | fixture corpus after installing pinned `protocol/requirements-test.txt` in an isolated environment |
| `android/` | `./gradlew lintDebug testDebugUnitTest assembleDebug` | lint + JVM tests + debug APK |
| `desktop/src-tauri/` | `cargo fmt --all -- --check` | format |
| `desktop/src-tauri/` | `cargo clippy --all-targets --locked -- -D warnings` | target-appropriate features; never blanket mutually-exclusive all-features |
| `desktop/src-tauri/` | `cargo test --locked` | core/contract tests |
| `desktop/` | `pnpm install --frozen-lockfile` | reproducible JS dependency install |
| `desktop/` | `pnpm lint && pnpm typecheck && pnpm test && pnpm build` | controls UI; `test` must be non-watch |
| `desktop/` | `pnpm tauri build` | full desktop/native packaging, not just Vite bundle |

Android instrumentation/device scenarios are additional to JVM tests. Windows gets its own target build/OS execution. CI should fail when required test suites are absent; do not mark a skipped hardware gate PASS. Unit/contract CI and hardware acceptance are distinct statuses.

## Acceptance matrix

| Layer | Automated evidence | Integration/hardware evidence |
|---|---|---|
| Protocol | every message + negative fixtures, Kotlin/Rust parity, units, direction, duplicate semantics | authenticated cross-runtime exchange |
| Session | fake clock and fault-injected transition matrix | kill desktop, interrupt Wi-Fi, revoke permission, stop notification |
| Trust | wrong fingerprint, expired/reused secret, revocation, unauthenticated media, log redaction | actual pairing and secure persistence on reference OS/device |
| Media | bounded parser/queues, corrupt data, slow sink, generation isolation | real Camera2 → native surface/output; negotiated vs delivered mode |
| Platform | port-level fake tests; target build | OBS/browser actual frames, permissions, resize, install/uninstall |
| UI | capability-disabled states, errors, start/stop actions | no raw-frame IPC; clear Ready/Streaming/Reconnecting/degraded states |

## Performance measurement

G0 fixes numerical pass thresholds and reference hardware before G5. Start with 720p30 to retire integration risk; 1080p30 / <250 ms / 30 minutes remain Linux release goals. Production 1080p60 / <120 ms is exploratory until measured.

Record sender and receiver frame counters, delivered FPS distribution, drops, queue high-water marks, CPU and periodic RSS for both processes, and thermal/battery state. Define warm-up, sample interval, baseline and allowed RSS slope/peak before a run. “No growth” is not established by two screenshots or a stable short sample. Report stalls and disconnections, not only averages.

For end-to-end latency, use an external high-frame-rate recording of a common timer/reference and receiver display, or another documented clock-calibrated method. Unrelated phone/desktop wall-clock subtraction does not measure valid one-way latency. Report method, sample count, p50/p95 and uncertainty. Do not manufacture latency from target settings.

## Evidence record template

Future files live under `docs/evidence/` as each gate runs; do not generate empty PASS records now.

```text
Gate/task and verdict: PASS / FAIL / BLOCKED
Date, inspected commit and dirty/untracked scope:
Changed paths / artifact checksums:
Environment: device/model, Android, SDK, OS/compositor, app/tool versions
Decisions and prerequisites:
Commands: exact cwd + command + exit status + log/artifact path
Scenario: actual source/receiver/network, duration, selected and negotiated mode
Measurements: method, thresholds fixed before test, actual values and uncertainty
Fault cases and recovery/resource cleanup:
Evidence type: simulated / build / integrated / real hardware
Independent reviewer findings and lead verification:
Limitations and unresolved support rows:
```

Do not put tokens, keys, raw QR enrollment data or personal footage into evidence. Use synthetic test patterns for stored artifacts unless the user explicitly approves real footage retention.

## Linux release checklist (all currently unverified)

- [ ] Works on declared LAN without cloud and with authenticated control/media.
- [ ] Declared reference mode passes measured 30-minute limits.
- [ ] OBS and browser show current frames, not just a device name.
- [ ] Supported EV/focus/lens controls apply correctly; unsupported states are honest.
- [ ] Stop, desktop death, permission revocation and Wi-Fi loss release phone capture.
- [ ] Reconnect follows approved policy and does not silently restart capture.
- [ ] Bounded memory/latency under slow consumers; no JS video-frame bridge.
- [ ] Trusted-peer persistence, revocation and secure storage work.
- [ ] Clean-machine packaging/dependencies and licensing inventory verified.

Under proposed D11 this checklist is the G5 camera release, with profiles/CLI/hotkeys in G6. If the owner retains the broader original MVP scope, G6 must also pass before calling it the agreed MVP. Broad distro and Windows support always require their own evidence; this checklist never implies them.
