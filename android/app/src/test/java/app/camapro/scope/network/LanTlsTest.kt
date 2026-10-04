package app.camapro.scope.network

import org.junit.Assert.*
import org.junit.Test

class LanTlsTest {
    @Test fun `fingerprint hashes certificate bytes with sha256`() {
        assertEquals("sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", LanTls.fingerprint("abc".toByteArray()))
    }
    @Test fun `tokens are random 256 bit values and malformed pins fail`() {
        val a = LanTls.randomToken()
        val b = LanTls.randomToken()
        assertNotEquals(a, b)
        assertTrue(Regex("[0-9a-f]{64}").matches(a))
        assertTrue(LanTls.validPin("sha256:$a"))
        for (pin in listOf("", "camapro-android-v020", "sha256:00", "sha256:" + "AB".repeat(32))) assertFalse(LanTls.validPin(pin))
    }
}
