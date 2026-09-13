package app.camapro.scope

import android.app.Activity
import android.os.Bundle
import android.widget.TextView

class MainActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val text = TextView(this)
        text.text = "Camapro Scope skeleton (G1.3). No camera functionality is implemented."
        setContentView(text)
    }
}
