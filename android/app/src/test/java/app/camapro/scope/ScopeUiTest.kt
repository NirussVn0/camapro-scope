package app.camapro.scope

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ScopeUiTest {

    @Test
    fun disconnectedIsGrayDotWithWaitingHint() {
        val ui = ScopeUi.map(serverIsRunning = false)
        assertEquals(ScopeUi.GRAY, ui.dotHex)
        assertEquals("Not connected", ui.statusWord)
        assertEquals("Waiting for desktop", ui.hint)
        assertNull(ui.tokenLine)
    }

    @Test
    fun streamingIsGreenDotWithTokenRow() {
        val ui = ScopeUi.map(serverIsRunning = true)
        assertEquals(ScopeUi.GREEN, ui.dotHex)
        assertEquals("Streaming", ui.statusWord)
        assertEquals("Token:", ui.tokenLine)
    }

    @Test
    fun failedStartKeepsNotConnectedWordAndShowsError() {
        val ui = ScopeUi.map(serverIsRunning = false, startFailedMessage = "Address already in use")
        assertEquals(ScopeUi.RED, ui.dotHex)
        assertEquals("Not connected", ui.statusWord)
        assertEquals("Address already in use", ui.hint)
    }

    @Test
    fun statusNeverConveyedByColorAlone() {
        // dot + word: word is non-empty in every state
        listOf(true, false).forEach { running ->
            assertEquals(true, ScopeUi.map(running).statusWord.isNotEmpty())
        }
    }

    @Test
    fun displayModeDefaultsToBlackScreen() {
        assertEquals(ScopeUi.DisplayMode.BLACK_SCREEN, ScopeUi.DEFAULT_DISPLAY_MODE)
    }
}
