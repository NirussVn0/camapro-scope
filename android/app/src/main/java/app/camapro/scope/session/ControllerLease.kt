package app.camapro.scope.session

data class ControllerLease(
    val controllerId: String,
    val token: String,
    var expiresAtMs: Long
) {
    fun isExpired(nowMs: Long): Boolean = nowMs > expiresAtMs
}
