use super::*;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls_pki_types::{CertificateDer, ServerName, UnixTime};

// Model the CA's challenge checks, including reverse-IP SNI (RFC 8738).
#[derive(Debug)]
struct ChallengeVerifier;
impl ServerCertVerifier for ChallengeVerifier {
    fn verify_server_cert(
        &self,
        cert: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        let (_, cert) = x509_parser::parse_x509_certificate(cert.as_ref()).unwrap();
        let san = cert.subject_alternative_name().unwrap().unwrap();
        assert_eq!(san.value.general_names.len(), 1);
        assert!(
            matches!(san.value.general_names[0], x509_parser::extensions::GeneralName::IPAddress(v) if v == [43, 139, 149, 158])
        );
        let ext = cert
            .extensions()
            .iter()
            .find(|e| e.oid.to_id_string() == "1.3.6.1.5.5.7.1.31")
            .unwrap();
        assert!(ext.critical);
        assert_eq!(ext.value, [&[4, 32][..], &[42; 32]].concat());
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_signature(message, cert, dss)
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_signature(message, cert, dss)
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn verify_signature(
    message: &[u8],
    cert: &CertificateDer<'_>,
    dss: &rustls::DigitallySignedStruct,
) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
    let (_, cert) = x509_parser::parse_x509_certificate(cert.as_ref()).unwrap();
    let algorithms = rustls::crypto::ring::default_provider().signature_verification_algorithms;
    let valid = algorithms
        .mapping
        .iter()
        .filter(|(scheme, _)| *scheme == dss.scheme)
        .flat_map(|(_, algorithms)| algorithms.iter())
        .any(|algorithm| {
            algorithm
                .verify_signature(
                    cert.public_key().subject_public_key.data.as_ref(),
                    message,
                    dss.signature(),
                )
                .is_ok()
        });
    if valid {
        Ok(HandshakeSignatureValid::assertion())
    } else {
        Err(rustls::Error::General(
            "challenge handshake signature mismatch".into(),
        ))
    }
}

#[tokio::test]
async fn serves_ip_challenge_over_real_tls_and_releases_port() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server =
        ChallengeServer::serve(listener, "43.139.149.158".parse().unwrap(), &[42; 32]).unwrap();
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(ChallengeVerifier))
    .with_no_client_auth();
    config.alpn_protocols = vec![b"acme-tls/1".to_vec()];
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    for _ in 0..3 {
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let tls = timeout(
            Duration::from_secs(3),
            connector.connect(
                ServerName::try_from("158.149.139.43.in-addr.arpa").unwrap(),
                stream,
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(tls.get_ref().1.alpn_protocol(), Some(&b"acme-tls/1"[..]));
    }
    drop(server);
    // Drop aborts the listener task; await scheduling instead of timing sleeps.
    tokio::task::yield_now().await;
    assert!(TcpListener::bind(addr).await.is_ok());
}
