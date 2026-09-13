package app.camapro.scope.session

import app.camapro.scope.camera.CameraEngine

/**
 * Android SessionManager.
 * Invariant D04: One phone + one controlling desktop lease.
 * Reconnect returns Ready and requires explicit start; no silent camera reactivation.
 * Owns controller lease, stream lifecycle, and watchdog.
 * Must NOT access Camera2 directly outside CameraEngine.
 */
class SessionManager(
    private val cameraEngine: CameraEngine
) {
    @Volatile
    var state: SessionState = SessionState.DISCONNECTED
        private set

    @Volatile
    var activeLease: ControllerLease? = null
        private set

    private val lock = Any()

    fun acquireLease(
        controllerId: String,
        token: String,
        ttlMs: Long,
        nowMs: Long
    ): Result<Unit> = synchronized(lock) {
        val current = activeLease
        if (current != null && !current.isExpired(nowMs) && current.controllerId != controllerId) {
            return Result.failure(IllegalStateException("Active lease held by controller ${current.controllerId}"))
        }

        // If reconnecting while streaming, ensure previous stream is cleanly stopped
        if (state == SessionState.STREAMING) {
            cameraEngine.stop()
        }

        activeLease = ControllerLease(
            controllerId = controllerId,
            token = token,
            expiresAtMs = nowMs + ttlMs
        )
        // Invariant D04: Reconnect returns READY, never STREAMING
        state = SessionState.READY
        Result.success(Unit)
    }

    fun heartbeat(controllerId: String, token: String, ttlMs: Long, nowMs: Long): Result<Unit> = synchronized(lock) {
        val lease = validateLease(controllerId, token, nowMs).getOrElse { return Result.failure(it) }
        lease.expiresAtMs = nowMs + ttlMs
        Result.success(Unit)
    }

    fun startStream(
        controllerId: String,
        token: String,
        cameraId: String,
        width: Int,
        height: Int,
        fps: Int,
        nowMs: Long,
        onFrame: (ByteArray) -> Unit
    ): Result<Unit> = synchronized(lock) {
        validateLease(controllerId, token, nowMs).getOrElse { return Result.failure(it) }

        if (state == SessionState.STREAMING) {
            return Result.failure(IllegalStateException("Already streaming"))
        }

        val startResult = cameraEngine.start(cameraId, width, height, fps, onFrame)
        if (startResult.isFailure) {
            state = SessionState.ERROR
            return startResult
        }

        state = SessionState.STREAMING
        Result.success(Unit)
    }

    fun stopStream(controllerId: String, token: String): Result<Unit> = synchronized(lock) {
        val lease = activeLease
        if (lease == null || lease.controllerId != controllerId || lease.token != token) {
            return Result.failure(SecurityException("Unauthorized controller or invalid token"))
        }

        cameraEngine.stop()
        state = SessionState.READY
        Result.success(Unit)
    }

    fun releaseLease(controllerId: String, token: String): Result<Unit> = synchronized(lock) {
        val lease = activeLease
        if (lease == null || lease.controllerId != controllerId || lease.token != token) {
            return Result.failure(SecurityException("Unauthorized controller or invalid token"))
        }

        if (state == SessionState.STREAMING) {
            cameraEngine.stop()
        }
        activeLease = null
        state = SessionState.DISCONNECTED
        Result.success(Unit)
    }

    fun watchdogCheck(nowMs: Long): Unit = synchronized(lock) {
        val lease = activeLease ?: return
        if (lease.isExpired(nowMs)) {
            if (state == SessionState.STREAMING) {
                cameraEngine.stop()
            }
            activeLease = null
            state = SessionState.DISCONNECTED
        }
    }

    private fun validateLease(controllerId: String, token: String, nowMs: Long): Result<ControllerLease> {
        val lease = activeLease ?: return Result.failure(IllegalStateException("No active lease"))
        if (lease.isExpired(nowMs)) {
            activeLease = null
            state = SessionState.DISCONNECTED
            return Result.failure(IllegalStateException("Lease has expired"))
        }
        if (lease.controllerId != controllerId || lease.token != token) {
            return Result.failure(SecurityException("Controller ID or token mismatch"))
        }
        return Result.success(lease)
    }
}
