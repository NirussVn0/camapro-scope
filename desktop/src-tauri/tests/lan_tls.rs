use camapro_scope_lib::core::control::lan_tls::{self, Identity, Peer};
use camapro_scope_lib::core::media::stream_client::StreamClient;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn tls_server(identity: &Identity) -> (String, std::thread::JoinHandle<bool>) {
    tls_server_with_header(identity, "")
}

fn tls_server_with_header(
    identity: &Identity,
    header: &'static str,
) -> (String, std::thread::JoinHandle<bool>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let config = identity.server_config().unwrap();
    let worker = std::thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut tls =
            rustls::StreamOwned::new(rustls::ServerConnection::new(config).unwrap(), socket);
        let mut byte = [0];
        let mut request = Vec::new();
        loop {
            match tls.read(&mut byte) {
                Ok(1) => request.push(byte[0]),
                _ => return false,
            }
            if request.ends_with(b"\r\n\r\n") {
                break;
            }
            if request.len() > 8192 {
                return false;
            }
        }
        let token = "ab".repeat(32);
        assert!(String::from_utf8_lossy(&request).contains(&format!("X-Camapro-Token: {token}")));
        tls.write_all(format!("HTTP/1.1 200 OK\r\n{header}Content-Length: 0\r\n\r\n").as_bytes())
            .unwrap();
        tls.conn.send_close_notify();
        tls.flush().unwrap();
        true
    });
    (addr, worker)
}

#[test]
fn final_probe_rejects_provisional_missing_and_duplicate_readiness() {
    let phone = Identity::generate().unwrap();
    let desktop = Identity::generate().unwrap();
    let peer = Peer {
        pin: lan_tls::fingerprint(&phone.cert),
        token: "ab".repeat(32),
    };
    for (header, accepted) in [
        ("", false),
        ("X-Camapro-Ready: false\r\n", false),
        ("X-Camapro-Ready: true\r\n", true),
        ("X-Camapro-Ready: true\r\nX-Camapro-Ready: false\r\n", false),
    ] {
        let (addr, worker) = tls_server_with_header(&phone, header);
        assert_eq!(
            lan_tls::probe_ready(&addr, &peer, &desktop).is_ok(),
            accepted
        );
        assert!(worker.join().unwrap());
    }
}

#[test]
fn pinned_probe_checks_real_certificate_before_sending_credentials() {
    let server = Identity::generate().unwrap();
    let client = Identity::generate().unwrap();
    let (addr, worker) = tls_server(&server);
    let peer = Peer {
        pin: lan_tls::fingerprint(&server.cert),
        token: "ab".repeat(32),
    };
    assert!(lan_tls::probe(&addr, &peer, &client).is_ok());
    assert!(worker.join().unwrap());

    let (addr, worker) = tls_server(&server);
    let wrong = Peer {
        pin: lan_tls::fingerprint(&client.cert),
        ..peer
    };
    assert!(lan_tls::probe(&addr, &wrong, &client).is_err());
    assert!(
        !worker.join().unwrap(),
        "Wrong certificate must never receive HTTP credentials"
    );
}

#[test]
fn plaintext_and_tls12_are_rejected_without_http_fallback() {
    let server = Identity::generate().unwrap();
    let (addr, worker) = tls_server(&server);
    let mut socket = TcpStream::connect(&addr).unwrap();
    socket
        .write_all(b"GET /pair HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    drop(socket);
    assert!(!worker.join().unwrap());

    let (addr, worker) = tls_server(&server);
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(server.cert.clone()))
        .unwrap();
    let config = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS12])
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    let socket = TcpStream::connect(&addr).unwrap();
    let mut stream = rustls::StreamOwned::new(
        rustls::ClientConnection::new(
            std::sync::Arc::new(config),
            rustls::pki_types::ServerName::try_from("camapro.local").unwrap(),
        )
        .unwrap(),
        socket,
    );
    assert!(stream.write_all(b"GET /status HTTP/1.1\r\n\r\n").is_err());
    assert!(!worker.join().unwrap());

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let worker = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut buf = [0; 4096];
        let n = socket.read(&mut buf).unwrap();
        assert!(!buf[..n].windows(4).any(|s| s == b"GET "));
        socket.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
    });
    assert!(lan_tls::connect_pinned(&addr, &lan_tls::fingerprint(&server.cert), &server).is_err());
    worker.join().unwrap();
}

#[test]
fn unenrolled_endpoint_and_changed_token_fail_closed() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    assert!(StreamClient::start(addr.clone(), "ab".repeat(32)).is_err());
    let server = Identity::generate().unwrap();
    lan_tls::remember_peer(
        &addr,
        Peer {
            pin: lan_tls::fingerprint(&server.cert),
            token: "ab".repeat(32),
        },
        Identity::generate().unwrap(),
        false,
    )
    .unwrap();
    assert!(lan_tls::connect(&addr, &"cd".repeat(32)).is_err());
}

#[test]
fn random_secrets_are_256_bit_and_unique_and_pin_is_exact_der_digest() {
    let a = lan_tls::random_secret().unwrap();
    let b = lan_tls::random_secret().unwrap();
    assert_eq!(a.len(), 64);
    assert_ne!(a, b);
    assert!(a.bytes().all(|v| v.is_ascii_hexdigit()));
    assert_eq!(
        lan_tls::fingerprint(b"abc"),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn mutual_tls_accepts_enrolled_client_and_rejects_different_identity() {
    use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
    use std::sync::Arc;
    let server = Identity::generate().unwrap();
    let client = Identity::generate().unwrap();
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from(client.cert.clone()))
        .unwrap();
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots),
        Arc::new(rustls::crypto::ring::default_provider()),
    )
    .build()
    .unwrap();
    // Test-only construction from the identity serialization used by secure storage.
    let key: Vec<u8> =
        serde_json::from_value(serde_json::to_value(&server).unwrap()["key"].clone()).unwrap();
    let config = Arc::new(
        rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![CertificateDer::from(server.cert.clone())],
            PrivatePkcs8KeyDer::from(key).into(),
        )
        .unwrap(),
    );
    for (identity, accepted) in [
        (Some(client), true),
        (Some(Identity::generate().unwrap()), false),
        (None, false),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let config = Arc::clone(&config);
        let worker = std::thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut stream =
                rustls::StreamOwned::new(rustls::ServerConnection::new(config).unwrap(), socket);
            let mut byte = [0];
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                if stream.read_exact(&mut byte).is_err() {
                    return false;
                }
                head.push(byte[0]);
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
            true
        });
        let peer = Peer {
            pin: lan_tls::fingerprint(&server.cert),
            token: "ab".repeat(32),
        };
        let success = if let Some(identity) = identity {
            lan_tls::probe(&addr, &peer, &identity).is_ok()
        } else {
            let mut roots = rustls::RootCertStore::empty();
            roots
                .add(CertificateDer::from(server.cert.clone()))
                .unwrap();
            let config = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth();
            let socket = TcpStream::connect(&addr).unwrap();
            let mut stream = rustls::StreamOwned::new(
                rustls::ClientConnection::new(
                    Arc::new(config),
                    rustls::pki_types::ServerName::try_from("camapro.local").unwrap(),
                )
                .unwrap(),
                socket,
            );
            stream.write_all(b"GET /status HTTP/1.1\r\n\r\n").is_ok()
                && stream.read_exact(&mut [0]).is_ok()
        };
        assert_eq!(success, accepted);
        assert_eq!(worker.join().unwrap(), accepted);
    }
}
