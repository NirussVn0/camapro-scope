package app.camapro.scope

import android.app.Activity
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
import android.widget.LinearLayout
import android.widget.RadioButton
import android.widget.RadioGroup
import android.widget.TextView
import app.camapro.scope.transport.BoundedFrameQueue
import app.camapro.scope.transport.MjpegHttpServer
import java.io.ByteArrayOutputStream
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit
import kotlin.random.Random

/**
 * Dev/demo screen per the mobile brief: black disconnected screen, floating
 * round button opening a compact bottom sheet (connection / token / display
 * radios / stop). Synthetic 2fps JPEG producer -> BoundedFrameQueue ->
 * MjpegHttpServer on 127.0.0.1:8100. Server behavior unchanged from the
 * pre-restyle version; only the chrome is new.
 *
 * ponytail: frames are a timestamped bitmap, not camera output; swap producer
 * for CameraEngine.onFrame -> queue.enqueue when the camera slice lands.
 * No glass blur (pre-Android-12 safe): solid #0F141E sheet + hairline stroke.
 */
class CameraActivity : Activity() {

    private val queue = BoundedFrameQueue()
    private var server: MjpegHttpServer? = null
    private var scheduler: ScheduledExecutorService? = null
    private var frameTask: ScheduledFuture<*>? = null
    private val token: String = Random.nextLong(0x10000000, 0xFFFFFFF0).toString(16)
    private val mainHandler = Handler(Looper.getMainLooper())
    private var displayMode: ScopeUi.DisplayMode = ScopeUi.DEFAULT_DISPLAY_MODE

    private lateinit var centerStatus: TextView
    private lateinit var centerHint: TextView
    private lateinit var sheet: View
    private lateinit var sheetStatus: TextView
    private lateinit var tokenRow: TextView
    private lateinit var previewCaption: TextView
    private lateinit var startButton: Button
    private lateinit var stopButton: Button

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val root = FrameLayout(this).apply { setBackgroundColor(Color.BLACK) }

        // Centered identity block: app name + dot/word status + hint.
        centerStatus = TextView(this).apply { textSize = 15f; setTextColor(Color.WHITE) }
        centerHint = TextView(this).apply { textSize = 13f; setTextColor(Color.parseColor("#94A3B8")); gravity = Gravity.CENTER }
        root.addView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(dp(24), dp(24), dp(24), dp(24))
            addView(TextView(this@CameraActivity).apply {
                text = "CamaPro Scope"
                textSize = 22f
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
            setPadding(0, dp(10), 0, dp(10))
        }
        sheetStatus = TextView(this).apply { textSize = 14f; setTextColor(Color.WHITE); setPadding(0, dp(6), 0, dp(12)) }
        previewCaption = TextView(this).apply {
            text = "Camera preview arrives in the next update"
            textSize = 12f
            setTextColor(Color.parseColor("#94A3B8"))
            visibility = View.GONE
            setPadding(0, dp(6), 0, 0)
        }

        val radios = RadioGroup(this).apply {
            id = View.generateViewId()
            addView(RadioButton(this@CameraActivity).apply {
                text = "Show Camera"; setTextColor(Color.parseColor("#F8FAFC")); id = View.generateViewId()
            })
            addView(RadioButton(this@CameraActivity).apply {
                text = "Black Screen"; setTextColor(Color.parseColor("#F8FAFC")); id = View.generateViewId(); isChecked = true
            })
            setOnCheckedChangeListener { _, checkedId ->
                displayMode = if (checkedId == getChildAt(0).id)
                    ScopeUi.DisplayMode.SHOW_CAMERA else ScopeUi.DisplayMode.BLACK_SCREEN
                // No local preview yet: same black, honest caption only.
                previewCaption.visibility = if (displayMode == ScopeUi.DisplayMode.SHOW_CAMERA) View.VISIBLE else View.GONE
            }
        }

        sheet = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(16), dp(20), dp(20))
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#0F141E"))
                setStroke(dp(1), Color.parseColor("#14FFFFFF"))
                cornerRadii = floatArrayOf(dp(12).toFloat(), dp(12).toFloat(), dp(12).toFloat(), dp(12).toFloat(), 0f, 0f, 0f, 0f)
            }
            visibility = View.GONE
            addView(sectionLabel("CONNECTION"))
            addView(sheetStatus)
            addView(LinearLayout(this@CameraActivity).apply {
                orientation = LinearLayout.HORIZONTAL
                addView(startButton, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginEnd = dp(8) })
                addView(stopButton, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginStart = dp(8) })
            })
            addView(tokenRow)
            addView(sectionLabel("DISPLAY WHILE CONNECTED"))
            addView(radios)
            addView(previewCaption)
        }
        root.addView(sheet, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.WRAP_CONTENT, Gravity.BOTTOM
        ))

        // Floating round button, bottom-right, subtle white/10 stroke.
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
        bind(null)
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
        backgroundTintList = ColorStateList.valueOf(Color.parseColor(bg))
    }

    private fun bind(startFailedMessage: String?) {
        mainHandler.post {
            val ui = ScopeUi.map(server != null, startFailedMessage)
            centerStatus.text = statusSpannable(ui)
            sheetStatus.text = statusSpannable(ui)
            centerHint.text = ui.hint
            tokenRow.visibility = if (ui.tokenLine == null) View.GONE else View.VISIBLE
            tokenRow.text = "${ui.tokenLine} $token"
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

    // --- server wiring: byte-identical to the pre-restyle activity ---

    private fun startStreaming() {
        if (server != null) return
        val s = MjpegHttpServer(frameSupplier = { queue.dequeue() }, token = token)
        try {
            s.start()
        } catch (e: Exception) {
            bind(e.message)
            return
        }
        server = s
        val sched = Executors.newSingleThreadScheduledExecutor { r ->
            Thread(r, "mjpeg-synthetic").apply { isDaemon = true }
        }
        scheduler = sched
        frameTask = sched.scheduleAtFixedRate({
            queue.enqueue(syntheticJpeg())
        }, 0, 500, TimeUnit.MILLISECONDS)
        bind(null)
    }

    private fun stopStreaming() {
        frameTask?.cancel(false)
        frameTask = null
        scheduler?.shutdownNow()
        scheduler = null
        server?.stop()
        server = null
        queue.clear()
        bind(null)
    }

    private fun syntheticJpeg(): ByteArray {
        val bmp = Bitmap.createBitmap(320, 240, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bmp)
        canvas.drawColor(Color.DKGRAY)
        val paint = Paint().apply { color = Color.GREEN; textSize = 28f }
        canvas.drawText("frame ${System.currentTimeMillis()}", 10f, 120f, paint)
        canvas.drawText("dropped=${queue.droppedFramesCount}", 10f, 160f, paint)
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
