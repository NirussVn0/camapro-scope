package app.camapro.scope.camera

/**
 * Sole owner of the camera hardware and capture session lifecycle.
 * Invariant D05: Single Camera2 owner; idempotent start/stop and deterministic cleanup.
 */
class CameraEngine(private val cameraSource: CameraSource) {

    enum class State {
        IDLE,
        STARTING,
        STREAMING,
        ERROR
    }

    @Volatile
    var state: State = State.IDLE
        private set

    private val lock = Any()

    fun start(
        cameraId: String,
        width: Int,
        height: Int,
        fps: Int,
        onFrame: (ByteArray) -> Unit
    ): Result<Unit> = synchronized(lock) {
        if (state == State.STREAMING || state == State.STARTING) {
            return Result.failure(IllegalStateException("Camera is already running in state $state"))
        }

        state = State.STARTING
        var openError: String? = null
        var opened = false

        cameraSource.openCamera(
            cameraId = cameraId,
            onOpened = {
                opened = true
            },
            onError = { code, msg ->
                openError = "Failed to open camera ($code): $msg"
            }
        )

        if (!opened || openError != null) {
            state = State.ERROR
            cameraSource.close()
            return Result.failure(IllegalStateException(openError ?: "Camera could not be opened"))
        }

        val captureStarted = cameraSource.startCapture(width, height, fps, onFrame)
        if (!captureStarted) {
            state = State.ERROR
            cameraSource.stopCapture()
            cameraSource.close()
            return Result.failure(IllegalStateException("Failed to start camera capture pipeline"))
        }

        state = State.STREAMING
        Result.success(Unit)
    }

    fun stop(): Unit = synchronized(lock) {
        if (state == State.IDLE) {
            return
        }
        try {
            cameraSource.stopCapture()
        } finally {
            cameraSource.close()
            state = State.IDLE
        }
    }

    companion object {
        val SUPPORTED_CONTROLS = setOf(
            "exposureCompensationSteps",
            "aeEnabled",
            "iso",
            "shutterNanos",
            "focusDistanceDiopters"
        )
    }

    /**
     * G4 live controls: validate against protocol schema and apply to CameraSource.
     * Fail-closed: unsupported controls are rejected immediately.
     */
    fun setControl(
        cameraId: String,
        capabilityRevision: Int,
        changes: Map<String, Any?>
    ): Result<Unit> = synchronized(lock) {
        if (changes.isEmpty()) {
            return Result.failure(IllegalArgumentException("changes must not be empty"))
        }
        val unsupported = changes.keys.filterNot { it in SUPPORTED_CONTROLS }
        if (unsupported.isNotEmpty()) {
            return Result.failure(IllegalArgumentException("Unsupported controls: $unsupported"))
        }
        val applied = cameraSource.setControl(changes)
        if (!applied) {
            return Result.failure(IllegalStateException("Failed to apply camera controls"))
        }
        return Result.success(Unit)
    }
}
