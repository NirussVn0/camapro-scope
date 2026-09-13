package app.camapro.scope.network

import org.junit.Assert.*
import org.junit.Before
import org.junit.Test

class TrustStoreTest {

    private lateinit var trustStore: TrustStore

    @Before
    fun setUp() {
        trustStore = TrustStore()
    }

    @Test
    fun `trusted peer is accepted for control connection`() {
        trustStore.addTrustedPeer("SHA256:trusted-1")
        val result = trustStore.authenticateControlConnection("SHA256:trusted-1")
        assertTrue(result.isSuccess)
    }

    @Test
    fun `revoked peer is rejected fail-closed`() {
        trustStore.addTrustedPeer("SHA256:trusted-1")
        trustStore.revokePeer("SHA256:trusted-1")

        val result = trustStore.authenticateControlConnection("SHA256:trusted-1")
        assertTrue(result.isFailure)
        assertEquals("unauthenticated", result.exceptionOrNull()?.message)
    }

    @Test
    fun `unauthenticated media request is rejected fail-closed`() {
        trustStore.addTrustedPeer("SHA256:trusted-1")

        // Missing media authorization
        val missingToken = trustStore.authenticateMediaRequest("SHA256:trusted-1", null)
        assertTrue(missingToken.isFailure)
        assertEquals("unauthenticated", missingToken.exceptionOrNull()?.message)

        // Invalid media authorization token
        val invalidToken = trustStore.authenticateMediaRequest("SHA256:trusted-1", "bad-token")
        assertTrue(invalidToken.isFailure)
        assertEquals("unauthenticated", invalidToken.exceptionOrNull()?.message)

        // Valid media authorization
        trustStore.grantMediaToken("SHA256:trusted-1", "valid-media-token-123")
        val valid = trustStore.authenticateMediaRequest("SHA256:trusted-1", "valid-media-token-123")
        assertTrue(valid.isSuccess)
    }
}
