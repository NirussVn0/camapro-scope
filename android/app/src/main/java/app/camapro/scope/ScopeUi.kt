package app.camapro.scope

/**
 * Pure state mapper for the CameraActivity UI: server state + token ->
 * status dot color token, status word, and what the sheet shows.
 * No Android imports — JVM testable.
 */
object ScopeUi {

    enum class DisplayMode { SHOW_CAMERA, BLACK_SCREEN }

    /** serverIsRunning, startFailedMessage -> UI strings + dot color token. */
    data class StatusUi(
        val dotHex: String,
        val statusWord: String,
        val hint: String,
        val tokenLine: String?
    )

    const val GRAY = "#9AA3AF"
    const val GREEN = "#4ADE80"
    const val RED = "#F87171"

    fun map(serverIsRunning: Boolean, startFailedMessage: String? = null): StatusUi {
        if (serverIsRunning) {
            return StatusUi(GREEN, "Streaming", "", "Token:")
        }
        if (startFailedMessage != null) {
            return StatusUi(RED, "Not connected", startFailedMessage, null)
        }
        return StatusUi(GRAY, "Not connected", "Waiting for desktop", null)
    }

    val DEFAULT_DISPLAY_MODE: DisplayMode = DisplayMode.BLACK_SCREEN
}
