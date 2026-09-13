package app.camapro.scope.camera

import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger

class CameraEngineTest {

    private lateinit var fakeSource: FakeCameraSource
    private lateinit var engine: CameraEngine

    class FakeCameraSource : CameraSource {
        var openSuccess = true
        var startCaptureSuccess = true
        var permissionGranted = true

        val openCount = AtomicInteger(0)
        val closeCount = AtomicInteger(0)
        val startCaptureCount = AtomicInteger(0)
        val stopCaptureCount = AtomicInteger(0)

        var frameCallback: ((ByteArray) -> Unit)? = null

        override fun openCamera(
            cameraId: String,
            onOpened: () -> Unit,
            onError: (errorCode: Int, message: String) -> Unit
        ) {
            openCount.incrementAndGet()
            if (!permissionGranted) {
                onError(1, "Permission denied")
                return
            }
            if (!openSuccess) {
                onError(2, "Hardware error opening camera")
                return
            }
            onOpened()
        }

        override fun startCapture(
            width: Int,
            height: Int,
            fps: Int,
            onFrame: (ByteArray) -> Unit
        ): Boolean {
            startCaptureCount.incrementAndGet()
            if (!startCaptureSuccess) {
                return false
            }
            frameCallback = onFrame
            return true
        }

        override fun stopCapture() {
            stopCaptureCount.incrementAndGet()
            frameCallback = null
        }

        override fun close() {
            closeCount.incrementAndGet()
            frameCallback = null
        }

        fun emitFrame(data: ByteArray) {
            frameCallback?.invoke(data)
        }
    }

    @Before
    fun setUp() {
        fakeSource = FakeCameraSource()
        engine = CameraEngine(fakeSource)
    }

    @Test
    fun `initial state is idle`() {
        assertEquals(CameraEngine.State.IDLE, engine.state)
    }

    @Test
    fun `successful start transitions to streaming and receives frames`() {
        val received = AtomicInteger(0)
        val result = engine.start("0", 1280, 720, 30) {
            received.incrementAndGet()
        }

        assertTrue(result.isSuccess)
        assertEquals(CameraEngine.State.STREAMING, engine.state)
        assertEquals(1, fakeSource.openCount.get())
        assertEquals(1, fakeSource.startCaptureCount.get())

        fakeSource.emitFrame(byteArrayOf(1, 2, 3))
        assertEquals(1, received.get())
    }

    @Test
    fun `stop is idempotent and deterministic`() {
        engine.start("0", 1280, 720, 30) {}
        assertEquals(CameraEngine.State.STREAMING, engine.state)

        engine.stop()
        assertEquals(CameraEngine.State.IDLE, engine.state)
        assertEquals(1, fakeSource.stopCaptureCount.get())
        assertEquals(1, fakeSource.closeCount.get())

        // Second stop should be a no-op
        engine.stop()
        assertEquals(CameraEngine.State.IDLE, engine.state)
        assertEquals(1, fakeSource.stopCaptureCount.get())
        assertEquals(1, fakeSource.closeCount.get())
    }

    @Test
    fun `start while already streaming is rejected`() {
        val first = engine.start("0", 1280, 720, 30) {}
        assertTrue(first.isSuccess)

        val second = engine.start("0", 1920, 1080, 30) {}
        assertTrue(second.isFailure)
        assertEquals(CameraEngine.State.STREAMING, engine.state)
        // Should not have opened camera a second time
        assertEquals(1, fakeSource.openCount.get())
    }

    @Test
    fun `denied permissions transitions to error and ensures camera is closed`() {
        fakeSource.permissionGranted = false

        val result = engine.start("0", 1280, 720, 30) {}
        assertTrue(result.isFailure)
        assertEquals(CameraEngine.State.ERROR, engine.state)
        assertEquals(1, fakeSource.closeCount.get())
    }

    @Test
    fun `partial start failure closes open camera deterministically`() {
        fakeSource.startCaptureSuccess = false

        val result = engine.start("0", 1280, 720, 30) {}
        assertTrue(result.isFailure)
        assertEquals(CameraEngine.State.ERROR, engine.state)
        assertEquals(1, fakeSource.openCount.get())
        assertEquals(1, fakeSource.closeCount.get())
    }
}
