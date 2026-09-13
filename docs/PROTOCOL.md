# Control protocol — v1 design, not frozen

## Status and authority

[control-message.schema.json](../protocol/control-message.schema.json) currently validates only a permissive draft envelope. It does **not** validate per-message payloads, correlation, authentication, ranges or lifecycle semantics. It is preserved unchanged during replanning to avoid implying a frozen wire contract. G1 replaces it with typed variants plus cross-language fixtures before Android/Rust implementations diverge. The schema `$id` is an identifier, not proof of a hosted endpoint.

## Transport and trust

Control and media are separate logical channels bound to the same authenticated peer/session. Proposed D03: WSS control and HTTPS MJPEG with pinned identity bootstrapped by QR. The exact certificate/key provisioning and platform TLS library compatibility must be settled in G0; do not accept arbitrary/self-signed certificates merely because the network is local. Later RTP requires its own approved encryption/authentication profile.

Pairing flow to freeze in G1:

1. User opens pairing on the phone; generate an expiring one-time nonce/secret and expose only non-secret discovery metadata.
2. QR binds protocol version, endpoint hint, peer identity fingerprint, secret and expiry. Endpoint is not identity. User explicitly approves the peer.
3. Verify pinned TLS identity before sending secret; consume the secret atomically on successful enrollment. Reject expired/reused secrets and bound/rate-limit attempts.
4. Store long-lived credentials using platform secure storage; stop with actionable error if unavailable (no plaintext fallback). Define revocation and device-reset behavior.
5. Authenticate each control connection and media request. Use header-bound short-lived session authorization, not bearer tokens in URLs, mDNS or logs. Media authorization ends on stop/revocation/lease expiry.

An explicitly opted-in developer experiment may use a synthetic source on loopback or a controlled isolated test network; it does not satisfy pairing/security acceptance and must not become an ordinary real-camera LAN default.

## Planned message semantics

Base fields: `v: 1`, `type`, optional `id` depending on message kind. G1 fixes request IDs to JSON-safe integers (0 through 9007199254740991) scoped to a connection generation; callers must not reuse them within that connection. Version mismatch is rejected before command execution. Extensions live in a bounded namespaced extension object; unknown commands and unknown camera-control keys fail closed rather than silently succeeding.

| Type | Direction | ID / result | Payload to specify in G1 |
|---|---|---|---|
| hello | desktop → phone | request ID; response | supported major version, identity reference, client info; auth already established |
| capabilities | phone → desktop | event, no request ID | camera IDs, modes, ranges/steps/units, resolution/FPS combinations, revision |
| camera.state | phone → desktop | event, no request ID | applied values, camera ID, revision, stream state |
| camera.set | desktop → phone | request ID; response or error | camera ID, expected capability revision, typed non-empty changes |
| session.start | desktop → phone | request ID; response or error | selected mode/transport; return authenticated media descriptor/session generation |
| session.stop | desktop → phone | request ID; response or error | session generation; idempotent already-stopped result |
| ping | desktop → phone | request ID; pong | current lease/session generation |
| pong | phone → desktop | same ping ID | bounded liveness response; not permission to start capture |
| response | phone → desktop | correlated ID, ok=true | typed result for original operation |
| error | phone → desktop | correlated ID if request-related | typed code, safe message, retryability; no secrets |

A response acknowledges completed application of the command, not merely queue admission. `session.start` response means sender/endpoint ready; desktop enters Streaming only after its first valid decoded frame. `session.stop` acknowledges released capture/encoder resources. Events project confirmed state, never optimistic UI state.

Example **design only** (must become a valid fixture against the replacement schema in G1):

```json
{"v":1,"id":42,"type":"camera.set","payload":{"cameraId":"0","capabilityRevision":3,"changes":{"exposureCompensationSteps":2}}}
```

EV is integer compensation steps with device-reported rational EV-per-step, not a hardcoded float. ISO is integer sensitivity, shutter is integer nanoseconds, focus distance is diopters; validate representability across Kotlin/Rust/JS. Manual exposure is an atomic AE-off + ISO + shutter change, validated against selected FPS. Capability limits are device state, not static schema constants. Rejected control leaves effective state unchanged.

## Ordering, failure and recovery

- Serialize lifecycle/lens commands. Coalesce sliders locally only before wire ID assignment and only within the same camera, control and generation. Every transmitted request receives one terminal result or times out.
- G1 defines a bounded duplicate-response cache scoped to authenticated connection generation. Duplicate identical request IDs return cached terminal result without reapplying; same ID with changed content returns an error. Reconnect uses a new generation; never blindly replay pending mutations.
- Proposed timing defaults for G1 tests: ping every 2 s, phone lease expiry after 6 s without valid authenticated heartbeat, command deadline 5 s, start/stop deadline 10 s. Use monotonic clocks; malformed/unauthenticated traffic never renews the lease.
- Proposed reconnect budget: at most 5 attempts with 1/2/4/8/8 s base delays and bounded jitter. Cancellation is immediate; exhausted budget returns Disconnected. Successful reconnection establishes a new generation and invalidates the old lease; the phone releases any old-generation capture before reporting Ready. Only then refresh capabilities/effective state and return Ready without capture (D04). Old-generation heartbeats and media credentials cannot renew or access the new session.
- Stop cancels in-flight start and invalidates stale callbacks. Local startup/decoder failure after a remote start must enter stop cleanup. If remote release cannot be confirmed, abandon the lease and cease its heartbeats; report Disconnected rather than Ready. Process death releases local resources and phone watchdog releases capture. Pairing cancellation/expiry cleans all ephemeral state. Revocation closes both control and media.
- Minimum error taxonomy: unsupported_version, unauthenticated, forbidden, busy, invalid_payload, unsupported_control, stale_capabilities, invalid_state, timeout, permission_denied, media_failure, internal. G1 freezes exact shapes, directions and retry rules.

## Parser and resource limits

Proposed G1 defaults: 64 KiB maximum control message, bounded JSON depth, 32 pending commands per peer and explicit oversized/busy errors. G0 measures and fixes MJPEG maximum part size, parser buffered bytes, read timeout and total pipeline memory budget for supported modes. Parse multipart boundaries incrementally; reject corrupt/truncated/oversized input without unbounded buffering. Do not let untrusted media descriptors redirect to arbitrary hosts or local resources: bind to the authenticated peer and negotiated endpoint policy.

## Contract acceptance (G1)

Shared fixtures must cover every message kind, required IDs/payloads, malformed/oversized data, wrong direction, unsupported versions, unknown controls, out-of-range/device-specific values, manual exposure atomicity, duplicate IDs, changed duplicate content, expired/replayed pairing, unauthenticated media, revoked peers, delayed events and stale generations. Two mandatory G1 semantic regressions: (1) stop times out while the control socket remains healthy—lease heartbeats cease, UI stays Disconnected and remote capture expires; (2) reconnect succeeds before old lease expiry—old capture and media authorization end before Ready, and delayed old-generation heartbeats/callbacks cannot revive them. Schema validation alone cannot prove temporal or capability semantics; both Kotlin and Rust must execute the same semantic vectors. TypeScript consumes the same names/units without owning device logic. See [roadmap](ROADMAP.md) for paths and dependencies.
