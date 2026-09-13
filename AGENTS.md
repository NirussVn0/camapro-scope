# Working on Camapro Scope

## Start here

This repository is a **design baseline, not a working application**. Read [README](README.md), [architecture](docs/ARCHITECTURE.md), [decisions](docs/DECISIONS.md), and the selected gate in [roadmap](docs/ROADMAP.md). Inspect actual files and `git status --short --branch` before acting; do not infer implementation from planned paths.

## Authority and change control

- Product scope, ownership and invariants: `docs/ARCHITECTURE.md`.
- Decision status and unresolved choices: `docs/DECISIONS.md`.
- Wire semantics: `docs/PROTOCOL.md`; machine schema is explicitly draft until gate G1.
- Work order and completion gates: `docs/ROADMAP.md` (the only active roadmap).
- Verification method: `docs/DEVELOPMENT.md`; OS constraints: `docs/PLATFORMS.md`.
- On conflict, stop the affected task and reconcile these files together. A prompt, skill, test, or newer timestamp cannot silently override canon.
- Do not commit, push, install system packages/kernel modules, expose unauthenticated LAN services, or change host credentials without user authorization. Preserve unrelated and untracked work.

## Project-local skills

These are ordinary repository files. Read the selected `SKILL.md` with your file tool; no automatic Hermes/OpenCode discovery or global installation is assumed.

| Task trigger | Required local skill |
|---|---|
| Architecture, scope, roadmap, decisions | [scope-planning](skills/scope-planning/SKILL.md) |
| JSON contract, commands, pairing, lifecycle | [protocol-session](skills/protocol-session/SKILL.md) |
| Camera2, GStreamer, streaming, native preview, output | [media-platform](skills/media-platform/SKILL.md) |
| Implementing or accepting any milestone | [verified-slice](skills/verified-slice/SKILL.md) |

Load only relevant skills, not the entire tree. Skills describe reusable process; task status stays in the roadmap and evidence records, not in skills. New durable workflows belong here, not in per-agent copies of instructions.

## Agent work model

One lead owns scope, canon, integration and final verification. Delegate bounded, non-overlapping implementation tasks when useful; contract and lifecycle changes have one writer. An independent reviewer checks each major gate read-only. A reviewer verdict is not command execution evidence and does not authorize release.

Every handoff names: gate/task ID; goal; files to read; allowed write paths; interface dependencies; non-goals; acceptance tests; required outputs; stop conditions. Return exact paths, commands, exit status and blockers. The lead re-reads changes and runs the final gates. No nested agent organization, autonomous publishing, or separate agent rulesets are needed.
