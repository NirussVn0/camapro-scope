package app.camapro.scope.transport

import org.junit.Assert.*
import org.junit.Before
import org.junit.Test

class MjpegTransportTest {

    private lateinit var queue: BoundedFrameQueue

    @Before
    fun setUp() {
        queue = BoundedFrameQueue(maxCapacity = 2, maxFrameSizeBytes = 1024 * 1024)
    }

    @Test
    fun `enqueue within capacity succeeds`() {
        val frame1 = byteArrayOf(1, 2, 3)
        assertTrue(queue.enqueue(frame1))
        assertEquals(1, queue.size)

        val frame2 = byteArrayOf(4, 5, 6)
        assertTrue(queue.enqueue(frame2))
        assertEquals(2, queue.size)
    }

    @Test
    fun `slow consumer drops oldest frame when capacity is exceeded`() {
        val frame1 = byteArrayOf(1)
        val frame2 = byteArrayOf(2)
        val frame3 = byteArrayOf(3)

        queue.enqueue(frame1)
        queue.enqueue(frame2)
        assertEquals(2, queue.size)
        assertEquals(0, queue.droppedFramesCount)

        // Enqueueing 3rd frame must drop oldest (frame1)
        queue.enqueue(frame3)
        assertEquals(2, queue.size)
        assertEquals(1, queue.droppedFramesCount)

        // Oldest (frame1) was dropped; next dequeued must be frame2, then frame3
        assertArrayEquals(frame2, queue.dequeue())
        assertArrayEquals(frame3, queue.dequeue())
        assertNull(queue.dequeue())
    }

    @Test
    fun `oversized frame is rejected fail-closed`() {
        val oversized = ByteArray(1024 * 1024 + 1)
        assertFalse(queue.enqueue(oversized))
        assertEquals(0, queue.size)
    }

    @Test
    fun `mjpeg part formatter produces valid multipart chunk`() {
        val jpeg = byteArrayOf(0xFF.toByte(), 0xD8.toByte(), 0xFF.toByte())
        val formatted = MjpegPartFormatter.formatPart(jpeg, timestampNanos = 123456789L)
        val headerString = String(formatted, Charsets.US_ASCII)

        assertTrue(headerString.contains("--frame\r\n"))
        assertTrue(headerString.contains("Content-Type: image/jpeg\r\n"))
        assertTrue(headerString.contains("Content-Length: 3\r\n"))
        assertTrue(headerString.contains("X-Timestamp: 123456789\r\n\r\n"))
    }
}
