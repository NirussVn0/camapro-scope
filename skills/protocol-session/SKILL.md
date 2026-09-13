---
name: protocol-session
description: "Use when changing Camapro Scope wire messages, pairing, commands or lifecycle; require shared fixtures and total cleanup/recovery semantics."
version: 1.0.0
author: Camapro Scope contributors
metadata:
  hermes:
    tags: [camapro-scope, project-workflow]
    related_skills: []
---

# Protocol and session changes

## Overview
Read `docs/PROTOCOL.md`, architecture lifecycle table and relevant decisions before editing Android/Rust contracts. The initial schema is only a draft envelope; never infer safety from syntactic validation.

## When to use
Message schemas/types, command semantics, capability units, authentication, lease/watchdog, retries or reconnect. For pixel/OS behavior also load media-platform.

## Procedure
1. Identify producer, consumer, direction, authorization and owning state transition. Resolve applicable open decisions before freezing a wire contract. Completion: units, limits and completion semantics are explicit.
2. Add valid/invalid fixtures under `protocol/fixtures/` for each changed message. Add semantic vectors for state/range/replay behavior that JSON Schema cannot express. Completion: a negative case fails for the expected behavioral reason, not a missing dependency.
3. Specify required ID, payload, error shape, supported version and bounds. Test JSON-safe integers, missing fields, unknown controls and unsupported device capabilities. Completion: a permissive object shape cannot pass as typed validation.
4. Update schema and Kotlin/Rust consumers together; both run the same fixture corpus. Keep effective camera state authoritative; failed atomic sensor updates must not partially apply. Completion: parity evidence with exact commands and exits.
5. Walk each lifecycle state against start/stop/cancel/timeout, permission loss, process death and stale callbacks. Use fake monotonic clock and connection generations. Completion: every path releases its resources and has a defined retry/terminal state; no automatic capture after reconnect unless policy is explicitly changed.
6. Test duplicate request IDs, changed duplicate payload, bounded cache expiry, canceled in-flight start and late events. If remote stop cannot be confirmed after a local failure, abandon the lease and stop heartbeats rather than projecting Ready while the phone still captures. Never blindly retry toggle or pending mutations on a new connection.
7. Exercise authentication on both control and media: wrong identity, expired/replayed enrollment, revoked peer, unauthenticated stream and oversized parser input. Completion: fail closed without secrets in logs/URLs or traffic renewing a lease before auth.
8. Update canon and rerun affected protocol/session tests. Future commands are in development docs; inspect manifests before executing them. Require real peer/device cleanup at the hardware gate, not only mocks.

## Common pitfalls
- Transport-connected is not authenticated or Streaming.
- An accepted command is not an applied camera value.
- A heartbeat response must not reactivate capture.
- A valid schema cannot prove ordering, revocation, atomicity or capability ranges.

## Verification checklist
- [ ] Contract corpus and both consumers agree.
- [ ] All changed transitions include cleanup and stale-generation rejection.
- [ ] Authentication/media authorization and replay failures tested.
- [ ] No claim of hardware success based only on a fake-clock test.
