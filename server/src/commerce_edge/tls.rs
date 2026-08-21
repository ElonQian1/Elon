use std::{future::Future, sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use axum::Router;
use hyper::server::conn::http1;
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use rustls::ServerConfig;
use rustls_pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    task::JoinSet,
    time::{timeout, timeout_at, Instant},
};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, info, warn};

use super::{certificate_config::CertificateProviderConfig, config::EdgeConfig};

const CONNECTION_DRAIN_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) async fn serve(
    config: &EdgeConfig,
    app: Router,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<()> {
    match config.certificate_provider() {
        CertificateProviderConfig::Pem { .. } => serve_pem(config, app, shutdown).await,
        CertificateProviderConfig::AcmeTlsAlpn01 { .. } => {
            super::acme::serve(config, app, shutdown).await
        }
    }
}

pub(crate) fn validate_provider_material(config: &EdgeConfig) -> Result<()> {
    if let Some((certificate_path, private_key_path)) = config.certificate_provider().pem_paths() {
        let certificates = load_certificates(certificate_path)?;
        let private_key = load_single_private_key(private_key_path)?;
        build_pem_server_config(certificates, private_key)?;
    }
    Ok(())
}

async fn serve_pem(
    config: &EdgeConfig,
    app: Router,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<()> {
    let (certificate_path, private_key_path) = config
        .certificate_provider()
        .pem_paths()
        .expect("PEM server requires PEM provider");
    let server_config = build_pem_server_config(
        load_certificates(certificate_path)?,
        load_single_private_key(private_key_path)?,
    )?;
    let listener = TcpListener::bind(config.listen_addr())
        .await
        .context("COMMERCE_EDGE_LISTEN_FAILED")?;
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let handshake_timeout = config.tls_handshake_timeout();
    let connection_timeout = config.connection_timeout();
    let mut connections = JoinSet::new();
    info!(listen_addr = %config.listen_addr(), "commerce edge TLS listener bound");

    tokio::pin!(shutdown);
    let serve_result = loop {
        tokio::select! {
            _ = &mut shutdown => break Ok(()),
            joined = connections.join_next(), if !connections.is_empty() => {
                log_connection_completion(joined);
            }
            accepted = listener.accept() => {
                let (stream, peer) = match accepted {
                    Ok(accepted) => accepted,
                    Err(error) => break Err(error).context("COMMERCE_EDGE_ACCEPT_FAILED"),
                };
                let acceptor = acceptor.clone();
                let app = app.clone();
                connections.spawn(async move {
                    if let Err(error) = serve_pem_connection(
                        stream,
                        acceptor,
                        app,
                        handshake_timeout,
                        connection_timeout,
                    ).await {
                        debug!(peer = %peer.ip(), error = %error, "commerce edge connection rejected");
                    }
                });
            }
        }
    };
    info!(
        active_connections = connections.len(),
        "commerce edge shutdown requested"
    );
    drain_connections(&mut connections).await;
    serve_result
}

async fn serve_pem_connection(
    stream: TcpStream,
    acceptor: TlsAcceptor,
    app: Router,
    handshake_timeout: Duration,
    connection_timeout: Duration,
) -> Result<()> {
    stream.set_nodelay(true)?;
    let tls_stream = timeout(handshake_timeout, acceptor.accept(stream))
        .await
        .context("COMMERCE_EDGE_TLS_HANDSHAKE_TIMEOUT")?
        .context("COMMERCE_EDGE_TLS_HANDSHAKE_REJECTED")?;
    serve_http_connection(tls_stream, app, connection_timeout).await
}

pub(super) async fn serve_http_connection<T>(
    stream: T,
    app: Router,
    connection_timeout: Duration,
) -> Result<()>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    let service = TowerToHyperService::new(app);
    let connection = http1::Builder::new().serve_connection(TokioIo::new(stream), service);
    timeout(connection_timeout, connection)
        .await
        .context("COMMERCE_EDGE_CONNECTION_TIMEOUT")?
        .context("COMMERCE_EDGE_HTTP_FAILED")?;
    Ok(())
}

pub(super) async fn drain_connections(connections: &mut JoinSet<()>) {
    let deadline = Instant::now() + CONNECTION_DRAIN_TIMEOUT;
    while !connections.is_empty() {
        match timeout_at(deadline, connections.join_next()).await {
            Ok(joined) => log_connection_completion(joined),
            Err(_) => {
                let remaining = connections.len();
                connections.abort_all();
                while connections.join_next().await.is_some() {}
                warn!(remaining, "commerce edge connection drain deadline reached");
                return;
            }
        }
    }
    info!("commerce edge active connections drained");
}

fn log_connection_completion(joined: Option<Result<(), tokio::task::JoinError>>) {
    if let Some(Err(error)) = joined {
        debug!(error = %error, "commerce edge connection task ended unexpectedly");
    }
}

fn build_pem_server_config(
    certificates: Vec<CertificateDer<'static>>,
    private_key: PrivateKeyDer<'static>,
) -> Result<ServerConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut server_config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(certificates, private_key)
        .context("COMMERCE_EDGE_TLS_IDENTITY_INVALID")?;
    server_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(server_config)
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
