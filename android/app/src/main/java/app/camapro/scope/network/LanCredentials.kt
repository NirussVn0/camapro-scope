package app.camapro.scope.network

/** One provisional enrollment generation; encrypted storage remains unchanged until commit. */
class LanCredentials(
    private val read: (String) -> String?,
    private val write: (String, String) -> Boolean,
    private val nowMs: () -> Long = { System.currentTimeMillis() }
) {
    class Enrollment internal constructor(val generation: Long, val pin: String, val expiresAtMs: Long)
    private var generation = 0L
    private var pending: Enrollment? = null
    private var trusted = read("desktop-pin")?.also {
        check(LanTls.validPin(it)) { "Invalid stored desktop identity" }
    }

    val token: String = read("auth-token")?.also {
        check(Regex("[0-9a-f]{64}").matches(it)) { "Invalid stored LAN credential" }
    } ?: LanTls.randomToken().also {
        check(write("auth-token", it)) { "Secure token storage unavailable" }
    }

    @Synchronized fun trustedPin(): String? = trusted
    @Synchronized fun begin(pin: String, expiresAtMs: Long): Enrollment {
        require(LanTls.validPin(pin)) { "Invalid desktop certificate pin" }
        require(nowMs() < expiresAtMs) { "Pairing QR expired" }
        return Enrollment(++generation, pin, expiresAtMs).also { pending = it }
    }
    @Synchronized fun commit(enrollment: Enrollment) {
        check(pending == enrollment) { "Stale enrollment" }
        check(nowMs() < enrollment.expiresAtMs) { "Pairing QR expired" }
        check(write("desktop-pin", enrollment.pin)) { "Secure trust storage unavailable" }
        trusted = enrollment.pin
        pending = null
    }
    @Synchronized fun abort(enrollment: Enrollment): Boolean {
        if (pending != enrollment) return false
        pending = null
        generation++
        return true
    }
    @Synchronized fun isCommitted(enrollment: Enrollment): Boolean =
        generation == enrollment.generation && pending == null && trusted == enrollment.pin && nowMs() < enrollment.expiresAtMs
    @Synchronized fun cancel() { pending = null; generation++ }
}
