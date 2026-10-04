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
    @Test
    fun `trickled incomplete TLS handshake headers and body hit absolute deadline`() {
        for (kind in listOf("headers", "TLS", "body")) {
            val tls = kind == "TLS"
            val factory = if (tls) javax.net.ssl.SSLContext.getInstance("TLSv1.3").apply { init(null, null, null) }.serverSocketFactory else null
            val server = MjpegHttpServer({ null }, "secret", requestedPort = 0, tlsFactory = factory)
            server.start()
            val sock = Socket("127.0.0.1", server.port)
            val pool = java.util.concurrent.Executors.newFixedThreadPool(2)
            try {
                val started = System.nanoTime()
                if (tls) sock.getOutputStream().write(byteArrayOf(22, 3, 3, 0, 100, 1, 0, 0, 96))
                else if (kind == "body") sock.getOutputStream().write("POST /control HTTP/1.1\r\nX-Camapro-Token: secret\r\nContent-Length: 100\r\n\r\n{".toByteArray())
                else sock.getOutputStream().write("GET /status HTTP/1.1\r\nX-Slow: ".toByteArray())
                pool.submit {
                    try {
                        while (!Thread.currentThread().isInterrupted) {
                            sock.getOutputStream().write(if (tls) 0 else 'a'.code)
                            sock.getOutputStream().flush()
                            Thread.sleep(200)
                        }
                    } catch (_: Exception) {}
                }
                val closed = pool.submit<Boolean> {
                    try {
                        // JSSE may send a TLS alert before EOF when the deadline closes it.
                        while (sock.getInputStream().read() != -1) {}
                        true
                    } catch (_: java.io.IOException) { true }
                }
                try {
                    assertTrue("absolute deadline must close trickled connection", closed.get(6500, java.util.concurrent.TimeUnit.MILLISECONDS))
                    assertTrue("test must trickle an incomplete handshake/request until deadline", System.nanoTime() - started >= 4_000_000_000L)
                } catch (_: java.util.concurrent.TimeoutException) {
                    fail("Trickle monopolized worker beyond absolute deadline ($kind)")
                }
            } finally { sock.close(); server.stop(); pool.shutdownNow() }
        }
    }

    @Test
    fun `authenticated stream readiness cancels deadline`() {
        val queue = BoundedFrameQueue()
        val server = startServer { queue.dequeue() }
        try {
            connect(server.port, "/stream", "secret").use { sock ->
                assertTrue(readHttpHeaders(sock.getInputStream()).startsWith("HTTP/1.1 200"))
                Thread.sleep(5500)
                queue.enqueue("after-deadline".toByteArray(US_ASCII))
                assertEquals(1, parseParts(sock.getInputStream(), 1).size)
            }
        } finally { server.stop() }
    }

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

    private fun post(port: Int, path: String, tokenHeader: String?, body: String, soTimeoutMs: Int = 5000): Socket {
        val sock = Socket("127.0.0.1", port)
        sock.soTimeout = soTimeoutMs
        val bodyBytes = body.toByteArray(Charsets.UTF_8)
        val req = StringBuilder("POST $path HTTP/1.1\r\nHost: 127.0.0.1\r\n")
        req.append("Content-Type: application/json\r\n")
        req.append("Content-Length: ${bodyBytes.size}\r\n")
        if (tokenHeader != null) req.append("X-Camapro-Token: $tokenHeader\r\n")
        req.append("\r\n")
        val out = sock.getOutputStream()
        out.write(req.toString().toByteArray(US_ASCII))
        out.write(bodyBytes)
        out.flush()
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
    fun `status and control stay available during media and second media is rejected`() {
        val queue = BoundedFrameQueue()
        queue.enqueue("a-frame".toByteArray(US_ASCII))
        val server = startServer { queue.dequeue() }
        try {
            val a = connect(server.port, "/stream", "secret")
            readHttpHeaders(a.getInputStream())
            val partsA = parseParts(a.getInputStream(), 1)
            assertEquals(1, partsA.size)
            assertArrayEquals("a-frame".toByteArray(US_ASCII), partsA[0])

            connect(server.port, "/status", "secret").use { status ->
                assertTrue(readHttpHeaders(status.getInputStream()).startsWith("HTTP/1.1 200"))
            }
            server.controlHandler = { "{\"ok\":true}" }
            post(server.port, "/control", "secret", "{}").use { control ->
                assertTrue(readHttpHeaders(control.getInputStream()).startsWith("HTTP/1.1 200"))
            }
            connect(server.port, "/stream", "secret").use { b ->
                assertTrue(readHttpHeaders(b.getInputStream()).startsWith("HTTP/1.1 409"))
            }
            a.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun `LAN cannot bind without TLS`() {
        val server = MjpegHttpServer({ null }, "secret", requestedPort = 0, bindAddress = "0.0.0.0")
        try {
            server.start()
            fail("Plaintext LAN binding must be rejected")
        } catch (_: IllegalStateException) {
        } finally { server.stop() }
    }

    @Test
    fun `stopped endpoint cannot be restarted by late pairing`() {
        val server = startServer { null }
        server.stop()
        try {
            server.start()
            fail("Late pairing must not reopen a stopped endpoint")
        } catch (_: IllegalStateException) {}
    }

    @Test
    fun `status and h264 reject URL-only authorization`() {
        val server = startServer { null }
        try {
            for (path in listOf("/status", "/stream.h264?token=secret")) {
                connect(server.port, path, null).use { sock ->
                    assertTrue(readHttpHeaders(sock.getInputStream()).startsWith("HTTP/1.1 401"))
                }
            }
        } finally { server.stop() }
    }

    @Test
    fun queryParameterTokenDoesNotAuthenticateStream() {
        val queue = BoundedFrameQueue()
        val server = startServer(token = "querysecret") { queue.dequeue() }
        try {
            // Connect with ?token=querysecret in the URL, without X-Camapro-Token header
            val sock = connect(server.port, "/stream?token=querysecret", tokenHeader = null)
            val header = readHttpHeaders(sock.getInputStream())
            assertTrue(header, header.startsWith("HTTP/1.1 401"))
            sock.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun statusEndpointReturns200OkWithJson() {
        val server = startServer(token = "statussecret") { null }
        try {
            val sock = connect(server.port, "/status", tokenHeader = "statussecret")
            val header = readHttpHeaders(sock.getInputStream())
            assertTrue(header, header.startsWith("HTTP/1.1 200 OK"))
            assertTrue(header, header.contains("X-Camapro-Ready: true"))
            assertTrue(header, header.contains("application/json"))
            assertTrue(header, header.contains("Access-Control-Allow-Origin: *"))
            sock.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun controlEndpointWithoutTokenGets401() {
        val server = startServer(token = "ctrlsecret") { null }
        try {
            val sock = post(server.port, "/control", tokenHeader = null, body = "{}")
            val resp = readHttpHeaders(sock.getInputStream())
            assertTrue(resp, resp.startsWith("HTTP/1.1 401"))
            sock.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun controlEndpointWithWrongTokenGets401() {
        val server = startServer(token = "ctrlsecret") { null }
        try {
            val sock = post(server.port, "/control", tokenHeader = "wrong", body = "{}")
            val resp = readHttpHeaders(sock.getInputStream())
            assertTrue(resp, resp.startsWith("HTTP/1.1 401"))
            sock.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun controlEndpointWithValidTokenExecutesHandlerAndReturnsJson() {
        val server = startServer(token = "ctrlsecret") { null }
        server.controlHandler = { reqBody ->
            """{"v":1,"id":42,"type":"response","ok":true,"result":{"applied":true}}"""
        }
        try {
            val sock = post(server.port, "/control", tokenHeader = "ctrlsecret", body = """{"v":1,"id":42,"type":"camera.set"}""")
            val headers = readHttpHeaders(sock.getInputStream())
            assertTrue(headers, headers.startsWith("HTTP/1.1 200 OK"))
            assertTrue(headers, headers.contains("Content-Type: application/json"))
            val inStream = sock.getInputStream()
            val buf = ByteArray(1024)
            val read = inStream.read(buf)
            val body = String(buf, 0, read, Charsets.UTF_8)
            assertTrue(body, body.contains(""""ok":true"""))
            sock.close()
        } finally {
            server.stop()
        }
    }

    @Test
    fun h264StreamEndpointStreamsRawBytes() {
        val nal = byteArrayOf(0, 0, 0, 1, 0x67, 0x42, 0x00, 0x1f)
        val server = startServer(token = "h264secret") { null }
        var delivered = false
        server.h264Supplier = {
            if (!delivered) {
                delivered = true
                nal
            } else {
                null
            }
        }
        try {
            val sock = connect(server.port, "/stream.h264", "h264secret")
            val headers = readHttpHeaders(sock.getInputStream())
            assertTrue(headers, headers.startsWith("HTTP/1.1 200 OK"))
            assertTrue(headers, headers.contains("Content-Type: video/x-h264"))
            val buf = ByteArray(8)
            val read = sock.getInputStream().read(buf)
            assertEquals(8, read)
            assertArrayEquals(nal, buf)
            sock.close()
        } finally {
            server.stop()
        }
    }
}
