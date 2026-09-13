package app.camapro.scope.transport

import java.util.ArrayDeque

/**
 * Bounded queue for in-flight media frames.
 * Invariant D05 & D07: Max capacity 2 frames; drop oldest under slow consumers;
 * maximum bounded buffer size prevents unbounded memory growth.
 */
class BoundedFrameQueue(
    val maxCapacity: Int = 2,
    val maxFrameSizeBytes: Int = 4 * 1024 * 1024
) {
    private val queue = ArrayDeque<ByteArray>(maxCapacity)
    private val lock = Any()

    @Volatile
    var droppedFramesCount: Long = 0
        private set

    val size: Int
        get() = synchronized(lock) { queue.size }

    fun enqueue(frame: ByteArray): Boolean = synchronized(lock) {
        if (frame.size > maxFrameSizeBytes) {
            return false
        }

        while (queue.size >= maxCapacity) {
            queue.pollFirst()
            droppedFramesCount++
        }

        queue.addLast(frame)
        true
    }

    fun dequeue(): ByteArray? = synchronized(lock) {
        queue.pollFirst()
    }

    fun clear() = synchronized(lock) {
        queue.clear()
    }
}

/**
 * Formatter for multipart/x-mixed-replace MJPEG stream parts.
 */
object MjpegPartFormatter {
    private const val BOUNDARY = "--frame\r\n"

    fun formatPart(jpegData: ByteArray, timestampNanos: Long = System.nanoTime()): ByteArray {
        val headers = "$BOUNDARY" +
                "Content-Type: image/jpeg\r\n" +
                "Content-Length: ${jpegData.size}\r\n" +
                "X-Timestamp: $timestampNanos\r\n\r\n"
        val headerBytes = headers.toByteArray(Charsets.US_ASCII)
        val tailBytes = "\r\n".toByteArray(Charsets.US_ASCII)

        val result = ByteArray(headerBytes.size + jpegData.size + tailBytes.size)
        System.arraycopy(headerBytes, 0, result, 0, headerBytes.size)
        System.arraycopy(jpegData, 0, result, headerBytes.size, jpegData.size)
        System.arraycopy(tailBytes, 0, result, headerBytes.size + jpegData.size, tailBytes.size)
        return result
    }
}
