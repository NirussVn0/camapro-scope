package app.camapro.scope.network

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class NetworkHelperTest {

    @Test
    fun classifiesTailscaleInterfaceAndIpsCorrectly() {
        assertEquals(
            NetworkHelper.EndpointType.TAILSCALE,
            NetworkHelper.classifyEndpoint("tailscale0", "100.85.12.34")
        )
        assertEquals(
            NetworkHelper.EndpointType.TAILSCALE,
            NetworkHelper.classifyEndpoint("tun0", "100.100.5.6")
        )
        assertTrue(NetworkHelper.isTailscaleIp("100.64.0.1"))
        assertTrue(NetworkHelper.isTailscaleIp("100.127.255.254"))
        assertFalse(NetworkHelper.isTailscaleIp("100.63.255.255"))
        assertFalse(NetworkHelper.isTailscaleIp("100.128.0.0"))
        assertFalse(NetworkHelper.isTailscaleIp("192.168.1.1"))
    }

    @Test
    fun classifiesWifiAndPrivateLanIpsCorrectly() {
        assertEquals(
            NetworkHelper.EndpointType.WIFI_LAN,
            NetworkHelper.classifyEndpoint("wlan0", "192.168.1.100")
        )
        assertEquals(
            NetworkHelper.EndpointType.WIFI_LAN,
            NetworkHelper.classifyEndpoint("eth0", "10.0.0.15")
        )
        assertEquals(
            NetworkHelper.EndpointType.WIFI_LAN,
            NetworkHelper.classifyEndpoint("enp8s0", "172.20.1.2")
        )
        assertTrue(NetworkHelper.isPrivateLanIp("192.168.0.1"))
        assertTrue(NetworkHelper.isPrivateLanIp("10.255.0.1"))
        assertTrue(NetworkHelper.isPrivateLanIp("172.16.0.1"))
        assertTrue(NetworkHelper.isPrivateLanIp("172.31.255.255"))
        assertFalse(NetworkHelper.isPrivateLanIp("172.32.0.1"))
        assertFalse(NetworkHelper.isPrivateLanIp("8.8.8.8"))
    }

    @Test
    fun classifiesLoopbackCorrectly() {
        assertEquals(
            NetworkHelper.EndpointType.LOOPBACK,
            NetworkHelper.classifyEndpoint("lo", "127.0.0.1")
        )
    }

    @Test
    fun createsValidPairingJsonMatchingSchema() {
        val jsonStr = NetworkHelper.createPairingJson("http://192.168.1.50:8100", "my-secret-token")
        val json = JSONObject(jsonStr)

        assertEquals(1, json.getInt("version"))
        assertEquals("http://192.168.1.50:8100", json.getString("endpoint_hint"))
        assertEquals("my-secret-token", json.getString("secret"))
        assertTrue(json.has("peer_fingerprint_sha256"))
        assertTrue(json.getLong("expires_at_ms") > System.currentTimeMillis())
    }
}
