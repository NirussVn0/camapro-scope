package app.camapro.scope

import android.Manifest
import android.app.AlertDialog
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.res.ColorStateList
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.SpannableStringBuilder
import android.text.Spanned
import android.text.style.ForegroundColorSpan
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.RadioButton
import android.widget.RadioGroup
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat
import app.camapro.scope.camera.Camera2Source
import app.camapro.scope.network.NetworkHelper
import app.camapro.scope.service.CameraStreamService
import app.camapro.scope.transport.BoundedFrameQueue
import app.camapro.scope.transport.MjpegHttpServer
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit
import kotlin.random.Random

/**
 * Main Mobile Controller for Camapro Scope:
 * - Real Camera2 hardware acquisition via Camera2Source (1080p ISP JPEG).
 * - Graceful fallback to synthetic frame generator if camera unavailable or denied.
 * - Multi-network binding (Wi-Fi LAN, Tailscale VPN, ADB Loopback) via MjpegHttpServer.
 * - Foreground Service (CameraStreamService) to keep stream alive when screen is locked.
 * - Instant desktop QR pairing via QrScanActivity and stream QR sharing via ZXing.
 * - Minimalist black AMOLED UI with collapsible bottom sheet.
 */
class CameraActivity : ComponentActivity() {

    private val queue = BoundedFrameQueue()
    private var server: MjpegHttpServer? = null
    private var cameraSource: Camera2Source? = null
    private var scheduler: ScheduledExecutorService? = null
    private var frameTask: ScheduledFuture<*>? = null
    private var isUsingRealCamera = false
    private var autoStartOnPermission = false

    private var token: String = Random.nextLong(0x10000000, 0xFFFFFFF0).toString(16)
    private val mainHandler = Handler(Looper.getMainLooper())
    private var displayMode: ScopeUi.DisplayMode = ScopeUi.DEFAULT_DISPLAY_MODE

    private lateinit var centerStatus: TextView
    private lateinit var centerHint: TextView
    private lateinit var sheet: View
    private lateinit var sheetStatus: TextView
    private lateinit var tokenRow: TextView
    private lateinit var endpointsContainer: LinearLayout
    private lateinit var previewCaption: TextView
    private lateinit var startButton: Button
    private lateinit var stopButton: Button

    private val cameraPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) {
            Toast.makeText(this, "Camera permission granted", Toast.LENGTH_SHORT).show()
            if (autoStartOnPermission) {
                startStreamingInternal(preferRealCamera = true)
            } else {
                bind(null)
            }
        } else {
            Toast.makeText(this, "Camera permission denied; synthetic feed will be used", Toast.LENGTH_LONG).show()
            if (autoStartOnPermission) {
                startStreamingInternal(preferRealCamera = false)
            } else {
                bind(null)
            }
        }
        autoStartOnPermission = false
    }

    private val qrScanLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == RESULT_OK) {
            val payload = result.data?.getStringExtra(QrScanActivity.EXTRA_QR_PAYLOAD)
            if (payload != null) {
                handleScannedDesktopPayload(payload)
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val root = FrameLayout(this).apply { setBackgroundColor(Color.BLACK) }

        // Centered identity block: app name + dot/word status + hint.
        centerStatus = TextView(this).apply { textSize = 15f; setTextColor(Color.WHITE) }
        centerHint = TextView(this).apply {
            textSize = 13f
            setTextColor(Color.parseColor("#94A3B8"))
            gravity = Gravity.CENTER
            setPadding(0, dp(6), 0, 0)
        }

        root.addView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(dp(24), dp(24), dp(24), dp(24))
            addView(TextView(this@CameraActivity).apply {
                text = "CamaPro Scope"
                textSize = 24f
                setTextColor(Color.parseColor("#F8FAFC"))
                gravity = Gravity.CENTER
                setTypeface(typeface, Typeface.BOLD)
                setPadding(0, 0, 0, dp(12))
            })
            addView(centerStatus.apply { gravity = Gravity.CENTER })
            addView(centerHint)
        }, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT
        ))

        startButton = button("Start", "#3B82F6").apply { setOnClickListener { startStreaming() } }
        stopButton = button("Stop", "#1F2937").apply { setOnClickListener { stopStreaming() } }

        tokenRow = TextView(this).apply {
            textSize = 13f
            setTextColor(Color.parseColor("#94A3B8"))
            typeface = Typeface.MONOSPACE
            setPadding(0, dp(8), 0, dp(8))
            isClickable = true
            setOnClickListener {
                copyToClipboard("Token", token)
            }
        }

        sheetStatus = TextView(this).apply {
            textSize = 14f
            setTextColor(Color.WHITE)
            setPadding(0, dp(6), 0, dp(10))
        }

        endpointsContainer = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(4), 0, dp(8))
        }

        previewCaption = TextView(this).apply {
            text = "Camera preview active on desktop stream"
            textSize = 12f
            setTextColor(Color.parseColor("#94A3B8"))
            visibility = View.GONE
            setPadding(0, dp(6), 0, 0)
        }

        val radios = RadioGroup(this).apply {
            id = View.generateViewId()
            addView(RadioButton(this@CameraActivity).apply {
                text = "Show Status"; setTextColor(Color.parseColor("#F8FAFC")); id = View.generateViewId()
            })
            addView(RadioButton(this@CameraActivity).apply {
                text = "Black Screen (AMOLED Save)"; setTextColor(Color.parseColor("#F8FAFC")); id = View.generateViewId(); isChecked = true
            })
            setOnCheckedChangeListener { _, checkedId ->
                displayMode = if (checkedId == getChildAt(0).id)
                    ScopeUi.DisplayMode.SHOW_CAMERA else ScopeUi.DisplayMode.BLACK_SCREEN
                previewCaption.visibility = if (displayMode == ScopeUi.DisplayMode.SHOW_CAMERA) View.VISIBLE else View.GONE
            }
        }

        val sheetContent = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(16), dp(20), dp(24))
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#0F141E"))
                setStroke(dp(1), Color.parseColor("#14FFFFFF"))
                cornerRadii = floatArrayOf(dp(16).toFloat(), dp(16).toFloat(), dp(16).toFloat(), dp(16).toFloat(), 0f, 0f, 0f, 0f)
            }

            addView(sectionLabel("CONNECTION"))
            addView(sheetStatus)

            // Start / Stop controls
            addView(LinearLayout(this@CameraActivity).apply {
                orientation = LinearLayout.HORIZONTAL
                addView(startButton, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginEnd = dp(8) })
                addView(stopButton, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginStart = dp(8) })
            })

            // QR Pairing Action Buttons
            addView(LinearLayout(this@CameraActivity).apply {
                orientation = LinearLayout.HORIZONTAL
                setPadding(0, dp(8), 0, 0)
                addView(button("Scan Desktop QR", "#2563EB").apply {
                    setOnClickListener {
                        qrScanLauncher.launch(Intent(this@CameraActivity, QrScanActivity::class.java))
                    }
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginEnd = dp(6) })

                addView(button("Show Stream QR", "#059669").apply {
                    setOnClickListener {
                        showStreamQrDialog()
                    }
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginStart = dp(6) })
            })

            addView(tokenRow)

            // Dynamic Endpoints list (Wi-Fi, Tailscale, ADB)
            addView(sectionLabel("AVAILABLE NETWORK ENDPOINTS"))
            addView(endpointsContainer)

            addView(sectionLabel("DISPLAY WHILE CONNECTED"))
            addView(radios)
            addView(previewCaption)
        }

        sheet = ScrollView(this).apply {
            addView(sheetContent)
            visibility = View.GONE
        }

        root.addView(sheet, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.WRAP_CONTENT, Gravity.BOTTOM
        ))

        // Floating round button, bottom-right
        val fab = TextView(this).apply {
            text = "•••"
            textSize = 18f
            setTextColor(Color.parseColor("#F8FAFC"))
            gravity = Gravity.CENTER
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.parseColor("#1AFFFFFF"))
                setStroke(dp(1), Color.parseColor("#33FFFFFF"))
            }
            contentDescription = "Settings"
            isClickable = true
            setOnClickListener {
                sheet.visibility = if (sheet.visibility == View.VISIBLE) View.GONE else View.VISIBLE
            }
        }
        root.addView(fab, FrameLayout.LayoutParams(dp(48), dp(48), Gravity.BOTTOM or Gravity.END).apply {
            setMargins(0, 0, dp(16), dp(16))
        })

        setContentView(root)
        refreshEndpoints()
        bind(null)

        if (ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
            autoStartOnPermission = false
            cameraPermissionLauncher.launch(Manifest.permission.CAMERA)
        }
    }

    private fun sectionLabel(text: String) = TextView(this).apply {
        this.text = text
        textSize = 11f
        setTextColor(Color.parseColor("#94A3B8"))
        letterSpacing = 0.08f
        setPadding(0, dp(12), 0, dp(4))
    }

    private fun button(text: String, bg: String) = Button(this).apply {
        this.text = text
        setTextColor(Color.WHITE)
        textSize = 13f
        backgroundTintList = ColorStateList.valueOf(Color.parseColor(bg))
    }

    private fun refreshEndpoints() {
        endpointsContainer.removeAllViews()
        val port = server?.port ?: MjpegHttpServer.DEFAULT_PORT
        val endpoints = NetworkHelper.getAvailableEndpoints(port, token)

        for (ep in endpoints) {
            val epView = TextView(this).apply {
                textSize = 12f
                typeface = Typeface.MONOSPACE
                setPadding(dp(8), dp(6), dp(8), dp(6))

                val badge = when (ep.type) {
                    NetworkHelper.EndpointType.TAILSCALE -> "[Tailscale VPN] "
                    NetworkHelper.EndpointType.WIFI_LAN -> "[Wi-Fi LAN] "
                    NetworkHelper.EndpointType.LOOPBACK -> "[ADB Loopback] "
                    NetworkHelper.EndpointType.OTHER -> "[Network] "
                }

                val sb = SpannableStringBuilder()
                sb.append(badge)
                val badgeColor = when (ep.type) {
                    NetworkHelper.EndpointType.TAILSCALE -> Color.parseColor("#38BDF8") // Cyan/Tailscale
                    NetworkHelper.EndpointType.WIFI_LAN -> Color.parseColor("#4ADE80") // Green
                    NetworkHelper.EndpointType.LOOPBACK -> Color.parseColor("#A78BFA") // Purple
                    NetworkHelper.EndpointType.OTHER -> Color.parseColor("#94A3B8")
                }
                sb.setSpan(ForegroundColorSpan(badgeColor), 0, badge.length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
                sb.append("${ep.ip}:$port")

                text = sb
                setTextColor(Color.parseColor("#E2E8F0"))

                background = GradientDrawable().apply {
                    setColor(Color.parseColor("#141E2E"))
                    cornerRadius = dp(6).toFloat()
                }

                setOnClickListener {
                    copyToClipboard("Stream URL", ep.streamUrl)
                }
            }

            endpointsContainer.addView(epView, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                setMargins(0, dp(3), 0, dp(3))
            })
        }
    }

    private fun copyToClipboard(label: String, text: String) {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText(label, text))
        Toast.makeText(this, "Copied $label to clipboard", Toast.LENGTH_SHORT).show()
    }

    private fun showStreamQrDialog() {
        val port = server?.port ?: MjpegHttpServer.DEFAULT_PORT
        val endpoints = NetworkHelper.getAvailableEndpoints(port, token)
        val primaryEndpoint = endpoints.firstOrNull { it.type != NetworkHelper.EndpointType.LOOPBACK } ?: endpoints.first()

        try {
            val qrBitmap = NetworkHelper.generateQrBitmap(primaryEndpoint.streamUrl, dp(260))
            val imageView = ImageView(this).apply {
                setImageBitmap(qrBitmap)
                setPadding(dp(16), dp(16), dp(16), dp(8))
            }

            val container = LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER
                setBackgroundColor(Color.parseColor("#0F141E"))
                addView(imageView)
                addView(TextView(this@CameraActivity).apply {
                    text = primaryEndpoint.streamUrl
                    textSize = 11f
                    setTextColor(Color.parseColor("#94A3B8"))
                    gravity = Gravity.CENTER
                    setPadding(dp(16), 0, dp(16), dp(16))
                })
            }

            AlertDialog.Builder(this, android.R.style.Theme_DeviceDefault_Dialog_Alert)
                .setTitle("Scan Stream QR")
                .setView(container)
                .setPositiveButton("Done", null)
                .setNeutralButton("Copy URL") { _, _ ->
                    copyToClipboard("Stream URL", primaryEndpoint.streamUrl)
                }
                .show()
        } catch (e: Exception) {
            Toast.makeText(this, "Failed to generate QR: ${e.message}", Toast.LENGTH_SHORT).show()
        }
    }

    private fun handleScannedDesktopPayload(raw: String) {
        try {
            val json = JSONObject(raw)
            val secret = json.optString("secret", "")
            val port = json.optInt("port", 8101)
            val ips = ArrayList<String>()

            val ipsArray = json.optJSONArray("desktop_ips")
            if (ipsArray != null) {
                for (i in 0 until ipsArray.length()) {
                    val ip = ipsArray.getString(i)
                    if (ip.isNotBlank() && !ips.contains(ip)) {
                        ips.add(ip)
                    }
                }
            }
            if (ips.isEmpty()) {
                val hint = json.optString("endpoint_hint", "")
                val extracted = hint.replace("http://", "").replace("ws://", "").split("/").firstOrNull()?.split(":")?.firstOrNull()
                if (!extracted.isNullOrBlank()) ips.add(extracted)
            }

            if (ips.isEmpty() || secret.isBlank()) {
                Toast.makeText(this, "Invalid QR code: missing desktop IP or secret", Toast.LENGTH_LONG).show()
                return
            }

            centerHint.text = "Connecting to Desktop at ${ips.first()}:$port..."
            Toast.makeText(this, "Connecting to Desktop...", Toast.LENGTH_SHORT).show()

            // Run network pairing on background thread
            Executors.newSingleThreadExecutor().execute {
                var pairedSuccess = false
                var connectedIp = ""
                var lastError = "No response from desktop"

                val myPort = server?.port ?: MjpegHttpServer.DEFAULT_PORT
                val myEndpoints = NetworkHelper.getAvailableEndpoints(myPort, token)
                val myIp = myEndpoints.firstOrNull { it.type != NetworkHelper.EndpointType.LOOPBACK }?.ip ?: "127.0.0.1"
                val myDeviceName = "${android.os.Build.MANUFACTURER} ${android.os.Build.MODEL}".trim()

                for (desktopIp in ips) {
                    try {
                        val url = java.net.URL("http://$desktopIp:$port/pair")
                        val conn = url.openConnection() as java.net.HttpURLConnection
                        conn.requestMethod = "POST"
                        conn.connectTimeout = 3000
                        conn.readTimeout = 3000
                        conn.doOutput = true
                        conn.setRequestProperty("Content-Type", "application/json")

                        val body = JSONObject().apply {
                            put("secret", secret)
                            put("phone_ip", myIp)
                            put("phone_port", myPort)
                            put("phone_token", token)
                            put("phone_name", myDeviceName)
                        }.toString()

                        conn.outputStream.use { os ->
                            os.write(body.toByteArray(Charsets.UTF_8))
                        }

                        val code = conn.responseCode
                        if (code == 200) {
                            pairedSuccess = true
                            connectedIp = desktopIp
                            break
                        } else {
                            lastError = "HTTP $code from Desktop"
                        }
                    } catch (e: Exception) {
                        lastError = e.message ?: "Connection timed out"
                    }
                }

                mainHandler.post {
                    if (pairedSuccess) {
                        centerHint.text = "✓ Paired with Desktop ($connectedIp)"
                        Toast.makeText(this@CameraActivity, "✓ Paired with Desktop ($connectedIp)! Video active.", Toast.LENGTH_LONG).show()
                        if (server == null) {
                            startStreaming()
                        }
                    } else {
                        centerHint.text = "❌ Cannot reach Desktop ($lastError)"
                        AlertDialog.Builder(this@CameraActivity, android.R.style.Theme_DeviceDefault_Dialog_Alert)
                            .setTitle("Pairing Failed")
                            .setMessage("Could not connect to Desktop at ${ips.joinToString(", ")}:$port.\n\nError: $lastError\n\nPlease verify:\n1. Phone and PC are connected to the same Wi-Fi network (or Tailscale VPN).\n2. PC firewall allows incoming connection on port $port.")
                            .setPositiveButton("OK", null)
                            .show()
                    }
                }
            }
        } catch (e: Exception) {
            Toast.makeText(this, "Invalid pairing payload: ${e.message}", Toast.LENGTH_LONG).show()
        }
    }

    private fun bind(startFailedMessage: String?) {
        mainHandler.post {
            val ui = ScopeUi.map(server != null, startFailedMessage)
            centerStatus.text = statusSpannable(ui)
            sheetStatus.text = statusSpannable(ui)

            if (server != null) {
                val feedType = if (isUsingRealCamera) "Real Camera2 (1080p)" else "Synthetic Emulator Feed"
                centerHint.text = "$feedType • Port ${server?.port}"
            } else if (startFailedMessage != null) {
                centerHint.text = startFailedMessage
            } else {
                centerHint.text = "Ready to stream on LAN / Tailscale / ADB"
            }

            tokenRow.visibility = if (ui.tokenLine == null) View.GONE else View.VISIBLE
            tokenRow.text = "${ui.tokenLine} $token (Tap to copy)"
            startButton.isEnabled = server == null
            stopButton.isEnabled = server != null
        }
    }

    private fun statusSpannable(ui: ScopeUi.StatusUi): SpannableStringBuilder {
        val sb = SpannableStringBuilder()
        sb.append("●")
        sb.setSpan(ForegroundColorSpan(Color.parseColor(ui.dotHex)), 0, 1, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
        sb.append("  ").append(ui.statusWord)
        return sb
    }

    private fun startStreaming() {
        if (server != null) return

        if (ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
            startStreamingInternal(preferRealCamera = true)
        } else {
            autoStartOnPermission = true
            cameraPermissionLauncher.launch(Manifest.permission.CAMERA)
        }
    }

    private fun startStreamingInternal(preferRealCamera: Boolean) {
        val s = MjpegHttpServer(
            frameSupplier = { queue.dequeue() },
            token = token,
            bindAddress = "0.0.0.0"
        )
        try {
            s.start()
        } catch (e: Exception) {
            bind(e.message)
            return
        }
        server = s

        var cameraStarted = false
        if (preferRealCamera) {
            val src = Camera2Source(this)
            cameraSource = src
            src.openCamera(
                cameraId = "0",
                onOpened = {
                    val captured = src.startCapture(1920, 1080, 30) { frameBytes ->
                        queue.enqueue(frameBytes)
                    }
                    if (captured) {
                        isUsingRealCamera = true
                        cameraStarted = true
                        mainHandler.post { bind(null) }
                    } else {
                        startSyntheticFallback("Failed to configure 1080p Camera2 stream")
                    }
                },
                onError = { _, msg ->
                    startSyntheticFallback("Camera2 open: $msg")
                }
            )
        } else {
            startSyntheticFallback(null)
        }

        // Keep streaming alive when screen is locked via Foreground Service
        val port = s.port
        val primaryEndpoint = NetworkHelper.getAvailableEndpoints(port, token).firstOrNull()
        CameraStreamService.start(this, primaryEndpoint?.displayLabel ?: "Port $port")

        refreshEndpoints()
        bind(null)
    }

    private fun startSyntheticFallback(reason: String?) {
        isUsingRealCamera = false
        val sched = Executors.newSingleThreadScheduledExecutor { r ->
            Thread(r, "mjpeg-synthetic").apply { isDaemon = true }
        }
        scheduler = sched
        frameTask = sched.scheduleAtFixedRate({
            queue.enqueue(syntheticJpeg())
        }, 0, 500, TimeUnit.MILLISECONDS)
        mainHandler.post { bind(reason) }
    }

    private fun stopStreaming() {
        frameTask?.cancel(false)
        frameTask = null
        scheduler?.shutdownNow()
        scheduler = null

        cameraSource?.stopCapture()
        cameraSource?.close()
        cameraSource = null
        isUsingRealCamera = false

        server?.stop()
        server = null
        queue.clear()

        CameraStreamService.stop(this)
        bind(null)
    }

    private fun syntheticJpeg(): ByteArray {
        val bmp = Bitmap.createBitmap(320, 240, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bmp)
        canvas.drawColor(Color.DKGRAY)
        val paint = Paint().apply { color = Color.GREEN; textSize = 24f }
        canvas.drawText("frame ${System.currentTimeMillis()}", 10f, 100f, paint)
        canvas.drawText("dropped=${queue.droppedFramesCount}", 10f, 140f, paint)
        canvas.drawText("feed: synthetic", 10f, 180f, paint)
        val out = ByteArrayOutputStream()
        bmp.compress(Bitmap.CompressFormat.JPEG, 60, out)
        bmp.recycle()
        return out.toByteArray()
    }

    private fun dp(v: Int): Int = (v * resources.displayMetrics.density).toInt()

    override fun onDestroy() {
        super.onDestroy()
        stopStreaming()
    }
}
