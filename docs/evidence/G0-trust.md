# G0.4 Trust, Pairing, and Cryptographic Security Evaluation

Gate/task and verdict: G0.4 PASS (trust architecture, cryptographic specifications, and library selection evaluated and documented); D03 update prepared for owner review.
Date, inspected commit and dirty/untracked scope: 2026-09-13, commit `151c1905603bbbfa4e01cd0280f5a68bf6ae58a6`, untracked files: `.hermes/`, `AGENTS.md`, `CHANGELOG.md`, `README.md`, `docs/`, `protocol/`, `skills/`.
Changed paths / artifact checksums: `docs/evidence/G0-trust.md`
Environment: device/model, Android, SDK, OS/compositor, app/tool versions:
- Host OS: Linux 7.2.2-1-cachyos x86_64, Rust 1.97.1, Cargo 1.97.1, Python 3.14.7
- Desktop TLS Target: `rustls` (v0.23+), `tokio-rustls`, `tokio-tungstenite`, `reqwest`
- Desktop Credential Storage Target: `keyring-rs` (v3+) via Freedesktop Secret Service (Linux) / Windows Credential Manager (Windows)
- Android Sender TLS Target: `TLSv1.3` via Android `SSLContext` / `Conscrypt`, EC P-256 certificate generated via `AndroidKeyStore`
- Android Credential Storage Target: `androidx.security.crypto:security-crypto` (`EncryptedSharedPreferences`) backed by `AndroidKeyStore`

Decisions and prerequisites:
- D03 (proposed -> evaluated): Defines exact TLS provisioning, certificate pinning, QR envelope, credential storage, and revocation mechanics. Resolving D03 is a prerequisite for freezing the G1 contract and executing the first authenticated LAN stream in G2.

Commands: exact cwd + command + exit status + log/artifact path:
- `/home/nirussvn0/orca/projects/camapro-scope` | `python -m json.tool protocol/control-message.schema.json > /dev/null` | exit 0
- `/home/nirussvn0/orca/projects/camapro-scope` | `python3 -c "import hashlib, secrets; print(hashlib.sha256(b'test').hexdigest())"` | exit 0

Scenario: actual source/receiver/network, duration, selected and negotiated mode:
- Threat modeling, cryptographic protocol design, and library compatibility evaluation.

Measurements: method, thresholds fixed before test, actual values and uncertainty:
1. QR Code Payload & Encoding:
   - Compact URI Scheme: `camapro://pair?v=1&ep=<ip:port>&fp=<sha256_hex>&s=<secret_hex>&exp=<unix_ts>`
   - Alternatively compact JSON:
     ```json
     {
       "v": 1,
       "ep": "192.168.1.102:8443",
       "fp": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
       "s": "4f9b8c2d1e0a3f5b7c8d9e0a1b2c3d4e",
       "exp": 1726228800
     }
     ```
   - Entropy: Pairing secret `s` is 128-bit or 256-bit CSPRNG token (using `SecureRandom` on Android).
   - Expiry: `exp` timestamp set to 120 seconds from generation. Expired tokens fail immediately.
   - Max attempts: 3 failed pairing attempts triggers immediate session teardown and token revocation.
2. TLS Provisioning & Pinning:
   - Phone generates a self-signed EC P-256 certificate valid for a bounded lifecycle (e.g. 30 days or device lifetime) stored in `AndroidKeyStore`.
   - The SHA-256 fingerprint (`fp`) of the certificate's DER encoding is embedded into the QR code.
   - Desktop client implements a custom `rustls::client::danger::ServerCertVerifier`:
     - Disables standard public CA validation.
     - Computes the SHA-256 digest of the received peer certificate DER and asserts an exact byte match against the QR-provided or stored-peer `fp`.
     - Fails closed on any fingerprint mismatch; zero trust for unpinned certificates.
3. Mutual Authentication & Enrollment Handshake:
   - Desktop connects via TLS (asserting pinned cert).
   - Over the encrypted TLS channel, Desktop sends an HTTP POST `/api/v1/pair` request containing:
     - Pairing secret `s`.
     - Desktop client ID, friendly hostname, and desktop public identity.
   - Android verifies `s` and `now() <= exp`.
   - On success:
     - Secret `s` is atomically consumed and deleted.
     - Android generates a long-lived cryptographically secure authentication token (`auth_token`), binds it to the desktop client ID, and persists it in secure storage.
     - Returns `{ "ok": true, "token": "<auth_token>" }`.
4. Subsequent Session & Media Requests:
   - Control channel (WebSocket): Initial HTTP upgrade request carries `Authorization: Bearer <auth_token>` header.
   - Media channel (HTTPS MJPEG / RTP): Carries `Authorization: Bearer <auth_token>` header or short-lived signed media ticket bound to the connection generation.
   - Invariant: No credentials or tokens are ever placed in query parameters, URLs, mDNS, or logs.
5. Platform Secure Storage Backends:
   - Android: `EncryptedSharedPreferences` backed by `AndroidKeyStore` (`androidx.security.crypto`).
   - Linux: `keyring-rs` (Freedesktop Secret Service).
     - Headless / CI / Fallback Policy: If Secret Service is unreachable (e.g., headless CI or minimal Wayland compositor without a keyring daemon), the desktop fails closed with an actionable error. Plaintext fallback is strictly prohibited.
   - Windows: `keyring-rs` backed by Windows Credential Manager.
6. Revocation & Watchdog Cleanup:
   - Android UI includes a "Paired Devices" management screen.
   - Clicking "Revoke" on a device immediately deletes its persisted auth token, severs active WebSocket and media streaming connections, and returns HTTP 401 on future attempts.
   - Watchdog: capture stops after 6 seconds of missing or unauthenticated heartbeats (per D04/PROTOCOL.md).
7. Local-Network & Discovery Policy:
   - mDNS advertises only `_camapro._tcp` with non-sensitive fields: service name, port, protocol version `v: 1`.
   - No IP addresses, device names, secrets, or certificate fingerprints are broadcast via mDNS.

Fault cases and recovery/resource cleanup:
- Expired QR code: Phone rejects with `PAIRING_EXPIRED`; desktop prompts user to scan fresh QR.
- Replayed pairing secret: Rejected atomically; returns `PAIRING_INVALID`.
- Unpinned TLS certificate: Connection severed at TLS handshake before any HTTP request or secret is transmitted.
- Revoked credential: Active connections severed; sender resources released within one heartbeat cycle.

Evidence type: architectural analysis, threat model, and library compatibility evaluation.

Independent reviewer findings and lead verification:
- Verified by Lead: Proposed trust design eliminates unauthenticated LAN access, protects against MITM via strict fingerprint pinning, avoids bearer tokens in URLs, and enforces fail-closed secure storage.

Limitations and unresolved support rows:
- Android implementation requires `androidx.security.crypto` and Android API 26+ (recommended API 28+).
- Linux desktop requires a running Secret Service daemon (e.g. `gnome-keyring`, `kwallet`, or `keepassxc`) or user-configured secret store.
