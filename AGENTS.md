---
type: Agent Guidelines & Skill Routing
status: Active Canon
version: 0.3.0
last_updated: 2026-09-27
owner: NirussVn0
authority: AGENTS.md
---

# Working on Camapro Scope

## Start here

This repository is an **active application codebase (v0.3.0)** with core Android capture (Camera2, foreground service, MJPEG server) and Linux desktop preview/virtual camera (Tauri 2, GStreamer Waylandsink, V4L2 loopback) implemented. Read [README](README.md), the [documentation index](docs/README.md), [architecture](docs/ARCHITECTURE.md), [decisions](docs/DECISIONS.md), and the selected gate in [roadmap](docs/ROADMAP.md). Inspect actual files and `git status --short --branch` before acting; do not infer implementation from planned paths.

## Authority and change control

- Technical documentation index & governance: `docs/README.md`.
- Product scope, ownership and invariants: `docs/ARCHITECTURE.md`.
- Decision status and unresolved choices: `docs/DECISIONS.md`.
- Wire semantics: `docs/PROTOCOL.md`; machine schema is frozen v1.
- Work order and completion gates: `docs/ROADMAP.md` (the only active roadmap).
- Verification method: `docs/DEVELOPMENT.md`; OS constraints: `docs/PLATFORMS.md`.
- On conflict, stop the affected task and reconcile these files together. A prompt, skill, test, or newer timestamp cannot silently override canon.
- Do not commit, push, install system packages/kernel modules, expose unauthenticated LAN services, or change host credentials without user authorization. Preserve unrelated and untracked work.

## Project-local skills

Skills are task-specific operational runbooks located in `.agents/skills/`. They provide step-by-step procedures, commands, and anti-pitfall checklists for specific implementation domains.

Follow the **strict one-way flow**: Task $\rightarrow$ `AGENTS.md` (pick skill) $\rightarrow$ `.agents/skills/` (runbook) $\rightarrow$ `docs/` (authoritative specs). Documents in `docs/` do not refer backwards to skills.

| Task trigger | Required local skill |
|---|---|
| Architecture, scope, roadmap, decisions | [scope-planning](.agents/skills/scope-planning/SKILL.md) |
| JSON contract, commands, pairing, lifecycle | [protocol-session](.agents/skills/protocol-session/SKILL.md) |
| Camera2, GStreamer, streaming, native preview, output | [media-platform](.agents/skills/media-platform/SKILL.md) |
| Implementing or accepting any milestone | [verified-slice](.agents/skills/verified-slice/SKILL.md) |

Load only the relevant skill for your immediate task. Skills describe reusable process; task status stays in the roadmap and evidence records, not in skills.

## Agent work model

One lead owns scope, canon, integration and final verification. Delegate bounded, non-overlapping implementation tasks when useful; contract and lifecycle changes have one writer. An independent reviewer checks each major gate read-only. A reviewer verdict is not command execution evidence and does not authorize release.

Every handoff names: gate/task ID; goal; files to read; allowed write paths; interface dependencies; non-goals; acceptance tests; required outputs; stop conditions. Return exact paths, commands, exit status and blockers. The lead re-reads changes and runs the final gates. No nested agent organization, autonomous publishing, or separate agent rulesets are needed.
