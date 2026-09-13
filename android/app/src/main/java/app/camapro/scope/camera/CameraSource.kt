package app.camapro.scope.camera

/**
 * Hardware/platform port for camera device acquisition.
 * Allows decoupling Camera2 API details from lifecycle and test fakes.
 */
interface CameraSource {
    fun openCamera(
        cameraId: String,
        onOpened: () -> Unit,
        onError: (errorCode: Int, message: String) -> Unit
    )

    fun startCapture(
        width: Int,
        height: Int,
        fps: Int,
        onFrame: (ByteArray) -> Unit
    ): Boolean

    fun stopCapture()

    fun close()
}
