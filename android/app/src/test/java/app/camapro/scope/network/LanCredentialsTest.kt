package app.camapro.scope.network

import org.junit.Assert.*
import org.junit.Test

class LanCredentialsTest {
    private val old = "sha256:" + "ab".repeat(32)
    private val new = "sha256:" + "cd".repeat(32)
    private fun credentials(storage: MutableMap<String, String>, writable: Boolean = true) = LanCredentials(
        { storage[it] }, { k, v -> if (writable) { storage[k] = v; true } else false }
    )

    @Test fun `confirmation requires current committed ticket and ambiguous outcome never rolls back`() {
        val storage = mutableMapOf("desktop-pin" to old)
        val credentials = credentials(storage)
        val attempt = credentials.begin(new, Long.MAX_VALUE)
        assertFalse(credentials.isCommitted(attempt))
        credentials.commit(attempt)
        assertTrue(credentials.isCommitted(attempt))
        // A lost final HTTP reply cannot revoke trust already confirmed remotely.
        assertFalse(credentials.abort(attempt))
        assertEquals(new, credentials.trustedPin())
        assertEquals(new, storage["desktop-pin"])
        credentials.cancel()
        assertFalse(credentials.isCommitted(attempt))
        val current = credentials.begin(old, Long.MAX_VALUE)
        assertFalse(credentials.isCommitted(attempt))
        assertFalse(credentials.isCommitted(current))
    }

    @Test fun `failure or Stop preserves prior persisted trust and rejects late success`() {
        val storage = mutableMapOf("desktop-pin" to old)
        val credentials = credentials(storage)
        val attempt = credentials.begin(new, Long.MAX_VALUE)
        assertEquals(old, storage["desktop-pin"])
        assertTrue(credentials.abort(attempt))
        assertEquals(old, credentials.trustedPin())
        val stopped = credentials.begin(new, Long.MAX_VALUE)
        credentials.cancel()
        try { credentials.commit(stopped); fail("Stop invalidates callback") } catch (_: IllegalStateException) {}
        assertEquals(old, storage["desktop-pin"])
    }

    @Test fun `only current successful generation persists trust`() {
        val storage = mutableMapOf("desktop-pin" to old)
        val credentials = credentials(storage)
        val stale = credentials.begin(old, Long.MAX_VALUE)
        val current = credentials.begin(new, Long.MAX_VALUE)
        try { credentials.commit(stale); fail("Stale generation") } catch (_: IllegalStateException) {}
        assertFalse(credentials.abort(stale))
        assertEquals(old, storage["desktop-pin"])
        credentials.commit(current)
        assertEquals(new, storage["desktop-pin"])
        assertEquals(new, credentials.trustedPin())
    }

    @Test fun `recreated credentials retain the same random token and storage failure is terminal`() {
        val storage = mutableMapOf<String, String>()
        val first = credentials(storage).token
        assertEquals(first, credentials(storage).token)
        assertTrue(Regex("[0-9a-f]{64}").matches(first))
        try { credentials(mutableMapOf(), false); fail("No plaintext or ephemeral fallback") } catch (_: IllegalStateException) {}
        storage["auth-token"] = "corrupt"
        try { credentials(storage); fail("Corrupt token fails closed") } catch (_: IllegalStateException) {}
    }

    @Test fun `failed final trust write leaves old identity intact`() {
        val storage = mutableMapOf("desktop-pin" to old, "auth-token" to "ef".repeat(32))
        val credentials = credentials(storage, false)
        val attempt = credentials.begin(new, Long.MAX_VALUE)
        try { credentials.commit(attempt); fail("Secure storage failure") } catch (_: IllegalStateException) {}
        credentials.abort(attempt)
        assertEquals(old, credentials.trustedPin())
        assertEquals(old, storage["desktop-pin"])
    }

    @Test fun `expired QR or response cannot change persisted trust`() {
        val storage = mutableMapOf("desktop-pin" to old, "auth-token" to "ef".repeat(32))
        var now = 1L
        val credentials = LanCredentials({ storage[it] }, { k, v -> storage[k] = v; true }, { now })
        try { credentials.begin(new, now); fail("Expired QR") } catch (_: IllegalArgumentException) {}
        val attempt = credentials.begin(new, 10L)
        now = 10L
        try { credentials.commit(attempt); fail("Expired callback") } catch (_: IllegalStateException) {}
        credentials.abort(attempt)
        assertEquals(old, storage["desktop-pin"])
        assertEquals(old, credentials.trustedPin())
    }
}
