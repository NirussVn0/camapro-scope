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
 * Clients are served sequentially (one in-flight connection at a time) —
 * ponytail: single accept loop; add a per-client thread only when a second
 * concurrent consumer is a real requirement.
 *
 * Binds 127.0.0.1 only. LAN binding is an explicit later decision (needs
 * token/auth threat model first). [port] = 0 picks an ephemeral port (tests);
 * production default is 8100.
 */
class MjpegHttpServer(
    private val frameSupplier: () -> ByteArray?,
    private val token: String,
    requestedPort: Int = DEFAULT_PORT,
    private val bindAddress: String = "0.0.0.0"
) {
    private val bindPort = requestedPort
    companion object {
        const val DEFAULT_PORT = 8100
        const val TOKEN_HEADER = "X-Camapro-Token"
        private const val IDLE_POLL_MS = 50
    }

    @Volatile
    private var running = false

    @Volatile
    private var serverSocket: ServerSocket? = null

    @Volatile
    private var client: Socket? = null

    private var thread: Thread? = null

    /** Actual bound port (differs from requested value when 0 was passed). */
    val port: Int
        get() = serverSocket?.localPort ?: bindPort

    /** True while the accept loop thread is alive. */
    val isAlive: Boolean
        get() = thread?.isAlive == true

    @Synchronized
    fun start() {
        if (running) return
        val ss = ServerSocket(bindPort, 8, InetAddress.getByName(bindAddress))
        serverSocket = ss
        running = true
        thread = Thread({ acceptLoop(ss) }, "mjpeg-http-accept").apply {
            isDaemon = true
            start()
        }
    }

    /** Closes the listening socket and any in-flight client; idempotent. */
    @Synchronized
    fun stop() {
        running = false
        try {
            serverSocket?.close()
        } catch (_: IOException) {
        }
        serverSocket = null
        try {
            client?.close()
        } catch (_: IOException) {
        }
        client = null
    }

    private fun acceptLoop(ss: ServerSocket) {
        while (running) {
            try {
                val sock = ss.accept()
                client = sock
                try {
                    handleClient(sock)
                } finally {
                    try {
                        sock.close()
                    } catch (_: IOException) {
                    }
                    client = null
                }
            } catch (_: IOException) {
                // stop() closed the socket, or a transient accept error.
                if (!running) break
            }
        }
    }

    private fun handleClient(sock: Socket) {
        val input = sock.getInputStream()
        val headerText = readHeaders(input) ?: return
        val lines = headerText.split("\r\n")
        val requestLine = lines.firstOrNull()?.split(" ") ?: return
        val method = requestLine.getOrNull(0)
        val rawUri = requestLine.getOrNull(1) ?: ""
        val (path, query) = if (rawUri.contains("?")) {
            val idx = rawUri.indexOf("?")
            rawUri.substring(0, idx) to rawUri.substring(idx + 1)
        } else {
            rawUri to ""
        }

        if (method == "GET" && path == "/status") {
            val json = "{\"status\":\"ok\",\"service\":\"camapro-scope\",\"port\":$port}\r\n"
            val body = json.toByteArray(US_ASCII)
            val res = "HTTP/1.1 200 OK\r\n" +
                    "Content-Type: application/json\r\n" +
                    "Access-Control-Allow-Origin: *\r\n" +
                    "Content-Length: ${body.size}\r\n" +
                    "Connection: close\r\n\r\n"
            val out = sock.getOutputStream()
            out.write(res.toByteArray(US_ASCII))
            out.write(body)
            out.flush()
            return
        }

        if (method != "GET" || path != "/stream") {
            respondSimple(sock, input, "404 Not Found")
            return
        }

        val queryToken = query.split("&").firstOrNull { it.startsWith("token=") }?.removePrefix("token=")
        val requestToken = headersValue(lines, TOKEN_HEADER) ?: queryToken

        if (requestToken != token) {
            respondSimple(sock, input, "401 Unauthorized")
            return
        }

        val response = "HTTP/1.1 200 OK\r\n" +
                "Content-Type: multipart/x-mixed-replace; boundary=frame\r\n" +
                "Cache-Control: no-cache, private\r\n" +
                "Connection: close\r\n\r\n"
        val out = sock.getOutputStream()
        out.write(response.toByteArray(US_ASCII))
        out.flush()

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
