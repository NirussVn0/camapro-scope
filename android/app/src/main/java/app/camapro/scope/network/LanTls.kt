package app.camapro.scope.network

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import java.math.BigInteger
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.MessageDigest
import java.security.SecureRandom
import java.security.cert.CertificateException
import java.security.cert.X509Certificate
import java.security.spec.ECGenParameterSpec
import java.util.Date
import javax.net.ssl.*
import javax.security.auth.x500.X500Principal

/** D03: native non-exportable phone key; exact DER SHA-256 pins, TLS 1.3 only. */
class LanTls(context: Context) {
    private val preferences = EncryptedSharedPreferences.create(
        context, "lan-trust-v1", MasterKey.Builder(context).setKeyScheme(MasterKey.KeyScheme.AES256_GCM).build(),
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )
    val credentials = LanCredentials(
        { preferences.getString(it, null) },
        { key, value -> preferences.edit().putString(key, value).commit() }
    )
    private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

    init {
        if (!keyStore.containsAlias(ALIAS)) {
            KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC, "AndroidKeyStore").apply {
                initialize(KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY)
                    .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
                    .setDigests(KeyProperties.DIGEST_SHA256)
                    .setCertificateSubject(X500Principal("CN=Camapro Scope"))
                    .setCertificateSerialNumber(BigInteger(128, SecureRandom()))
                    .setCertificateNotBefore(Date(System.currentTimeMillis() - 86_400_000L))
                    .setCertificateNotAfter(Date(System.currentTimeMillis() + 10L * 365 * 86_400_000L))
                    .build())
                generateKeyPair()
            }
        }
    }

    val fingerprint: String get() = fingerprint(keyStore.getCertificate(ALIAS).encoded)

    fun serverFactory(pin: String? = credentials.trustedPin()): SSLServerSocketFactory {
        val managers = KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm()).apply { init(keyStore, null) }
        return SSLContext.getInstance("TLSv1.3").apply {
            init(managers.keyManagers, arrayOf(pinTrustManager { pin }), SecureRandom())
        }.serverSocketFactory
    }

    fun openPinned(url: java.net.URL, pin: String): HttpsURLConnection {
        require(url.protocol == "https" && validPin(pin)) { "Pinned HTTPS required" }
        val context = SSLContext.getInstance("TLSv1.3").apply {
            init(null, arrayOf(pinTrustManager { pin }), SecureRandom())
        }
        return (url.openConnection() as HttpsURLConnection).apply {
            sslSocketFactory = Tls13SocketFactory(context.socketFactory)
            // IP/SAN names are not enrollment identity: the QR's exact certificate pin is.
            hostnameVerifier = HostnameVerifier { _, session ->
                session.peerCertificates.firstOrNull()?.let { fingerprint(it.encoded) == pin } == true
            }
            instanceFollowRedirects = false
        }
    }

    companion object {
        private const val ALIAS = "camapro-lan-tls-v1"
        fun fingerprint(der: ByteArray): String = "sha256:" + MessageDigest.getInstance("SHA-256").digest(der).joinToString("") { "%02x".format(it) }
        fun validPin(pin: String) = Regex("sha256:[0-9a-f]{64}").matches(pin)
        fun randomToken(): String = ByteArray(32).also { SecureRandom().nextBytes(it) }.joinToString("") { "%02x".format(it) }
        private fun pinTrustManager(pin: () -> String?) = object : X509TrustManager {
            override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
            private fun check(chain: Array<out X509Certificate>) {
                val expected = pin() ?: throw CertificateException("Desktop is not enrolled")
                if (chain.isEmpty() || fingerprint(chain[0].encoded) != expected) throw CertificateException("Certificate pin mismatch")
                chain[0].checkValidity()
            }
            override fun checkClientTrusted(chain: Array<out X509Certificate>, authType: String) = check(chain)
            override fun checkServerTrusted(chain: Array<out X509Certificate>, authType: String) = check(chain)
        }
    }

    private class Tls13SocketFactory(private val delegate: SSLSocketFactory) : SSLSocketFactory() {
        private fun configure(socket: java.net.Socket): java.net.Socket = (socket as SSLSocket).apply {
            enabledProtocols = arrayOf("TLSv1.3")
        }
        override fun getDefaultCipherSuites(): Array<String> = delegate.defaultCipherSuites
        override fun getSupportedCipherSuites(): Array<String> = delegate.supportedCipherSuites
        override fun createSocket(): java.net.Socket = configure(delegate.createSocket())
        override fun createSocket(s: java.net.Socket, host: String, port: Int, autoClose: Boolean): java.net.Socket = configure(delegate.createSocket(s, host, port, autoClose))
        override fun createSocket(host: String, port: Int): java.net.Socket = configure(delegate.createSocket(host, port))
        override fun createSocket(host: String, port: Int, local: java.net.InetAddress, localPort: Int): java.net.Socket = configure(delegate.createSocket(host, port, local, localPort))
        override fun createSocket(host: java.net.InetAddress, port: Int): java.net.Socket = configure(delegate.createSocket(host, port))
        override fun createSocket(host: java.net.InetAddress, port: Int, local: java.net.InetAddress, localPort: Int): java.net.Socket = configure(delegate.createSocket(host, port, local, localPort))
    }
}
