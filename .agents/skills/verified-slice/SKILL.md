---
name: verified-slice
description: "Use when implementing or accepting a Camapro Scope milestone; require red-green tests, integration evidence and independent review before completion claims."
version: 1.0.0
author: Camapro Scope contributors
metadata:
  hermes:
    tags: [camapro-scope, project-workflow]
    related_skills: []
---

# Verified slice delivery

## Overview
Use `AGENTS.md`, the selected gate in `docs/ROADMAP.md` and `docs/DEVELOPMENT.md`. Deliver one observable behavior, not a forest of placeholders. Read other local skills when the slice touches their boundaries.

## When to use
Any implementation task, worker handoff, milestone acceptance, regression fix or release review.

## Procedure
1. Inspect workspace status, manifests and prerequisite evidence. State allowed write paths and explicit exclusions. Completion: no unrelated changes or pending decisions are silently incorporated.
2. Define a behavioral acceptance scenario and failing automated test. Execute and record intended failure before changing production behavior. For a feasibility spike, record the concrete hypothesis and measurement instead of inventing TDD evidence.
3. Implement the smallest coherent path through real owners/adapters. Run the focused test to green and inspect output; an import error is not a useful red test. Completion: no mock substituted for integration while reporting delivered behavior.
4. Run applicable format/lint/contract/unit/build checks from the directories in development docs, but only after those commands exist. Record commands, exits, artifacts and missing prerequisites. Missing required tests/toolchains are BLOCKED, not skipped PASS.
5. Exercise the slice end to end and its cleanup/error scenario. Hardware gates require actual phone/consumer output; ordinary CI may only establish synthetic integration. Completion: evidence category is explicit.
6. Review touched code/docs, secret handling and dependency boundaries. Update canon if behavior changed; keep unresolved decisions open. Completion: docs do not advertise absent features.
7. Major gate: ask a separate read-only reviewer to inspect exact files/evidence and return path-cited findings. Lead verifies fixes and reruns final checks after the last edit. Do not treat a reviewer summary as build/runtime proof.
8. Write an evidence record and update roadmap status only when all required checks pass. Final report: what changed, actual verification, limits and remaining blockers; commit/push only when authorized.

## Common pitfalls
- Frontend build alone does not prove Tauri/native packaging.
- JVM unit tests alone do not prove Camera2 hardware/service cleanup.
- A clean tracked diff ignores untracked files.
- Earlier green tests are stale after a relevant final edit.
- No hardware available means hardware acceptance blocked, not a reduced definition of done.

## Verification checklist
- [ ] Failing behavior then passing behavior, or honest measured spike.
- [ ] Fresh relevant full gates and inspected artifacts.
- [ ] Real integration and required fault paths exercised.
- [ ] Independent major-gate review resolved; completion claims match evidence.
