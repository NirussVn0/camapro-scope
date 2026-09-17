package app.camapro.scope

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import android.util.Size
import android.view.View
import android.widget.TextView
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.ComponentActivity
import androidx.core.app.ActivityCompat
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.core.content.ContextCompat
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/**
 * QR code scanner for pairing with desktop.
 * Scans QR codes containing QrPayload JSON, returns result to caller via setResult().
 * ponytail: minimal implementation; add viewfinder overlay and haptic feedback later.
 */
class QrScanActivity : ComponentActivity() {

    companion object {
        const val EXTRA_QR_PAYLOAD = "qr_payload"
        private const val REQUEST_CODE = 1001
    }

    private lateinit var previewView: PreviewView
    private lateinit var statusText: TextView
    private lateinit var cameraExecutor: ExecutorService
    private var scanned = false

    private val permissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) startCamera() else finishWithError("Camera permission denied")
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Programmatic layout — no XML dependency
        val root = android.widget.FrameLayout(this).apply {
            layoutParams = android.widget.FrameLayout.LayoutParams(
                android.widget.FrameLayout.LayoutParams.MATCH_PARENT,
                android.widget.FrameLayout.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(0xFF000000.toInt())
        }

        previewView = PreviewView(this).apply {
            layoutParams = android.widget.FrameLayout.LayoutParams(
                android.widget.FrameLayout.LayoutParams.MATCH_PARENT,
                android.widget.FrameLayout.LayoutParams.MATCH_PARENT
            )
        }
        root.addView(previewView)

        // Close button (top-left)
        val closeBtn = TextView(this).apply {
            layoutParams = android.widget.FrameLayout.LayoutParams(
                android.widget.FrameLayout.LayoutParams.WRAP_CONTENT,
                android.widget.FrameLayout.LayoutParams.WRAP_CONTENT,
                android.view.Gravity.TOP or android.view.Gravity.START
            ).apply {
                setMargins(48, 64, 0, 0)
            }
            text = "✕ Close"
            setTextColor(0xFFFFFFFF.toInt())
            textSize = 15f
            setPadding(32, 16, 32, 16)
            background = android.graphics.drawable.GradientDrawable().apply {
                setColor(0x66000000.toInt())
                cornerRadius = 24f
            }
            isClickable = true
            setOnClickListener { finish() }
        }
        root.addView(closeBtn)

        statusText = TextView(this).apply {
            layoutParams = android.widget.FrameLayout.LayoutParams(
                android.widget.FrameLayout.LayoutParams.WRAP_CONTENT,
                android.widget.FrameLayout.LayoutParams.WRAP_CONTENT,
                android.view.Gravity.BOTTOM or android.view.Gravity.CENTER_HORIZONTAL
            ).apply { bottomMargin = 96 }
            text = "Point camera at Desktop QR code"
            setTextColor(0xFFFFFFFF.toInt())
            textSize = 15f
            setPadding(32, 16, 32, 16)
            background = android.graphics.drawable.GradientDrawable().apply {
                setColor(0x88000000.toInt())
                cornerRadius = 24f
            }
        }
        root.addView(statusText)

        setContentView(root)

        cameraExecutor = Executors.newSingleThreadExecutor()

        when {
            ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA) ==
                PackageManager.PERMISSION_GRANTED -> startCamera()
            shouldShowRequestPermissionRationale(Manifest.permission.CAMERA) -> {
                statusText.text = "Camera needed to scan pairing QR"
                permissionLauncher.launch(Manifest.permission.CAMERA)
            }
            else -> permissionLauncher.launch(Manifest.permission.CAMERA)
        }
    }

    private fun startCamera() {
        val cameraProviderFuture = ProcessCameraProvider.getInstance(this)
        cameraProviderFuture.addListener({
            val cameraProvider = cameraProviderFuture.get()

            val preview = Preview.Builder().build().also {
                it.setSurfaceProvider(previewView.surfaceProvider)
            }

            val imageAnalysis = ImageAnalysis.Builder()
                .setTargetResolution(Size(1280, 720))
                .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                .build()
                .also { analysis ->
                    val options = BarcodeScannerOptions.Builder()
                        .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
                        .build()
                    val scanner = BarcodeScanning.getClient(options)

                    analysis.setAnalyzer(cameraExecutor) { imageProxy ->
                        if (scanned) {
                            imageProxy.close()
                            return@setAnalyzer
                        }
                        val mediaImage = imageProxy.image ?: run {
                            imageProxy.close()
                            return@setAnalyzer
                        }
                        val inputImage = InputImage.fromMediaImage(
                            mediaImage, imageProxy.imageInfo.rotationDegrees
                        )
                        scanner.process(inputImage)
                            .addOnSuccessListener { barcodes ->
                                for (barcode in barcodes) {
                                    val raw = barcode.rawValue ?: continue
                                    if (!scanned && isValidQrPayload(raw)) {
                                        scanned = true
                                        runOnUiThread {
                                            statusText.text = "✓ Paired"
                                        }
                                        setResult(RESULT_OK, android.content.Intent().apply {
                                            putExtra(EXTRA_QR_PAYLOAD, raw)
                                        })
                                        finish()
                                        break
                                    }
                                }
                            }
                            .addOnFailureListener {
                                // Transient frame error; keep scanning
                            }
                            .addOnCompleteListener { imageProxy.close() }
                    }
                }

            try {
                cameraProvider.unbindAll()
                cameraProvider.bindToLifecycle(
                    this, CameraSelector.DEFAULT_BACK_CAMERA, preview, imageAnalysis
                )
            } catch (e: Exception) {
                finishWithError("Camera bind failed: ${e.message}")
            }
        }, ContextCompat.getMainExecutor(this))
    }

    /**
     * Basic validation: must be valid JSON with required QrPayload fields.
     * Full schema validation happens on desktop side after enrollment.
     */
    private fun isValidQrPayload(raw: String): Boolean {
        return try {
            val json = org.json.JSONObject(raw)
            json.has("version") && json.has("endpoint_hint") &&
                json.has("peer_fingerprint_sha256") && json.has("secret") &&
                json.has("expires_at_ms")
        } catch (_: Exception) {
            false
        }
    }

    private fun finishWithError(msg: String) {
        setResult(RESULT_CANCELED, android.content.Intent().apply {
            putExtra(EXTRA_QR_PAYLOAD, msg)
        })
        finish()
    }

    override fun onDestroy() {
        super.onDestroy()
        cameraExecutor.shutdown()
    }
}
