package app.camapro.scope.network

class TrustStore {

    private val trustedPeers = mutableSetOf<String>()
    private val revokedPeers = mutableSetOf<String>()
    private val activeMediaTokens = mutableMapOf<String, String>()
    private val lock = Any()

    fun addTrustedPeer(peerFingerprintSha256: String) = synchronized(lock) {
        revokedPeers.remove(peerFingerprintSha256)
        trustedPeers.add(peerFingerprintSha256)
    }

    fun revokePeer(peerFingerprintSha256: String) = synchronized(lock) {
        trustedPeers.remove(peerFingerprintSha256)
        revokedPeers.add(peerFingerprintSha256)
        activeMediaTokens.remove(peerFingerprintSha256)
    }

    fun grantMediaToken(peerFingerprintSha256: String, token: String) = synchronized(lock) {
        if (trustedPeers.contains(peerFingerprintSha256) && !revokedPeers.contains(peerFingerprintSha256)) {
            activeMediaTokens[peerFingerprintSha256] = token
        }
    }

    fun authenticateControlConnection(peerFingerprintSha256: String): Result<Unit> = synchronized(lock) {
        if (revokedPeers.contains(peerFingerprintSha256) || !trustedPeers.contains(peerFingerprintSha256)) {
            return Result.failure(SecurityException("unauthenticated"))
        }
        Result.success(Unit)
    }

    fun authenticateMediaRequest(peerFingerprintSha256: String, mediaAuthToken: String?): Result<Unit> = synchronized(lock) {
        if (mediaAuthToken == null) {
            return Result.failure(SecurityException("unauthenticated"))
        }
        if (revokedPeers.contains(peerFingerprintSha256) || !trustedPeers.contains(peerFingerprintSha256)) {
            return Result.failure(SecurityException("unauthenticated"))
        }
        val expected = activeMediaTokens[peerFingerprintSha256]
        if (expected == null || expected != mediaAuthToken) {
            return Result.failure(SecurityException("unauthenticated"))
        }
        Result.success(Unit)
    }
}
