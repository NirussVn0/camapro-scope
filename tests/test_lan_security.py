"""Source-level regression checks complement the native TLS handshake tests."""
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class LanSecurity(unittest.TestCase):
    def test_pairing_has_no_predictable_identity_or_cleartext_callback(self):
        source = (ROOT / 'desktop/src-tauri/src/core/control/pairing_server.rs').read_text()
        self.assertNotIn('now_ms ^', source)
        self.assertNotIn('format!("http://', source)
        self.assertNotIn('format!("sha256:{:064x}", now_ms)', source)

    def test_media_authorization_is_header_only(self):
        source = (ROOT / 'android/app/src/main/java/app/camapro/scope/transport/MjpegHttpServer.kt').read_text()
        self.assertNotIn('?: queryToken', source)

    def test_pairing_never_starts_capture(self):
        source = (ROOT / 'android/app/src/main/java/app/camapro/scope/CameraActivity.kt').read_text()
        pairing = source.split('private fun handleScannedDesktopPayload')[1].split('private fun bind(')[0]
        self.assertNotIn('startStreaming()', pairing)
        self.assertNotIn('"http://', pairing)

    def test_trust_commit_is_after_callback_success_and_token_is_persistent(self):
        source = (ROOT / 'android/app/src/main/java/app/camapro/scope/CameraActivity.kt').read_text()
        pairing = source.split('private fun handleScannedDesktopPayload')[1].split('private fun bind(')[0]
        self.assertNotIn('trustDesktop(', pairing)
        self.assertIn('serverFactory(pin), provisional = true', pairing)
        self.assertGreater(pairing.index('credentials.commit('), pairing.index('if (code == 200)'))
        self.assertIn('token: String get() = lanTls.credentials.token', source)

    def test_desktop_success_requires_post_commit_confirmation(self):
        desktop = (ROOT / 'desktop/src-tauri/src/core/control/pairing_server.rs').read_text()
        phone = (ROOT / 'android/app/src/main/java/app/camapro/scope/CameraActivity.kt').read_text()
        self.assertIn('POST /pair/confirm HTTP/1.1', desktop)
        self.assertIn('probe_ready', desktop)
        pairing = phone.split('private fun handleScannedDesktopPayload')[1].split('private fun bind(')[0]
        self.assertGreater(pairing.index('/pair/confirm'), pairing.index('credentials.commit('))
        self.assertGreater(pairing.index('/pair/confirm'), pairing.index('endpoint.replaceTlsFactory(lanTls.serverFactory())'))


if __name__ == '__main__':
    unittest.main()
