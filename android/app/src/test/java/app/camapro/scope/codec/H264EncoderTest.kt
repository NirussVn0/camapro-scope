package app.camapro.scope.codec

import org.junit.Assert.*
import org.junit.Test

class H264EncoderTest {

    @Test
    fun `isAnnexB correctly detects 4-byte start codes`() {
        val validAnnexB = byteArrayOf(0, 0, 0, 1, 0x67, 0x42, 0x00, 0x1f)
        assertTrue(H264Encoder.isAnnexB(validAnnexB))

        val invalidAnnexB = byteArrayOf(0, 0, 1, 0x67, 0x42, 0x00, 0x1f)
        assertFalse(H264Encoder.isAnnexB(invalidAnnexB))

        val empty = byteArrayOf()
        assertFalse(H264Encoder.isAnnexB(empty))

        val short = byteArrayOf(0, 0, 0)
        assertFalse(H264Encoder.isAnnexB(short))
    }

    @Test
    fun `encoder initializes with declared parameters`() {
        val encoder = H264Encoder(
            width = 1280,
            height = 720,
            fps = 60,
            bitrate = 2_500_000
        ) {}

        assertEquals(1280, encoder.width)
        assertEquals(720, encoder.height)
        assertEquals(60, encoder.fps)
        assertEquals(2_500_000, encoder.bitrate)
        assertFalse(encoder.isRunning)
        assertNull(encoder.inputSurface)
    }

    @Test
    fun `stop is clean and idempotent when never started`() {
        val encoder = H264Encoder(1920, 1080, 30, 4_000_000) {}
        encoder.stop()
        assertFalse(encoder.isRunning)
        assertNull(encoder.inputSurface)

        // Second stop should be safe
        encoder.stop()
        assertFalse(encoder.isRunning)
    }
}
