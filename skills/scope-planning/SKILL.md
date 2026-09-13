---
name: scope-planning
description: "Use when changing Camapro Scope architecture, decisions, scope or milestones; reconcile canon and turn risks into evidence gates."
version: 1.0.0
author: Camapro Scope contributors
metadata:
  hermes:
    tags: [camapro-scope, project-workflow]
    related_skills: []
---

# Scope planning

## Overview
Keep one active roadmap and explicit decision ownership. Read `docs/ARCHITECTURE.md`, `docs/DECISIONS.md`, `docs/ROADMAP.md` and the actual files implicated by the change.

## When to use
Architecture, scope changes, risk spikes, task decomposition or agent handoffs. Not a substitute for executing media/protocol tests.

## Procedure
1. Capture `git status --short --branch` and `git rev-parse HEAD`; inspect untracked files too. Completion: distinguish pre-existing work from your modifications.
2. Trace the real entrypoint → command owner → session → media/platform/persistence. For a docs-only baseline, say no runtime exists. Completion: each claim labeled planned, implemented, integrated or verified.
3. Classify findings: contradiction, missing boundary, unresolved product policy, or missing verification. Cite exact paths and observed behavior; do not score quality numerically without a rubric.
4. Propose the smallest corrective decision in `docs/DECISIONS.md`, including alternatives and evidence. New security, support, capture-resume or product-scope policies remain proposed/open until approved. Completion: no worker must invent a policy to proceed.
5. Update the existing roadmap in dependency order; give exact paths, observable result, tests, hardware needs and stop criteria. Retire stale instructions in the implementation prompt rather than creating another versioned roadmap.
6. Route implementation through `AGENTS.md`. Handoff one bounded task with allowed write paths, prerequisites and required command/evidence outputs. Completion: one writer per shared contract and a reviewer for each major gate.
7. Check links and read all affected canon together. Completion: no contradictory phase names, false implemented claims or unreviewed support promises.

## Common pitfalls
- Folder scaffolding masquerading as architectural progress: prove a slice instead.
- Implicitly approving a proposal by coding it: stop at the owner gate.
- Copying task status into skills: status belongs in roadmap/evidence only.
- Empty Git diff on untracked content: inspect direct file changes against a baseline.

## Verification checklist
- [ ] Evidence-backed diagnosis and preserved unrelated work.
- [ ] One active roadmap, coherent decision statuses and agent routing.
- [ ] Tests and hardware limits explicitly distinguishable from goals.
