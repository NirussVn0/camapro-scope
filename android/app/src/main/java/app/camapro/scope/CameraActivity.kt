package app.camapro.scope

import android.app.Activity
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
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
 * Dev/demo screen: synthetic 2fps JPEG producer -> BoundedFrameQueue ->
 * MjpegHttpServer on 127.0.0.1:8100. No Camera2 usage here (engine/fake stay
 * in unit tests); real source wiring is a later slice.
 *
 * ponytail: frames are a timestamped bitmap, not camera output; swap producer
 * for CameraEngine.onFrame -> queue.enqueue when the camera slice lands.
 */
class CameraActivity : Activity() {

    private val queue = BoundedFrameQueue()
    private var server: MjpegHttpServer? = null
    private var scheduler: ScheduledExecutorService? = null
    private var frameTask: ScheduledFuture<*>? = null
    private val token: String = Random.nextLong(0x10000000, 0xFFFFFFF).toString(16)
    private val mainHandler = Handler(Looper.getMainLooper())

    private lateinit var statusText: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        statusText = TextView(this).apply {
            textSize = 14f
            typeface = android.graphics.Typeface.MONOSPACE
            setPadding(32, 32, 32, 16)
        }
        val startButton = Button(this).apply { text = "Start" }
        val stopButton = Button(this).apply { text = "Stop" }
        val buttons = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_HORIZONTAL
            addView(startButton)
            addView(stopButton)
        }
        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            addView(statusText)
            addView(buttons)
        })

        startButton.setOnClickListener { startStreaming() }
        stopButton.setOnClickListener { stopStreaming() }
        updateStatus("Idle", "token: $token")
    }

    private fun startStreaming() {
        if (server != null) return
        val s = MjpegHttpServer(frameSupplier = { queue.dequeue() }, token = token)
        try {
            s.start()
        } catch (e: Exception) {
            updateStatus("Start failed: ${e.message}", "token: $token")
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
        updateStatus("Streaming http://127.0.0.1:${s.port}/stream", "token: $token")
    }

    private fun stopStreaming() {
        frameTask?.cancel(false)
        frameTask = null
        scheduler?.shutdownNow()
        scheduler = null
        server?.stop()
        server = null
        queue.clear()
        updateStatus("Idle", "token: $token")
    }

    private fun updateStatus(state: String, extra: String) {
        mainHandler.post { statusText.text = "$state\n$extra" }
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

    override fun onDestroy() {
        super.onDestroy()
        stopStreaming()
    }
}
