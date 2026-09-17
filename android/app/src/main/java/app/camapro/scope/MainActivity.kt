package app.camapro.scope

import android.app.Activity
import android.content.Intent
import android.os.Bundle

/**
 * Trampoline forwarding legacy G1.3 MainActivity invocations to CameraActivity.
 * Removed from LAUNCHER in AndroidManifest.xml to eliminate duplicate app icons.
 */
class MainActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val intent = Intent(this, CameraActivity::class.java).apply {
            addFlags(Intent.FLAG_ACTIVITY_FORWARD_RESULT)
        }
        startActivity(intent)
        finish()
    }
}
