---
type: Documentation Index & Governance Guide
status: Active Canon
version: 0.3.0
last_updated: 2026-09-27
owner: NirussVn0
authority: docs/README.md
---

# Camapro Scope — Documentation Directory & Governance

## 1. Purpose & Core Principles

This directory (`docs/`) is the **Single Source of Truth (SSOT)** for Camapro Scope's system architecture, technical decisions, wire protocol, platform constraints, and execution milestones.

To keep documentation clean, reliable, and free from circular loops or fragmentation ("chống lộn tùng phèo") during future development, all contributors and AI agents must adhere to the following principles:

1. **Docs = System Specification (The "What" & "Why"):** Documents in `docs/` define the system invariants, architectural boundaries, data contracts, and factual progress. They are system specifications, not execution runbooks.
2. **Skills = Task Runbooks (The "How"):** Actionable task procedures, TDD steps, checklists, and anti-pitfall guides belong in `.agents/skills/`, NOT in `docs/`.
3. **Strict One-Way Direction:** 
   $$\text{Task} \longrightarrow \text{AGENTS.md} \longrightarrow \text{.agents/skills/} \longrightarrow \text{docs/}$$
   Agents read the task, load the relevant operational skill in `.agents/skills/`, and reference `docs/` for authoritative technical truth. Documents in `docs/` never loop backwards to skills.
4. **Universal Metadata Header:** Every markdown file in `docs/` must begin with standard YAML frontmatter and a Markdown attribute table right below the H1 heading.

---

## 2. Document Map & Taxonomy

| File | Type | Role & Scope | Authority Level |
|---|---|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Architecture Specification | System runtime ownership, dataflow (Android $\rightarrow$ Desktop), component boundaries, invariants (bounded queues, single Camera2 owner). | System Canon (Highest) |
| [DECISIONS.md](DECISIONS.md) | Decision Ledger | Record of all technical choices (D01–D11) with status (retained / accepted / proposed / open). No worker may invent unapproved policies. | Architectural Choice Canon |
| [ROADMAP.md](ROADMAP.md) | Execution Roadmap | The ONLY active roadmap (Gates G0 to G8). Tracks completion gates, hardware dependencies, and verification criteria. | Work Order Canon |
| [PROTOCOL.md](PROTOCOL.md) | Protocol Specification | Wire message format, envelope definitions, error taxonomy, lease/watchdog timeout rules, and 43 shared test fixtures. | Contract Canon |
| [PLATFORMS.md](PLATFORMS.md) | Platform Specification | OS minimums (Android minSdk 26, Linux Wayland GStreamer 1.28+, Windows 11 MF), hardware requirements, and platform adapter boundaries. | Platform Canon |
| [DESIGN.md](DESIGN.md) | UI/UX Design System | Desktop & mobile visual design canon: color tokens, 3-column layout, StageFrame preview, and mobile floating sheets. | Design Canon |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Development & Verification | Runnable local test/build commands, CI rules, performance measurement standards, and release checklist. | Verification Canon |
| [ASSESSMENT.md](ASSESSMENT.md) | Historical Technical Audit | Archived baseline audit (2026-09-13) before code implementation. Kept for historical context only. | Superseded / Archive |

---

## 3. Authority & Precedence Rules

When resolving technical questions, ambiguities, or conflicts:
1. `ARCHITECTURE.md` defines what the system is and the invariants that must never be broken (e.g. no raw video frames through JS, CameraEngine owns Camera2).
2. `DECISIONS.md` defines approved choices. A proposal cannot be treated as accepted without owner approval.
3. `PROTOCOL.md` defines the exact wire contract. Code must match the protocol schema and fixtures, not vice-versa.
4. `ROADMAP.md` is the only active work order. No agent or developer may create alternative or competing milestone roadmaps.
5. On conflict between documentation and code: stop and reconcile; code is wrong unless an explicit architectural decision changes the spec.

---

## 4. Documentation Maintenance Rules

1. **Header Format:** Every document MUST start with standard YAML frontmatter containing: `type`, `status`, `version`, `last_updated`, `owner`, and `authority`. Do not duplicate this metadata into redundant markdown tables.
2. **Updating Timestamps:** Whenever a document's technical content is modified, update `last_updated: YYYY-MM-DD` in the frontmatter.
3. **No Orphan or Competing Docs:** Never create secondary roadmaps, duplicate checklists, or scratch planning files in `docs/`. Retain historical snapshots only if explicitly marked as archived.
4. **Clean Boundaries with Skills:** Keep `docs/` pure technical documentation. Put agent-specific instructions and procedural rules into `.agents/skills/`.
