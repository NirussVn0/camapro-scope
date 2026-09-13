package app.camapro.scope.network

data class QrPayload(
    val version: Int,
    val endpointHint: String,
    val peerFingerprintSha256: String,
    val secret: String,
    val expiresAtMs: Long
)

class EnrollmentManager {

    private data class EnrollmentRecord(
        val payload: QrPayload,
        var consumed: Boolean = false
    )

    private val records = mutableMapOf<String, EnrollmentRecord>()
    private val lock = Any()

    fun issueEnrollment(
        endpointHint: String,
        peerFingerprintSha256: String,
        secret: String,
        ttlMs: Long,
        nowMs: Long
    ): QrPayload = synchronized(lock) {
        val payload = QrPayload(
            version = 1,
            endpointHint = endpointHint,
            peerFingerprintSha256 = peerFingerprintSha256,
            secret = secret,
            expiresAtMs = nowMs + ttlMs
        )
        records[secret] = EnrollmentRecord(payload)
        payload
    }

    fun consumeEnrollment(
        peerFingerprintSha256: String,
        secret: String,
        nowMs: Long
    ): Result<Unit> = synchronized(lock) {
        val record = records[secret] ?: return Result.failure(SecurityException("unauthenticated"))

        if (record.consumed) {
            // Replay attack prevention: fails closed
            return Result.failure(SecurityException("unauthenticated"))
        }

        if (nowMs > record.payload.expiresAtMs) {
            // Expired secret: fails closed
            return Result.failure(SecurityException("unauthenticated"))
        }

        if (record.payload.peerFingerprintSha256 != peerFingerprintSha256) {
            return Result.failure(SecurityException("Fingerprint mismatch"))
        }

        record.consumed = true
        Result.success(Unit)
    }
}
