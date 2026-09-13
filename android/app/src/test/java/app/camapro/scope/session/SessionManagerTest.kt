package app.camapro.scope.session

import app.camapro.scope.camera.CameraEngine
import app.camapro.scope.camera.CameraEngineTest.FakeCameraSource
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import java.util.concurrent.atomic.AtomicInteger

class SessionManagerTest {

    private lateinit var fakeSource: FakeCameraSource
    private lateinit var cameraEngine: CameraEngine
    private lateinit var sessionManager: SessionManager

    @Before
    fun setUp() {
        fakeSource = FakeCameraSource()
        cameraEngine = CameraEngine(fakeSource)
        sessionManager = SessionManager(cameraEngine)
    }

    @Test
    fun `initial session state is disconnected`() {
        assertEquals(SessionState.DISCONNECTED, sessionManager.state)
        assertNull(sessionManager.activeLease)
    }

    @Test
    fun `acquire lease moves to ready state`() {
        val result = sessionManager.acquireLease(
            controllerId = "desktop-1",
            token = "secret-token-abc",
            ttlMs = 5000,
            nowMs = 1000
        )

        assertTrue(result.isSuccess)
        assertEquals(SessionState.READY, sessionManager.state)
        assertNotNull(sessionManager.activeLease)
        assertEquals("desktop-1", sessionManager.activeLease?.controllerId)
    }

    @Test
    fun `another controller cannot steal existing active lease`() {
        sessionManager.acquireLease("desktop-1", "token-1", 5000, 1000)

        val conflict = sessionManager.acquireLease("desktop-2", "token-2", 5000, 1500)
        assertTrue(conflict.isFailure)
        assertEquals("desktop-1", sessionManager.activeLease?.controllerId)
        assertEquals(SessionState.READY, sessionManager.state)
    }

    @Test
    fun `startStream with valid lease starts camera and transitions to streaming`() {
        sessionManager.acquireLease("desktop-1", "token-1", 5000, 1000)

        val receivedFrames = AtomicInteger(0)
        val result = sessionManager.startStream(
            controllerId = "desktop-1",
            token = "token-1",
            cameraId = "0",
            width = 1280,
            height = 720,
            fps = 30,
            nowMs = 1100
        ) {
            receivedFrames.incrementAndGet()
        }

        assertTrue(result.isSuccess)
        assertEquals(SessionState.STREAMING, sessionManager.state)
        assertEquals(CameraEngine.State.STREAMING, cameraEngine.state)

        fakeSource.emitFrame(byteArrayOf(1, 2, 3))
        assertEquals(1, receivedFrames.get())
    }

    @Test
    fun `startStream with invalid controller or token fails`() {
        sessionManager.acquireLease("desktop-1", "token-1", 5000, 1000)

        val result = sessionManager.startStream(
            controllerId = "desktop-1",
            token = "wrong-token",
            cameraId = "0",
            width = 1280,
            height = 720,
            fps = 30,
            nowMs = 1100
        ) {}

        assertTrue(result.isFailure)
        assertEquals(SessionState.READY, sessionManager.state)
        assertEquals(CameraEngine.State.IDLE, cameraEngine.state)
    }

    @Test
    fun `stopStream stops camera and transitions back to ready`() {
        sessionManager.acquireLease("desktop-1", "token-1", 5000, 1000)
        sessionManager.startStream("desktop-1", "token-1", "0", 1280, 720, 30, 1100) {}

        val result = sessionManager.stopStream("desktop-1", "token-1")
        assertTrue(result.isSuccess)
        assertEquals(SessionState.READY, sessionManager.state)
        assertEquals(CameraEngine.State.IDLE, cameraEngine.state)
    }

    @Test
    fun `watchdog tick stops stream when heartbeat expires`() {
        sessionManager.acquireLease("desktop-1", "token-1", ttlMs = 2000, nowMs = 1000)
        sessionManager.startStream("desktop-1", "token-1", "0", 1280, 720, 30, 1100) {}

        assertEquals(SessionState.STREAMING, sessionManager.state)

        // Tick before expiry
        sessionManager.watchdogCheck(nowMs = 2500)
        assertEquals(SessionState.STREAMING, sessionManager.state)

        // Tick after expiry (> 3000 ms)
        sessionManager.watchdogCheck(nowMs = 3001)
        assertEquals(SessionState.DISCONNECTED, sessionManager.state)
        assertEquals(CameraEngine.State.IDLE, cameraEngine.state)
        assertNull(sessionManager.activeLease)
    }

    @Test
    fun `reconnect returns ready and does not silently reactivate camera`() {
        sessionManager.acquireLease("desktop-1", "token-1", ttlMs = 2000, nowMs = 1000)
        sessionManager.startStream("desktop-1", "token-1", "0", 1280, 720, 30, 1100) {}

        // Release/disconnect
        sessionManager.releaseLease("desktop-1", "token-1")
        assertEquals(SessionState.DISCONNECTED, sessionManager.state)
        assertEquals(CameraEngine.State.IDLE, cameraEngine.state)

        // Reconnect: acquire new lease
        val reconnected = sessionManager.acquireLease("desktop-1", "token-new", ttlMs = 2000, nowMs = 5000)
        assertTrue(reconnected.isSuccess)
        // Must be READY, NEVER automatically STREAMING
        assertEquals(SessionState.READY, sessionManager.state)
        assertEquals(CameraEngine.State.IDLE, cameraEngine.state)
    }
}
