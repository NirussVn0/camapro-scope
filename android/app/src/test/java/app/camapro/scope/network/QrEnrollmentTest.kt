package app.camapro.scope.network

import org.junit.Assert.*
import org.junit.Before
import org.junit.Test

class QrEnrollmentTest {

    private lateinit var enrollmentManager: EnrollmentManager

    @Before
    fun setUp() {
        enrollmentManager = EnrollmentManager()
    }

    @Test
    fun `successful enrollment consumes secret and binds fingerprint`() {
        val payload = enrollmentManager.issueEnrollment(
            endpointHint = "192.168.1.50:8443",
            peerFingerprintSha256 = "SHA256:abcdef1234567890",
            secret = "one-time-secret-xyz",
            ttlMs = 60_000,
            nowMs = 1000
        )

        assertEquals("192.168.1.50:8443", payload.endpointHint)
        assertEquals(61_000L, payload.expiresAtMs)

        // Valid consumption before expiry
        val consumeResult = enrollmentManager.consumeEnrollment(
            peerFingerprintSha256 = "SHA256:abcdef1234567890",
            secret = "one-time-secret-xyz",
            nowMs = 5000
        )
        assertTrue(consumeResult.isSuccess)
    }

    @Test
    fun `expired QR pairing secret is rejected fail-closed`() {
        enrollmentManager.issueEnrollment(
            endpointHint = "192.168.1.50:8443",
            peerFingerprintSha256 = "SHA256:abcdef1234567890",
            secret = "secret-1",
            ttlMs = 10_000,
            nowMs = 1000
        )

        // Attempting to consume after expiresAtMs (11_000)
        val result = enrollmentManager.consumeEnrollment(
            peerFingerprintSha256 = "SHA256:abcdef1234567890",
            secret = "secret-1",
            nowMs = 11_001
        )
        assertTrue(result.isFailure)
        assertEquals("unauthenticated", result.exceptionOrNull()?.message)
    }

    @Test
    fun `replaying an already consumed secret fails closed`() {
        enrollmentManager.issueEnrollment(
            endpointHint = "192.168.1.50:8443",
            peerFingerprintSha256 = "SHA256:abcdef1234567890",
            secret = "secret-replay",
            ttlMs = 60_000,
            nowMs = 1000
        )

        val first = enrollmentManager.consumeEnrollment("SHA256:abcdef1234567890", "secret-replay", 2000)
        assertTrue(first.isSuccess)

        // Second attempt must fail closed (replay attack prevention)
        val replay = enrollmentManager.consumeEnrollment("SHA256:abcdef1234567890", "secret-replay", 3000)
        assertTrue(replay.isFailure)
        assertEquals("unauthenticated", replay.exceptionOrNull()?.message)
    }

    @Test
    fun `fingerprint mismatch is rejected`() {
        enrollmentManager.issueEnrollment(
            endpointHint = "192.168.1.50:8443",
            peerFingerprintSha256 = "SHA256:valid-fingerprint",
            secret = "secret-match",
            ttlMs = 60_000,
            nowMs = 1000
        )

        val mismatch = enrollmentManager.consumeEnrollment("SHA256:wrong-fingerprint", "secret-match", 2000)
        assertTrue(mismatch.isFailure)
    }
}
