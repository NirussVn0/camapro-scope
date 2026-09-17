package app.camapro.scope.network

import android.graphics.Bitmap
import android.graphics.Color
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.qrcode.QRCodeWriter
import com.google.zxing.qrcode.decoder.ErrorCorrectionLevel
import org.json.JSONObject
import java.net.Inet4Address
import java.net.NetworkInterface
import java.util.Collections

/**
 * Network endpoint discovery and QR code utilities for LAN and Tailscale connectivity.
 */
object NetworkHelper {

    enum class EndpointType {
        TAILSCALE,
        WIFI_LAN,
        LOOPBACK,
        OTHER
    }

    data class NetworkEndpoint(
        val interfaceName: String,
        val ip: String,
        val type: EndpointType,
        val streamUrl: String
    ) {
        val displayLabel: String
            get() = when (type) {
                EndpointType.TAILSCALE -> "Tailscale: $ip"
                EndpointType.WIFI_LAN -> "Wi-Fi LAN: $ip"
                EndpointType.LOOPBACK -> "ADB Loopback: $ip"
                EndpointType.OTHER -> "$interfaceName: $ip"
            }
    }

    /**
     * Discovers all active IPv4 endpoints (Wi-Fi, Tailscale, Localhost)
     * and constructs streaming URLs.
     */
    fun getAvailableEndpoints(port: Int, token: String): List<NetworkEndpoint> {
        val endpoints = mutableListOf<NetworkEndpoint>()

        try {
            val interfaces = Collections.list(NetworkInterface.getNetworkInterfaces())
            for (intf in interfaces) {
                if (!intf.isUp || intf.isLoopback) continue

                val addresses = Collections.list(intf.inetAddresses)
                for (addr in addresses) {
                    if (addr is Inet4Address && !addr.isLoopbackAddress) {
                        val ip = addr.hostAddress ?: continue
                        val type = classifyEndpoint(intf.name, ip)
                        val streamUrl = "http://$ip:$port/stream?token=$token"
                        endpoints.add(
                            NetworkEndpoint(
                                interfaceName = intf.name,
                                ip = ip,
                                type = type,
                                streamUrl = streamUrl
                            )
                        )
                    }
                }
            }
        } catch (_: Exception) {
            // Fallback to basic loopback if network enumeration fails
        }

        // Always include localhost loopback for ADB port forwarding
        endpoints.add(
            NetworkEndpoint(
                interfaceName = "lo",
                ip = "127.0.0.1",
                type = EndpointType.LOOPBACK,
                streamUrl = "http://127.0.0.1:18100/stream?token=$token"
            )
        )

        // Sort: Tailscale first, then Wi-Fi, then others, then Loopback
        return endpoints.sortedBy { ep ->
            when (ep.type) {
                EndpointType.TAILSCALE -> 0
                EndpointType.WIFI_LAN -> 1
                EndpointType.OTHER -> 2
                EndpointType.LOOPBACK -> 3
            }
        }
    }

    /**
     * Classifies an IP address into Tailscale, Wi-Fi/LAN, or Other.
     * Tailscale uses the Carrier-Grade NAT (CGNAT) block: 100.64.0.0/10 (100.64.0.0 - 100.127.255.255)
     * or interfaces named tailscale0 / tun0.
     */
    fun classifyEndpoint(interfaceName: String, ip: String): EndpointType {
        val lowerName = interfaceName.lowercase()
        if (lowerName.contains("tailscale") || lowerName.startsWith("ts") || isTailscaleIp(ip)) {
            return EndpointType.TAILSCALE
        }

        if (lowerName.contains("wlan") || lowerName.contains("eth") || isPrivateLanIp(ip)) {
            return EndpointType.WIFI_LAN
        }

        if (ip == "127.0.0.1") {
            return EndpointType.LOOPBACK
        }

        return EndpointType.OTHER
    }

    fun isTailscaleIp(ip: String): Boolean {
        val parts = ip.split(".").mapNotNull { it.toIntOrNull() }
        if (parts.size != 4) return false
        // 100.64.0.0/10 -> first octet 100, second octet 64..127
        return parts[0] == 100 && parts[1] in 64..127
    }

    fun isPrivateLanIp(ip: String): Boolean {
        val parts = ip.split(".").mapNotNull { it.toIntOrNull() }
        if (parts.size != 4) return false
        return when (parts[0]) {
            10 -> true
            172 -> parts[1] in 16..31
            192 -> parts[1] == 168
            else -> false
        }
    }

    /**
     * Generates a QR Code Bitmap for desktop pairing or quick player access.
     */
    fun generateQrBitmap(content: String, sizePx: Int = 512): Bitmap {
        val hints = HashMap<EncodeHintType, Any>().apply {
            put(EncodeHintType.CHARACTER_SET, "UTF-8")
            put(EncodeHintType.ERROR_CORRECTION, ErrorCorrectionLevel.M)
            put(EncodeHintType.MARGIN, 2)
        }

        val writer = QRCodeWriter()
        val bitMatrix = writer.encode(content, BarcodeFormat.QR_CODE, sizePx, sizePx, hints)
        val bitmap = Bitmap.createBitmap(sizePx, sizePx, Bitmap.Config.ARGB_8888)

        for (x in 0 until sizePx) {
            for (y in 0 until sizePx) {
                bitmap.setPixel(x, y, if (bitMatrix.get(x, y)) Color.BLACK else Color.WHITE)
            }
        }
        return bitmap
    }

    /**
     * Constructs a standard pairing payload JSON that matches QrEnrollment schema.
     */
    fun createPairingJson(
        endpointUrl: String,
        token: String,
        ttlMs: Long = 3600_000L
    ): String {
        return JSONObject().apply {
            put("version", 1)
            put("endpoint_hint", endpointUrl)
            put("secret", token)
            put("peer_fingerprint_sha256", "camapro-android-v020")
            put("expires_at_ms", System.currentTimeMillis() + ttlMs)
        }.toString()
    }
}
