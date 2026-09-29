use super::*;
use axum::{
    extract::ws::{Message, WebSocketUpgrade},
    routing::get,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::TlsConnector;

#[tokio::test]
async fn tls_connection_supports_websocket_upgrade() {
    let directory = std::env::temp_dir().join(format!("pwa-tls-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let certificate = directory.join("certificate.pem");
    let key = directory.join("key.pem");
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    std::fs::write(&certificate, generated.cert.pem()).unwrap();
    std::fs::write(&key, generated.key_pair.serialize_pem()).unwrap();
    let server = Server::bind(Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        certificate: certificate.clone(),
        key: key.clone(),
    })
    .await
    .unwrap();
    let address = server.listener.local_addr().unwrap();
    let app = Router::new().route(
        "/ws",
        get(|upgrade: WebSocketUpgrade| async move {
            upgrade.on_upgrade(|mut socket| async move {
                socket
                    .send(Message::Text("pwa-upgrade-ready".into()))
                    .await
                    .unwrap();
            })
        }),
    );
    let running = tokio::spawn(server.serve(app));
    let mut roots = rustls::RootCertStore::empty();
    roots.add(generated.cert.der().clone()).unwrap();
    let client = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(client));
    let tcp = tokio::net::TcpStream::connect(address).await.unwrap();
    let mut stream = connector
        .connect("localhost".try_into().unwrap(), tcp)
        .await
        .unwrap();
    stream.write_all(b"GET /ws HTTP/1.1\r\nHost: localhost\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").await.unwrap();
    let result = timeout(Duration::from_secs(5), async {
        let mut bytes = Vec::new();
        loop {
            let mut buffer = [0; 1024];
            let count = stream.read(&mut buffer).await.unwrap();
            bytes.extend_from_slice(&buffer[..count]);
            if count == 0 || String::from_utf8_lossy(&bytes).contains("pwa-upgrade-ready") {
                return bytes;
            }
            assert!(bytes.len() < 4096);
        }
    })
    .await;
    running.abort();
    let _ = running.await;
    std::fs::remove_file(certificate).unwrap();
    std::fs::remove_file(key).unwrap();
    std::fs::remove_dir(directory).unwrap();
    let bytes = result.unwrap();
    let response = String::from_utf8_lossy(&bytes);
    assert!(response.starts_with("HTTP/1.1 101"), "{response}");
    assert!(response.contains("pwa-upgrade-ready"));
}
