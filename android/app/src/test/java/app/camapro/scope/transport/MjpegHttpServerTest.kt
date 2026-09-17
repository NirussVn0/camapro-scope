package app.camapro.scope.transport

import org.junit.Assert.*
import org.junit.Test
import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.net.ConnectException
import java.net.Socket
import java.net.SocketTimeoutException
import java.nio.charset.StandardCharsets.US_ASCII

/**
 * JVM unit tests for MjpegHttpServer against a real socket on 127.0.0.1
 * (ephemeral port). No Robolectric, no instrumentation.
 */
class MjpegHttpServerTest {

    private fun startServer(token: String = "secret", supplier: () -> ByteArray?): MjpegHttpServer {
        val server = MjpegHttpServer(frameSupplier = supplier, token = token, requestedPort = 0)
        server.start()
        return server
    }

    private fun connect(port: Int, path: String, tokenHeader: String?, soTimeoutMs: Int = 5000): Socket {
        val sock = Socket("127.0.0.1", port)
        sock.soTimeout = soTimeoutMs
        val req = StringBuilder("GET $path HTTP/1.1\r\nHost: 127.0.0.1\r\n")
        if (tokenHeader != null) req.append("X-Camapro-Token: $tokenHeader\r\n")
        req.append("\r\n")
        sock.getOutputStream().write(req.toString().toByteArray(US_ASCII))
        sock.getOutputStream().flush()
        return sock
    }

    /** Reads until the terminating CRLF CRLF of an HTTP header block (max 4KB). */
    private fun readHttpHeaders(input: InputStream): String {
        val out = ByteArrayOutputStream()
        var n = 0
        while (n < 4096 && !out.toString("US-ASCII").endsWith("\r\n\r\n")) {
            val b = input.read()
            if (b == -1) break
            out.write(b)
            n++
        }
        return out.toString("US-ASCII")
    }

    /**
     * Parses complete multipart parts from the body stream. Each returned
     * element's size is verified against its own Content-Length, and the two
     * bytes after the body must be CRLF.
     */
    private fun parseParts(input: InputStream, maxParts: Int): List<ByteArray> {
        val parts = mutableListOf<ByteArray>()
        var buf = ByteArray(0)
        while (parts.size < maxParts) {
            var text = String(buf, US_ASCII)
            var idx = text.indexOf("--frame\r\n")
            while (idx < 0) {
                val b = input.read()
                if (b == -1) return parts
                buf += b.toByte()
                text = String(buf, US_ASCII)
                idx = text.indexOf("--frame\r\n")
            }
            var headerEnd = text.indexOf("\r\n\r\n", idx)
            while (headerEnd < 0) {
                val b = input.read()
                if (b == -1) return parts
                buf += b.toByte()
                text = String(buf, US_ASCII)
                headerEnd = text.indexOf("\r\n\r\n", idx)
            }
            val headers = text.substring(idx + "--frame\r\n".length, headerEnd)
            val lenLine = headers.lines().firstOrNull { it.startsWith("Content-Length:") }
            assertNotNull("part must carry Content-Length, headers were: $headers", lenLine)
            val contentLength = lenLine!!.removePrefix("Content-Length:").trim().toInt()
            val need = headerEnd + 4 + contentLength + 2
            while (buf.size < need) {
                val b = input.read()
                if (b == -1) return parts
                buf += b.toByte()
            }
            val bodyStart = headerEnd + 4
            val body = buf.copyOfRange(bodyStart, bodyStart + contentLength)
            assertEquals("body must be followed by CRLF", "\r\n",
                String(buf.copyOfRange(bodyStart + contentLength, bodyStart + contentLength + 2), US_ASCII))
            parts.add(body)
            buf = buf.copyOfRange(need, buf.size)
        }
        return parts
    }

    @Test
    fun `missing token gets 401`() {
        val server = startServer { null }
        try {
            val sock = connect(server.port, "/stream", null, soTimeoutMs = 2000)
            val resp = readHttpHeaders(sock.getInputStream())
            assertTrue(resp, resp.startsWith("HTTP/1.1 401"))
            sock.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun `wrong token gets 401`() {
        val server = startServer { null }
        try {
            val sock = connect(server.port, "/stream", "nope", soTimeoutMs = 2000)
            val resp = readHttpHeaders(sock.getInputStream())
            assertTrue(resp, resp.startsWith("HTTP/1.1 401"))
            sock.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun `unknown path gets 404 even with valid token`() {
        val server = startServer { null }
        try {
            val sock = connect(server.port, "/", "secret", soTimeoutMs = 2000)
            val resp = readHttpHeaders(sock.getInputStream())
            assertTrue(resp, resp.startsWith("HTTP/1.1 404"))
            sock.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun `valid request streams parts with boundary and matching content length`() {
        val queue = BoundedFrameQueue()
        queue.enqueue("frame-one".toByteArray(US_ASCII))
        queue.enqueue("frame-two-payload".toByteArray(US_ASCII))
        queue.enqueue("frame-3".toByteArray(US_ASCII)) // cap 2 drops oldest -> 2 frames remain
        assertEquals(1, queue.droppedFramesCount)

        val server = startServer { queue.dequeue() }
        try {
            val sock = connect(server.port, "/stream", "secret")
            val header = readHttpHeaders(sock.getInputStream())
            assertTrue(header, header.startsWith("HTTP/1.1 200"))
            assertTrue(header, header.contains("Content-Type: multipart/x-mixed-replace; boundary=frame"))
            val parts = parseParts(sock.getInputStream(), 2)
            assertEquals(2, parts.size)
            assertArrayEquals("frame-two-payload".toByteArray(US_ASCII), parts[0])
            assertArrayEquals("frame-3".toByteArray(US_ASCII), parts[1])
            sock.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun `stop unblocks accept loop, closes socket, is idempotent`() {
        val server = startServer { null }
        val port = server.port
        server.stop()

        val deadline = System.currentTimeMillis() + 2000
        while (server.isAlive && System.currentTimeMillis() < deadline) {
            Thread.sleep(20)
        }
        assertFalse("accept thread did not exit within 2s", server.isAlive)

        var refused = false
        try {
            Socket("127.0.0.1", port).close()
        } catch (e: ConnectException) {
            refused = true
        }
        assertTrue("server socket still accepting after stop()", refused)

        server.stop() // idempotent second call must not throw
    }

    @Test
    fun `second client is served sequentially after first disconnects`() {
        val queue = BoundedFrameQueue()
        queue.enqueue("a-frame".toByteArray(US_ASCII))
        val server = startServer { queue.dequeue() }
        try {
            val a = connect(server.port, "/stream", "secret")
            readHttpHeaders(a.getInputStream())
            val partsA = parseParts(a.getInputStream(), 1)
            assertEquals(1, partsA.size)
            assertArrayEquals("a-frame".toByteArray(US_ASCII), partsA[0])

            // B connects but must NOT be served while A holds the single writer.
            val b = connect(server.port, "/stream", "secret", soTimeoutMs = 600)
            var bServedEarly = false
            try {
                b.getInputStream().read()
                bServedEarly = true
            } catch (e: SocketTimeoutException) {
                // expected: nothing accepted yet
            }
            assertFalse("server must serve sequentially, not concurrently", bServedEarly)
            b.soTimeout = 5000

            // A hangs up -> writer loop observes EOF, accept loop takes B.
            a.close()
            val headerB = readHttpHeaders(b.getInputStream())
            assertTrue(headerB, headerB.startsWith("HTTP/1.1 200"))

            queue.enqueue("b-frame".toByteArray(US_ASCII))
            val partsB = parseParts(b.getInputStream(), 1)
            assertEquals(1, partsB.size)
            assertArrayEquals("b-frame".toByteArray(US_ASCII), partsB[0])
            b.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun queryParameterTokenAuthenticatesStream() {
        val queue = BoundedFrameQueue()
        val server = startServer(token = "querysecret") { queue.dequeue() }
        try {
            // Connect with ?token=querysecret in the URL, without X-Camapro-Token header
            val sock = connect(server.port, "/stream?token=querysecret", tokenHeader = null)
            val header = readHttpHeaders(sock.getInputStream())
            assertTrue(header, header.startsWith("HTTP/1.1 200"))
            assertTrue(header, header.contains("multipart/x-mixed-replace"))

            queue.enqueue("query-frame".toByteArray(US_ASCII))
            val parts = parseParts(sock.getInputStream(), 1)
            assertEquals(1, parts.size)
            assertArrayEquals("query-frame".toByteArray(US_ASCII), parts[0])
            sock.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun statusEndpointReturns200OkWithJson() {
        val server = startServer(token = "statussecret") { null }
        try {
            val sock = connect(server.port, "/status", tokenHeader = null)
            val header = readHttpHeaders(sock.getInputStream())
            assertTrue(header, header.startsWith("HTTP/1.1 200 OK"))
            assertTrue(header, header.contains("application/json"))
            assertTrue(header, header.contains("Access-Control-Allow-Origin: *"))
            sock.close()
        } finally {
            server.stop()
        }
    }
}

