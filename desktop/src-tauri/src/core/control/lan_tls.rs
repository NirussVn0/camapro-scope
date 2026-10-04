//! All LAN connections use an enrolled certificate pin and our client identity.
//! Secrets/keys persist only in the OS keyring; failure is terminal, never TOFU.
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme, StreamOwned};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

pub type TlsStream = StreamOwned<ClientConnection, TcpStream>;

pub fn fingerprint(der: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(der))
}

pub fn valid_pin(pin: &str) -> bool {
    pin.strip_prefix("sha256:").is_some_and(|v| {
        v.len() == 64
            && v.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    })
}

pub fn random_secret() -> Result<String, String> {
    let mut bytes = [0; 32];
    getrandom::getrandom(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Identity {
    pub cert: Vec<u8>,
    key: Vec<u8>,
}

impl Identity {
    pub fn generate() -> Result<Self, String> {
        let rcgen::CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(vec!["camapro.local".into()])
                .map_err(|e| e.to_string())?;
        Ok(Self {
            cert: cert.der().to_vec(),
            key: signing_key.serialize_der(),
        })
    }

    pub fn load() -> Result<Self, String> {
        let entry = keyring::Entry::new("app.camapro.scope", "lan-identity-v1")
            .map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(s) => serde_json::from_str(&s).map_err(|_| "Invalid keyring identity".into()),
            Err(keyring::Error::NoEntry) => {
                let identity = Self::generate()?;
                entry
                    .set_password(&serde_json::to_string(&identity).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("Secure identity storage unavailable: {e}"))?;
                Ok(identity)
            }
            Err(e) => Err(format!("Secure identity storage unavailable: {e}")),
        }
    }

    pub fn server_config(&self) -> Result<Arc<rustls::ServerConfig>, String> {
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(self.cert.clone())],
            PrivatePkcs8KeyDer::from(self.key.clone()).into(),
        )
        .map_err(|e| e.to_string())?;
        Ok(Arc::new(config))
    }
}

#[derive(Debug)]
struct PinVerifier(String);
impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        cert: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if fingerprint(cert.as_ref()) != self.0 {
            return Err(rustls::Error::General(
                "Peer certificate pin mismatch".into(),
            ));
        }
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            m,
            c,
            d,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            m,
            c,
            d,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub fn connect_pinned(addr: &str, pin: &str, identity: &Identity) -> Result<TlsStream, String> {
    if !valid_pin(pin) {
        return Err("Invalid certificate pin".into());
    }
    let address: std::net::SocketAddr = addr
        .parse()
        .map_err(|_| "Endpoint must be an IP address and port")?;
    let socket =
        TcpStream::connect_timeout(&address, Duration::from_secs(3)).map_err(|e| e.to_string())?;
    let _deadline = super::socket_deadline::SocketDeadline::new(&socket, Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    socket
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    let config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| e.to_string())?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(PinVerifier(pin.to_string())))
            .with_client_auth_cert(
                vec![CertificateDer::from(identity.cert.clone())],
                PrivatePkcs8KeyDer::from(identity.key.clone()).into(),
            )
            .map_err(|e| e.to_string())?;
    let mut stream = StreamOwned::new(
        ClientConnection::new(Arc::new(config), ServerName::IpAddress(address.ip().into()))
            .map_err(|e| e.to_string())?,
        socket,
    );
    while stream.conn.is_handshaking() {
        stream
            .conn
            .complete_io(&mut stream.sock)
            .map_err(|e| format!("TLS handshake failed: {e}"))?;
    }
    Ok(stream)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Peer {
    pub pin: String,
    pub token: String,
}

static PEERS: OnceLock<Mutex<HashMap<String, (Peer, Identity)>>> = OnceLock::new();

/// Called only after QR-secret verification and a pinned mutual TLS status probe.
pub fn remember_peer(
    addr: &str,
    peer: Peer,
    identity: Identity,
    persist: bool,
) -> Result<(), String> {
    if !valid_pin(&peer.pin)
        || peer.token.is_empty()
        || peer.token.len() > 128
        || !peer
            .token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("Invalid enrollment credentials".into());
    }
    if persist {
        keyring::Entry::new("app.camapro.scope", &format!("lan-peer:{addr}"))
            .map_err(|e| e.to_string())?
            .set_password(&serde_json::to_string(&peer).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Secure peer storage unavailable: {e}"))?;
    }
    PEERS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "Trust state poisoned")?
        .insert(addr.into(), (peer, identity));
    Ok(())
}

pub fn enrolled_peer(addr: &str) -> Result<(Peer, Identity), String> {
    if let Some(p) = PEERS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "Trust state poisoned")?
        .get(addr)
        .cloned()
    {
        return Ok(p);
    }
    let stored = keyring::Entry::new("app.camapro.scope", &format!("lan-peer:{addr}"))
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| "Phone endpoint is not enrolled; scan desktop QR first")?;
    let peer: Peer = serde_json::from_str(&stored).map_err(|_| "Invalid stored enrollment")?;
    Ok((peer, Identity::load()?))
}

pub fn connect(addr: &str, token: &str) -> Result<TlsStream, String> {
    let (peer, identity) = enrolled_peer(addr)?;
    if token != peer.token {
        return Err("Enrollment token mismatch".into());
    }
    connect_pinned(addr, &peer.pin, &identity)
}

pub fn status(addr: &str) -> Result<(), String> {
    let (peer, identity) = enrolled_peer(addr)?;
    probe(addr, &peer, &identity)
}

pub fn probe(addr: &str, peer: &Peer, identity: &Identity) -> Result<(), String> {
    probe_status(addr, peer, identity, false)
}

pub fn probe_ready(addr: &str, peer: &Peer, identity: &Identity) -> Result<(), String> {
    probe_status(addr, peer, identity, true)
}

fn probe_status(
    addr: &str,
    peer: &Peer,
    identity: &Identity,
    require_ready: bool,
) -> Result<(), String> {
    let mut stream = connect_pinned(addr, &peer.pin, identity)?;
    let _deadline =
        super::socket_deadline::SocketDeadline::new(&stream.sock, Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
    stream.write_all(format!("GET /status HTTP/1.1\r\nHost: {addr}\r\nX-Camapro-Token: {}\r\nConnection: close\r\n\r\n", peer.token).as_bytes()).map_err(|e| e.to_string())?;
    let mut response = Vec::new();
    // Bounded response headers; no dependency on TLS close_notify or response body.
    while !response.ends_with(b"\r\n\r\n") {
        if response.len() >= 8192 {
            return Err("Status headers too large".into());
        }
        let mut byte = [0];
        stream.read_exact(&mut byte).map_err(|e| e.to_string())?;
        response.push(byte[0]);
    }
    if !response.starts_with(b"HTTP/1.1 200 ") {
        return Err("Authenticated phone status probe failed".into());
    }
    if require_ready {
        let text = std::str::from_utf8(&response).map_err(|_| "Invalid status encoding")?;
        let ready: Vec<_> = text
            .lines()
            .filter_map(|line| line.split_once(':'))
            .filter(|(key, _)| key.eq_ignore_ascii_case("X-Camapro-Ready"))
            .map(|(_, value)| value.trim())
            .collect();
        if ready != ["true"] {
            return Err("Phone listener is not committed and ready".into());
        }
    }
    Ok(())
}
