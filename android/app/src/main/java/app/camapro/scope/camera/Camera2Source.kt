package app.camapro.scope.camera

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.graphics.ImageFormat
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CaptureRequest
import android.os.Handler
import android.os.HandlerThread
import android.util.Range
import android.view.Surface
import androidx.core.content.ContextCompat

/**
 * Production Camera2 hardware implementation of CameraSource.
 * Directly streams hardware ISP JPEG frames via ImageReader into onFrame callback.
 * Invariant D05: Single Camera2 owner; bounded 2-frame hardware queue.
 */
class Camera2Source(
    private val context: Context,
    var previewSurface: Surface? = null
) : CameraSource {

    private val cameraManager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
    private var cameraDevice: CameraDevice? = null
    private var captureSession: CameraCaptureSession? = null
    private var imageReader: android.media.ImageReader? = null

    private var backgroundThread: HandlerThread? = null
    private var backgroundHandler: Handler? = null

    private val lock = Any()
    private var isClosed = false

    @SuppressLint("MissingPermission")
    override fun openCamera(
        cameraId: String,
        onOpened: () -> Unit,
        onError: (errorCode: Int, message: String) -> Unit
    ) = synchronized(lock) {
        if (ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
            onError(-1, "Camera permission not granted")
            return
        }

        startBackgroundThread()

        val targetCameraId = resolveCameraId(cameraId)
        if (targetCameraId == null) {
            onError(-2, "No suitable camera found on device")
            return
        }

        try {
            cameraManager.openCamera(
                targetCameraId,
                object : CameraDevice.StateCallback() {
                    override fun onOpened(camera: CameraDevice) {
                        synchronized(lock) {
                            if (isClosed) {
                                camera.close()
                                return
                            }
                            cameraDevice = camera
                        }
                        onOpened()
                    }

                    override fun onDisconnected(camera: CameraDevice) {
                        synchronized(lock) {
                            camera.close()
                            cameraDevice = null
                        }
                        onError(-3, "Camera disconnected")
                    }

                    override fun onError(camera: CameraDevice, error: Int) {
                        synchronized(lock) {
                            camera.close()
                            cameraDevice = null
                        }
                        onError(error, "Camera open error: code $error")
                    }
                },
                backgroundHandler
            )
        } catch (e: Exception) {
            onError(-4, "Exception opening camera: ${e.message}")
        }
    }

    override fun startCapture(
        width: Int,
        height: Int,
        fps: Int,
        onFrame: (ByteArray) -> Unit
    ): Boolean = synchronized(lock) {
        val device = cameraDevice ?: return false
        val handler = backgroundHandler ?: return false

        try {
            // maxImages = 2 ensures strict 2-frame bounded buffer at the ISP/reader level
            val reader = android.media.ImageReader.newInstance(width, height, ImageFormat.JPEG, 2)
            reader.setOnImageAvailableListener({ r ->
                val image = r.acquireLatestImage() ?: return@setOnImageAvailableListener
                try {
                    val planes = image.planes
                    if (planes.isNotEmpty()) {
                        val buffer = planes[0].buffer
                        val bytes = ByteArray(buffer.remaining())
                        buffer.get(bytes)
                        onFrame(bytes)
                    }
                } catch (_: Exception) {
                    // Stale or recycled frame; drop gracefully
                } finally {
                    image.close()
                }
            }, handler)
            imageReader = reader

            val surfaces = mutableListOf<Surface>(reader.surface)
            previewSurface?.let { if (it.isValid) surfaces.add(it) }

            val captureRequestBuilder = device.createCaptureRequest(CameraDevice.TEMPLATE_RECORD).apply {
                addTarget(reader.surface)
                previewSurface?.let { if (it.isValid) addTarget(it) }

                set(CaptureRequest.CONTROL_AF_MODE, CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_VIDEO)
                set(CaptureRequest.CONTROL_AE_MODE, CaptureRequest.CONTROL_AE_MODE_ON)
                set(CaptureRequest.CONTROL_AWB_MODE, CaptureRequest.CONTROL_AWB_MODE_AUTO)

                // Select closest available target FPS range
                selectFpsRange(device.id, fps)?.let { range ->
                    set(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, range)
                }
            }

            var sessionConfigured = false
            val sessionLock = Object()

            @Suppress("DEPRECATION")
            device.createCaptureSession(
                surfaces,
                object : CameraCaptureSession.StateCallback() {
                    override fun onConfigured(session: CameraCaptureSession) {
                        synchronized(lock) {
                            captureSession = session
                            try {
                                session.setRepeatingRequest(captureRequestBuilder.build(), null, handler)
                            } catch (_: Exception) {
                            }
                        }
                        synchronized(sessionLock) {
                            sessionConfigured = true
                            sessionLock.notifyAll()
                        }
                    }

                    override fun onConfigureFailed(session: CameraCaptureSession) {
                        synchronized(sessionLock) {
                            sessionConfigured = false
                            sessionLock.notifyAll()
                        }
                    }
                },
                handler
            )

            synchronized(sessionLock) {
                if (!sessionConfigured) {
                    sessionLock.wait(2500)
                }
            }
            return sessionConfigured
        } catch (e: Exception) {
            return false
        }
    }

    override fun stopCapture(): Unit = synchronized(lock) {
        try {
            captureSession?.stopRepeating()
            captureSession?.abortCaptures()
            captureSession?.close()
        } catch (_: Exception) {
        } finally {
            captureSession = null
        }

        try {
            imageReader?.close()
        } catch (_: Exception) {
        } finally {
            imageReader = null
        }
    }

    override fun close(): Unit = synchronized(lock) {
        isClosed = true
        stopCapture()
        try {
            cameraDevice?.close()
        } catch (_: Exception) {
        } finally {
            cameraDevice = null
        }
        stopBackgroundThread()
    }

    private fun resolveCameraId(requestedId: String): String? {
        val ids = cameraManager.cameraIdList
        if (ids.isEmpty()) return null

        if (requestedId.isNotBlank() && ids.contains(requestedId)) {
            return requestedId
        }

        // Prefer back camera
        for (id in ids) {
            val characteristics = cameraManager.getCameraCharacteristics(id)
            val facing = characteristics.get(CameraCharacteristics.LENS_FACING)
            if (facing == CameraCharacteristics.LENS_FACING_BACK) {
                return id
            }
        }
        return ids.firstOrNull()
    }

    private fun selectFpsRange(cameraId: String, targetFps: Int): Range<Int>? {
        return try {
            val characteristics = cameraManager.getCameraCharacteristics(cameraId)
            val ranges = characteristics.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES) ?: return null
            ranges.minByOrNull { range ->
                Math.abs(range.upper - targetFps)
            }
        } catch (_: Exception) {
            null
        }
    }

    private fun startBackgroundThread() {
        if (backgroundThread == null) {
            backgroundThread = HandlerThread("Camera2Bg").apply {
                start()
                backgroundHandler = Handler(looper)
            }
        }
    }

    private fun stopBackgroundThread() {
        backgroundThread?.quitSafely()
        try {
            backgroundThread?.join(500)
        } catch (_: Exception) {
        }
        backgroundThread = null
        backgroundHandler = null
    }
}
