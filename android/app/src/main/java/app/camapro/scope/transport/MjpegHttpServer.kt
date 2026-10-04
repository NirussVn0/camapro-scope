package app.camapro.scope.transport

import java.io.IOException
import java.io.InputStream
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.nio.charset.StandardCharsets.US_ASCII

/**
 * Minimal MJPEG-over-HTTP server (stdlib java.net only, no new deps).
 *
 * Serves exactly `GET /stream` with `multipart/x-mixed-replace; boundary=frame`,
 * pulling frames from [frameSupplier] (delegates to [BoundedFrameQueue]).
 * Requests without a matching `X-Camapro-Token` header get 401; any other
 * path gets 404.
 *
 * One media writer plus concurrent control/status; a bounded executor owns sockets.
 * LAN requires mutual TLS; plaintext is available only on loopback for JVM tests.
 */
class MjpegHttpServer(
    private val frameSupplier: () -> ByteArray?,
    private val token: String,
    requestedPort: Int = DEFAULT_PORT,
    private val bindAddress: String = "127.0.0.1",
    var controlHandler: ((String) -> String)? = null,
    var h264Supplier: (() -> ByteArray?)? = null,
    private var tlsFactory: javax.net.ssl.SSLServerSocketFactory? = null
) {
    private val bindPort = requestedPort
    companion object {
        const val DEFAULT_PORT = 8100
        const val TOKEN_HEADER = "X-Camapro-Token"
        private const val IDLE_POLL_MS = 50
    }

    @Volatile
    private var running = false
    private var terminated = false
    @Volatile private var enrollmentOnly = false

    @Volatile
    private var serverSocket: ServerSocket? = null

    private val clients = java.util.concurrent.ConcurrentHashMap.newKeySet<Socket>()
    private val mediaWriter = java.util.concurrent.Semaphore(1)
    private var workers: java.util.concurrent.ThreadPoolExecutor? = null
    private var deadlines: java.util.concurrent.ScheduledThreadPoolExecutor? = null

    private var thread: Thread? = null

    /** Actual bound port (differs from requested value when 0 was passed). */
    val port: Int
        get() = serverSocket?.localPort ?: bindPort

    /** True while the accept loop thread is alive. */
    val isAlive: Boolean
        get() = thread?.isAlive == true

    @Synchronized
    fun start() {
        check(!terminated) { "Stopped endpoint cannot be restarted" }
        if (running) return
        val factory = tlsFactory
        check(factory != null || InetAddress.getByName(bindAddress).isLoopbackAddress) { "LAN requires pinned TLS" }
        val ss = if (factory != null) {
            (factory.createServerSocket(bindPort, 8, InetAddress.getByName(bindAddress)) as javax.net.ssl.SSLServerSocket).apply {
                enabledProtocols = arrayOf("TLSv1.3")
                needClientAuth = true
            }
        } else ServerSocket(bindPort, 8, InetAddress.getByName(bindAddress))
        serverSocket = ss
        deadlines = java.util.concurrent.ScheduledThreadPoolExecutor(1, java.util.concurrent.ThreadFactory { r ->
            Thread(r, "mjpeg-request-deadline").apply { isDaemon = true }
        }).apply { removeOnCancelPolicy = true }
        workers = java.util.concurrent.ThreadPoolExecutor(2, 2, 0L, java.util.concurrent.TimeUnit.MILLISECONDS,
            java.util.concurrent.ArrayBlockingQueue(8), java.util.concurrent.ThreadFactory { r ->
                Thread(r, "mjpeg-tls-client").apply { isDaemon = true }
            })
        running = true
        thread = Thread({ acceptLoop(ss) }, "mjpeg-http-accept").apply {
            isDaemon = true
            start()
        }
    }

    /** Closes the listening socket and any in-flight client; idempotent. */
    @Synchronized
    fun stop() {
        terminated = true
        closeSockets()
    }

    private fun closeSockets() {
        running = false
        try {
            serverSocket?.close()
        } catch (_: IOException) {
        }
        serverSocket = null
        disconnectClients()
        workers?.shutdownNow()
        workers = null
        deadlines?.shutdownNow()
        deadlines = null
    }

    fun disconnectClients() {
        clients.forEach { try { it.close() } catch (_: IOException) {} }
        clients.clear()
    }

    /** New trust generation gets a fresh TLS context, including its session cache. */
    @Synchronized
    fun replaceTlsFactory(factory: javax.net.ssl.SSLServerSocketFactory, provisional: Boolean = false) {
        check(!terminated && running) { "Endpoint was stopped during pairing" }
        closeSockets()
        thread?.join(3500)
        tlsFactory = factory
        enrollmentOnly = provisional
        start()
    }

    private fun acceptLoop(ss: ServerSocket) {
        while (running && !ss.isClosed) {
            try {
                val sock = ss.accept()
                sock.soTimeout = 3000
                clients.add(sock)
                var deadline: java.util.concurrent.ScheduledFuture<*>? = null
                try {
                    val requestDeadline = deadlines?.schedule({ try { sock.close() } catch (_: IOException) {} }, 5000, java.util.concurrent.TimeUnit.MILLISECONDS)
                        ?: throw java.util.concurrent.RejectedExecutionException()
                    deadline = requestDeadline
                    val executor = workers ?: throw java.util.concurrent.RejectedExecutionException()
                    executor.execute {
                        try { handleClient(sock, requestDeadline) } catch (_: IOException) {
                            // TLS rejection, bounded read timeout, or disconnect.
                        } finally {
                            requestDeadline.cancel(false)
                            try { sock.close() } catch (_: IOException) {}
                            clients.remove(sock)
                        }
                    }
                } catch (_: java.util.concurrent.RejectedExecutionException) {
                    deadline?.cancel(false)
                    sock.close()
                    clients.remove(sock)
                }
            } catch (_: IOException) {
                // stop() closed the socket, or a transient accept error.
                if (!running || ss.isClosed) break
            }
        }
    }

    private fun handleClient(sock: Socket, deadline: java.util.concurrent.ScheduledFuture<*>) {
        if (sock is javax.net.ssl.SSLSocket) sock.startHandshake()
        handleRequest(sock, deadline)
    }

    private fun handleRequest(sock: Socket, deadline: java.util.concurrent.ScheduledFuture<*>) {
        val input = sock.getInputStream()
        val headerText = readHeaders(input) ?: return
        val lines = headerText.split("\r\n")
        val requestLine = lines.firstOrNull()?.split(" ") ?: return
        val method = requestLine.getOrNull(0)
        val rawUri = requestLine.getOrNull(1) ?: ""
        val path = rawUri.substringBefore('?')
        if (enrollmentOnly && (method != "GET" || path != "/status")) {
            respondSimple(sock, input, "403 Forbidden")
            return
        }

        if (method == "GET" && path == "/status") {
            if (headersValue(lines, TOKEN_HEADER) != token) {
                respondSimple(sock, input, "401 Unauthorized")
                return
            }
            val json = "{\"status\":\"ok\",\"service\":\"camapro-scope\",\"port\":$port}\r\n"
            val body = json.toByteArray(US_ASCII)
            val res = "HTTP/1.1 200 OK\r\n" +
                    "Content-Type: application/json\r\n" +
                    "X-Camapro-Ready: ${!enrollmentOnly}\r\n" +
                    "Access-Control-Allow-Origin: *\r\n" +
                    "Content-Length: ${body.size}\r\n" +
                    "Connection: close\r\n\r\n"
            val out = sock.getOutputStream()
            out.write(res.toByteArray(US_ASCII))
            out.write(body)
            out.flush()
            return
        }

        if (method == "POST" && path == "/control") {
            val requestToken = headersValue(lines, TOKEN_HEADER)
            if (requestToken != token) {
                val err = "{\"v\":1,\"type\":\"error\",\"ok\":false,\"error\":{\"code\":\"unauthenticated\",\"message\":\"Invalid or missing token\",\"retryable\":false}}\r\n"
                respondJson(sock, 401, "Unauthorized", err)
                return
            }

            val contentLength = headersValue(lines, "Content-Length")?.toIntOrNull() ?: 0
            if (contentLength <= 0 || contentLength > 65536) {
                val err = "{\"v\":1,\"type\":\"error\",\"ok\":false,\"error\":{\"code\":\"invalid_payload\",\"message\":\"Invalid Content-Length\",\"retryable\":false}}\r\n"
                respondJson(sock, 400, "Bad Request", err)
                return
            }

            val bodyBytes = ByteArray(contentLength)
            var readBytes = 0
            while (readBytes < contentLength) {
                val r = input.read(bodyBytes, readBytes, contentLength - readBytes)
                if (r == -1) break
                readBytes += r
            }
            if (readBytes != contentLength) return
            val body = String(bodyBytes, 0, readBytes, Charsets.UTF_8)
            val handler = controlHandler
            if (handler == null) {
                val err = "{\"v\":1,\"type\":\"error\",\"ok\":false,\"error\":{\"code\":\"internal\",\"message\":\"No control handler configured\",\"retryable\":false}}\r\n"
                respondJson(sock, 500, "Internal Server Error", err)
                return
            }

            val resJson = handler(body)
            respondJson(sock, 200, "OK", resJson)
            return
        }

        if (method == "GET" && path == "/stream.h264") {
            val requestToken = headersValue(lines, TOKEN_HEADER)

            if (requestToken != token) {
                respondSimple(sock, input, "401 Unauthorized")
                return
            }
            if (!mediaWriter.tryAcquire()) {
                respondSimple(sock, input, "409 Conflict")
                return
            }
            try {

            val response = "HTTP/1.1 200 OK\r\n" +
                    "Content-Type: video/x-h264\r\n" +
                    "Cache-Control: no-cache, private\r\n" +
                    "Connection: close\r\n\r\n"
            val out = sock.getOutputStream()
            out.write(response.toByteArray(US_ASCII))
            out.flush()
            deadline.cancel(false) // Authenticated media readiness ends the request budget.

            val supplier = h264Supplier ?: frameSupplier
            while (running) {
                val frame = supplier()
                if (frame == null) {
                    sock.soTimeout = IDLE_POLL_MS
                    try {
                        if (input.read() == -1) return
                    } catch (_: java.net.SocketTimeoutException) {
                    }
                    continue
                }
                try {
                    out.write(frame)
                    out.flush()
                } catch (_: IOException) {
                    return
                }
            }
            } finally { mediaWriter.release() }
            return
        }

        if (method != "GET" || path != "/stream") {
            respondSimple(sock, input, "404 Not Found")
            return
        }

        val requestToken = headersValue(lines, TOKEN_HEADER)

        if (requestToken != token) {
            respondSimple(sock, input, "401 Unauthorized")
            return
        }
        if (!mediaWriter.tryAcquire()) {
            respondSimple(sock, input, "409 Conflict")
            return
        }
        try {

        val response = "HTTP/1.1 200 OK\r\n" +
                "Content-Type: multipart/x-mixed-replace; boundary=frame\r\n" +
                "Cache-Control: no-cache, private\r\n" +
                "Connection: close\r\n\r\n"
        val out = sock.getOutputStream()
        out.write(response.toByteArray(US_ASCII))
        out.flush()
        deadline.cancel(false)

        while (running) {
            val frame = frameSupplier()
            if (frame == null) {
                // Idle: poll for client disconnect. EOF/-1 or any IOException
                // on the read means the browser is gone -> stop writing.
                sock.soTimeout = IDLE_POLL_MS
                try {
                    if (input.read() == -1) return
                } catch (_: java.net.SocketTimeoutException) {
                    // still connected, keep waiting for a frame
                }
                continue
            }
            try {
                out.write(MjpegPartFormatter.formatPart(frame))
                out.flush()
            } catch (_: IOException) {
                return // client disconnect: stop writer, no crash
            }
        }
        } finally { mediaWriter.release() }
    }

    private fun readHeaders(input: InputStream): String? {
        val buf = ByteArrayOutputStream8k()
        return try {
            while (!buf.endsWithDoubleCrlf()) {
                val b = input.read()
                if (b == -1 || buf.full()) return null
                buf.write(b)
            }
            buf.text()
        } catch (_: IOException) {
            null
        }
    }

    private fun headersValue(lines: List<String>, name: String): String? {
        val prefix = name + ":"
        return lines.firstOrNull { it.startsWith(prefix, ignoreCase = true) }
            ?.substringAfter(':')?.trim()
    }

    private fun respondSimple(sock: Socket, input: InputStream, status: String) {
        val body = status.toByteArray(US_ASCII)
        val resp = "HTTP/1.1 $status\r\n" +
                "Content-Type: text/plain; charset=us-ascii\r\n" +
                "Content-Length: ${body.size}\r\n" +
                "Connection: close\r\n\r\n"
        try {
            val out = sock.getOutputStream()
            out.write(resp.toByteArray(US_ASCII))
            out.write(body)
            out.flush()
        } catch (_: IOException) {
        }
    }

    private fun respondJson(sock: Socket, statusCode: Int, statusText: String, json: String) {
        val body = json.toByteArray(Charsets.UTF_8)
        val resp = "HTTP/1.1 $statusCode $statusText\r\n" +
                "Content-Type: application/json; charset=utf-8\r\n" +
                "Access-Control-Allow-Origin: *\r\n" +
                "Content-Length: ${body.size}\r\n" +
                "Connection: close\r\n\r\n"
        try {
            val out = sock.getOutputStream()
            out.write(resp.toByteArray(US_ASCII))
            out.write(body)
            out.flush()
        } catch (_: IOException) {
        }
    }

    /** Tiny fixed-cap header buffer (request headers are tiny; 8KB is generous). */
    private class ByteArrayOutputStream8k {
        private val bytes = ArrayList<Byte>(256)
        fun write(b: Int) {
            bytes.add(b.toByte())
        }

        fun full(): Boolean = bytes.size >= 8192

        fun endsWithDoubleCrlf(): Boolean {
            val n = bytes.size
            return n >= 4 && bytes[n - 4] == '\r'.code.toByte() && bytes[n - 3] == '\n'.code.toByte() &&
                    bytes[n - 2] == '\r'.code.toByte() && bytes[n - 1] == '\n'.code.toByte()
        }

        fun text(): String {
            val arr = ByteArray(bytes.size)
            for (i in bytes.indices) arr[i] = bytes[i]
            return String(arr, US_ASCII)
        }
    }
}
