---
type: Protocol Specification
status: Frozen v1 Contract (Active)
version: 1.0.0
last_updated: 2026-10-01
owner: NirussVn0
authority: docs/PROTOCOL.md
---

# Control protocol — v1 specification

## 1. Scope and Security Model

Control and media are separate logical channels bound to the same authenticated peer/session:
* **Control Channel:** WebSocket (WSS) / HTTP JSON control messages.
* **Media Channel:** Authenticated MJPEG HTTP stream with bounded frames (future: encrypted H.264).
* **Pairing Flow (D03):** 
  1. Desktop generates an expiring one-time secret and displays QR binding callback endpoint IP, port, its actual certificate fingerprint, and secret. Phone scans and pins that identity.
  2. Phone calls the pinned HTTPS enrollment callback with its endpoint certificate fingerprint and authorization credential. Desktop verifies the phone through a pinned mutual TLS status probe. The initial QR secret is consumed atomically; a failed probe requires a fresh QR.
  3. Phone commits approved trust and confirms completion through a private pinned HTTPS callback. The pending completion ticket is single-use, bounded by the original QR expiry and 30 seconds, and bound to both endpoint credentials and callback peer address. Desktop verifies committed endpoint readiness before storing enrollment and emitting pairing success. Credentials persist in encrypted phone preferences and desktop `keyring-rs`; storage failures fail closed.
  4. Header-bound session authorization is required for every control and media request (no bearer tokens in URLs, mDNS, or logs).

The desktop-issued enrollment callbacks are bootstrap transport details, not new v1 control-message types. These mechanisms are implemented with synthetic transport tests; actual AndroidKeyStore/desktop interoperability still requires device qualification. The lifecycle rules below remain requirements: the current production Activity capture path does not yet integrate remote stop and watchdog ownership.

---

## 2. Message Envelopes and Types

All wire messages follow `v: 1` discriminated schema in `protocol/control-message.schema.json`. Request IDs are JSON-safe integers (0 to $2^{53}-1$) scoped to the connection generation.

| Type | Direction | Request ID | Payload Summary |
|---|---|---|---|
| `hello` | Desktop $\rightarrow$ Phone | Required | Supported major version, peer identity reference, client info. |
| `capabilities` | Phone $\rightarrow$ Desktop | Event (none) | Available camera IDs, supported modes, resolution/FPS combinations, capability revision. |
| `camera.state` | Phone $\rightarrow$ Desktop | Event (none) | Confirmed applied values, active camera ID, capability revision, stream state. |
| `camera.set` | Desktop $\rightarrow$ Phone | Required | Target camera ID, expected capability revision, typed non-empty control changes. |
| `session.start` | Desktop $\rightarrow$ Phone | Required | Selected mode and transport descriptor; returns session generation and media endpoint. |
| `session.stop` | Desktop $\rightarrow$ Phone | Required | Active session generation; idempotent release confirmation. |
| `ping` | Desktop $\rightarrow$ Phone | Required | Current lease/session generation. |
| `pong` | Phone $\rightarrow$ Desktop | Matching ping ID | Bounded liveness confirmation (does not authorize stream start). |
| `response` | Phone $\rightarrow$ Desktop | Correlated ID | `ok: true`, plus typed result for the completed operation. |
| `error` | Phone $\rightarrow$ Desktop | Correlated ID / optional | Typed error code, safe human-readable message, retryable flag. |

### Camera Control Units
* **EV Compensation:** Integer steps with device-reported rational step size (`exposureCompensationSteps`).
* **Manual Exposure:** Atomic transaction setting AE off + ISO (integer sensitivity) + shutter speed (integer nanoseconds).
* **Focus:** Diopters (float).
* **Fail-closed:** Unknown controls, out-of-range parameters, or unsupported hardware features fail closed without partial application.

---

## 3. Error Taxonomy

All error responses strictly use one of these 12 frozen codes:

| Error Code | Meaning | Retryable |
|---|---|---|
| `unsupported_version` | Major protocol version mismatch. | No |
| `unauthenticated` | Missing or invalid session token/identity. | No (re-auth required) |
| `forbidden` | Action not permitted for peer or current state. | No |
| `busy` | Subsystem currently executing a conflicting operation. | Yes (backoff) |
| `invalid_payload` | Schema validation or JSON syntax failure. | No |
| `unsupported_control` | Control property not supported by selected camera. | No |
| `stale_capabilities` | Expected capability revision does not match current camera state. | Yes (after refresh) |
| `invalid_state` | Command invalid in current lifecycle state (e.g. stop when idle). | No |
| `timeout` | Operation timed out before completion. | Yes |
| `permission_denied` | OS denied camera or hardware permission. | No |
| `media_failure` | Camera2 capture pipeline or encoder failure. | Yes |
| `internal` | Unexpected server/client internal runtime error. | No |

---

## 4. Timing Constants and Lifecycle Invariants

* **Ping Interval:** Every 2.0 s from desktop.
* **Lease Expiry:** 6.0 s without authenticated heartbeat $\rightarrow$ phone watchdog stops Camera2 capture immediately.
* **Command Deadline:** 5.0 s.
* **Session Start/Stop Deadline:** 10.0 s.
* **Reconnect Budget:** Maximum 5 attempts (backoff: 1s, 2s, 4s, 8s, 8s with jitter).
* **Generation Isolation:** Every reconnection creates a monotonically incremented connection generation. Stale generation completions, delayed events, and old-generation heartbeats are rejected immediately.
* **Idempotency:** Start and stop commands are idempotent. Duplicate request IDs with identical content return cached response; duplicate request IDs with changed content return `invalid_payload`.

---

## 5. Parser and Resource Limits

* **Max Control Message Size:** 64 KiB.
* **Max Pending Commands:** 32 in-flight requests per peer.
* **Multipart MJPEG Part Boundary:** Incremental parsing with bounded buffer; drop oldest frame on slow consumer (queue bound $\le 2$ frames).
