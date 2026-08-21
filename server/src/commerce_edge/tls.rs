use std::{future::Future, sync::Arc};

use anyhow::{bail, Context, Result};
use axum::Router;
use hyper::server::conn::http1;
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use rustls::ServerConfig;
use rustls_pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use tokio::{net::TcpListener, time::timeout};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, info};

use super::config::EdgeConfig;

pub(crate) async fn serve(
    config: &EdgeConfig,
    app: Router,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<()> {
    let certificates = load_certificates(config.certificate_chain_path())?;
    let private_key = load_single_private_key(config.private_key_path())?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut server_config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(certificates, private_key)
        .context("COMMERCE_EDGE_TLS_IDENTITY_INVALID")?;
    server_config.alpn_protocols = vec![b"http/1.1".to_vec()];

    let listener = TcpListener::bind(config.listen_addr())
        .await
        .context("COMMERCE_EDGE_LISTEN_FAILED")?;
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let handshake_timeout = config.tls_handshake_timeout();
    let connection_timeout = config.connection_timeout();
    info!(listen_addr = %config.listen_addr(), "commerce edge TLS listener bound");

    tokio::pin!(shutdown);
    loop {
        let accepted = tokio::select! {
            _ = &mut shutdown => break,
            accepted = listener.accept() => accepted.context("COMMERCE_EDGE_ACCEPT_FAILED")?,
        };
        let (stream, peer) = accepted;
        let acceptor = acceptor.clone();
        let app = app.clone();
        tokio::spawn(async move {
            if let Err(error) =
                serve_connection(stream, acceptor, app, handshake_timeout, connection_timeout).await
            {
                debug!(peer = %peer.ip(), error = %error, "commerce edge connection rejected");
            }
        });
    }
    info!("commerce edge shutdown requested");
    Ok(())
}

async fn serve_connection(
    stream: tokio::net::TcpStream,
    acceptor: TlsAcceptor,
    app: Router,
    handshake_timeout: std::time::Duration,
    connection_timeout: std::time::Duration,
) -> Result<()> {
    stream.set_nodelay(true)?;
    let tls_stream = timeout(handshake_timeout, acceptor.accept(stream))
        .await
        .context("COMMERCE_EDGE_TLS_HANDSHAKE_TIMEOUT")?
        .context("COMMERCE_EDGE_TLS_HANDSHAKE_REJECTED")?;
    let service = TowerToHyperService::new(app);
    let io = TokioIo::new(tls_stream);
    let connection = http1::Builder::new().serve_connection(io, service);
    timeout(connection_timeout, connection)
        .await
        .context("COMMERCE_EDGE_CONNECTION_TIMEOUT")?
        .context("COMMERCE_EDGE_HTTP_FAILED")?;
    Ok(())
}

fn load_certificates(path: &std::path::Path) -> Result<Vec<CertificateDer<'static>>> {
    let certificates = CertificateDer::pem_file_iter(path)
        .context("COMMERCE_EDGE_CERTIFICATE_READ_FAILED")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("COMMERCE_EDGE_CERTIFICATE_PARSE_FAILED")?;
    if certificates.is_empty() {
        bail!("COMMERCE_EDGE_CERTIFICATE_MISSING");
    }
    Ok(certificates)
}

fn load_single_private_key(path: &std::path::Path) -> Result<PrivateKeyDer<'static>> {
    let mut keys =
        PrivateKeyDer::pem_file_iter(path).context("COMMERCE_EDGE_PRIVATE_KEY_READ_FAILED")?;
    let key = keys
        .next()
        .transpose()
        .context("COMMERCE_EDGE_PRIVATE_KEY_PARSE_FAILED")?
        .ok_or_else(|| anyhow::anyhow!("COMMERCE_EDGE_PRIVATE_KEY_MISSING"))?;
    if keys
        .next()
        .transpose()
        .context("COMMERCE_EDGE_PRIVATE_KEY_PARSE_FAILED")?
        .is_some()
    {
        bail!("COMMERCE_EDGE_PRIVATE_KEY_AMBIGUOUS");
    }
    Ok(key)
}
