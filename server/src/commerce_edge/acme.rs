use std::{collections::HashSet, future::Future, path::Path, sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use axum::Router;
use futures::StreamExt;
use rustls::ServerConfig;
use rustls_acme::{caches::DirCache, AcmeConfig};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    task::JoinSet,
    time::{timeout_at, Instant},
};
use tokio_rustls::LazyConfigAcceptor;
use tracing::{debug, info, warn};

use super::{config::EdgeConfig, tls};

pub(super) async fn serve(
    config: &EdgeConfig,
    app: Router,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<()> {
    let settings = config
        .certificate_provider()
        .acme()
        .expect("ACME server requires ACME provider");
    prepare_cache_dir(settings.cache_dir)?;

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut acme_state = AcmeConfig::new_with_provider(settings.domains.to_vec(), provider.clone())
        .contact_push(settings.contact.to_owned())
        .cache(DirCache::new(settings.cache_dir.to_path_buf()))
        .directory_lets_encrypt(settings.environment.is_production())
        .state();
    let resolver = acme_state.resolver();
    let challenge_config = acme_state.challenge_rustls_config_with_provider(provider.clone());
    let normal_config = normal_server_config(provider, resolver)?;
    let allowed_domains = Arc::new(settings.domains.iter().cloned().collect::<HashSet<_>>());

    let listener = TcpListener::bind(config.listen_addr())
        .await
        .context("COMMERCE_EDGE_LISTEN_FAILED")?;
    let mut acme_task = tokio::spawn(async move {
        while let Some(event) = acme_state.next().await {
            match event {
                Ok(event) => info!(?event, "commerce edge ACME event"),
                Err(error) => warn!(error = %error, "commerce edge ACME event failed"),
            }
        }
        Err::<(), anyhow::Error>(anyhow::anyhow!("COMMERCE_EDGE_ACME_STATE_ENDED"))
    });
    let mut connections = JoinSet::new();
    let handshake_timeout = config.tls_handshake_timeout();
    let connection_timeout = config.connection_timeout();
    info!(listen_addr = %config.listen_addr(), production = settings.environment.is_production(), "commerce edge ACME TLS listener bound");

    tokio::pin!(shutdown);
    let mut acme_task_finished = false;
    let serve_result = loop {
        tokio::select! {
            _ = &mut shutdown => break Ok(()),
            state_result = &mut acme_task => {
                acme_task_finished = true;
                break match state_result {
                    Ok(result) => result,
                    Err(error) => Err(error).context("COMMERCE_EDGE_ACME_TASK_FAILED"),
                };
            }
            joined = connections.join_next(), if !connections.is_empty() => {
                log_connection_completion(joined);
            }
            accepted = listener.accept() => {
                let (stream, peer) = match accepted {
                    Ok(accepted) => accepted,
                    Err(error) => break Err(error).context("COMMERCE_EDGE_ACCEPT_FAILED"),
                };
                let challenge_config = Arc::clone(&challenge_config);
                let normal_config = Arc::clone(&normal_config);
                let allowed_domains = Arc::clone(&allowed_domains);
                let app = app.clone();
                connections.spawn(async move {
                    if let Err(error) = serve_acme_connection(
                        stream,
                        challenge_config,
                        normal_config,
                        allowed_domains,
                        app,
                        handshake_timeout,
                        connection_timeout,
                    ).await {
                        debug!(peer = %peer.ip(), error = %error, "commerce edge ACME connection rejected");
                    }
                });
            }
        }
    };

    if !acme_task_finished {
        acme_task.abort();
        let _ = acme_task.await;
    }
    info!(
        active_connections = connections.len(),
        "commerce edge ACME shutdown requested"
    );
    tls::drain_connections(&mut connections).await;
    serve_result
}

async fn serve_acme_connection(
    stream: TcpStream,
    challenge_config: Arc<ServerConfig>,
    normal_config: Arc<ServerConfig>,
    allowed_domains: Arc<HashSet<String>>,
    app: Router,
    handshake_timeout: Duration,
    connection_timeout: Duration,
) -> Result<()> {
    stream.set_nodelay(true)?;
    let deadline = Instant::now() + handshake_timeout;
    let start = timeout_at(
        deadline,
        LazyConfigAcceptor::new(Default::default(), stream),
    )
    .await
    .context("COMMERCE_EDGE_TLS_CLIENT_HELLO_TIMEOUT")?
    .context("COMMERCE_EDGE_TLS_CLIENT_HELLO_REJECTED")?;
    let client_hello = start.client_hello();
    let server_name = client_hello
        .server_name()
        .ok_or_else(|| anyhow::anyhow!("COMMERCE_EDGE_TLS_SNI_REQUIRED"))?;
    let is_challenge = rustls_acme::is_tls_alpn_challenge(&client_hello);
    let connection_kind = classify_connection(server_name, is_challenge, &allowed_domains)?;
    let selected_config = match connection_kind {
        ConnectionKind::AcmeChallenge => challenge_config,
        ConnectionKind::Application => normal_config,
    };
    let mut tls_stream = timeout_at(deadline, start.into_stream(selected_config))
        .await
        .context("COMMERCE_EDGE_TLS_HANDSHAKE_TIMEOUT")?
        .context("COMMERCE_EDGE_TLS_HANDSHAKE_REJECTED")?;

    if connection_kind == ConnectionKind::AcmeChallenge {
        tls_stream
            .shutdown()
            .await
            .context("COMMERCE_EDGE_ACME_CHALLENGE_CLOSE_FAILED")?;
        return Ok(());
    }
    tls::serve_http_connection(tls_stream, app, connection_timeout).await
}

fn normal_server_config(
    provider: Arc<rustls::crypto::CryptoProvider>,
    resolver: Arc<rustls_acme::ResolvesServerCertAcme>,
) -> Result<Arc<ServerConfig>> {
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_cert_resolver(resolver);
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConnectionKind {
    AcmeChallenge,
    Application,
}

fn classify_connection(
    server_name: &str,
    is_challenge: bool,
    allowed_domains: &HashSet<String>,
) -> Result<ConnectionKind> {
    if !allowed_domains
        .iter()
        .any(|domain| domain.eq_ignore_ascii_case(server_name))
    {
        bail!("COMMERCE_EDGE_TLS_SNI_NOT_ALLOWED");
    }
    Ok(if is_challenge {
        ConnectionKind::AcmeChallenge
    } else {
        ConnectionKind::Application
    })
}

fn prepare_cache_dir(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                bail!("COMMERCE_EDGE_ACME_CACHE_SYMLINK_FORBIDDEN");
            }
            if !metadata.is_dir() {
                bail!("COMMERCE_EDGE_ACME_CACHE_NOT_DIRECTORY");
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(path).context("COMMERCE_EDGE_ACME_CACHE_CREATE_FAILED")?;
        }
        Err(error) => return Err(error).context("COMMERCE_EDGE_ACME_CACHE_INSPECT_FAILED"),
    }
    set_owner_only_permissions(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .context("COMMERCE_EDGE_ACME_CACHE_PERMISSION_FAILED")
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn log_connection_completion(joined: Option<Result<(), tokio::task::JoinError>>) {
    if let Some(Err(error)) = joined {
        debug!(error = %error, "commerce edge ACME connection task ended unexpectedly");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_classification_rejects_unknown_sni_and_separates_challenges() {
        let domains = HashSet::from(["commerce.example.com".to_owned()]);
        assert_eq!(
            classify_connection("commerce.example.com", false, &domains).unwrap(),
            ConnectionKind::Application
        );
        assert_eq!(
            classify_connection("commerce.example.com", true, &domains).unwrap(),
            ConnectionKind::AcmeChallenge
        );
        assert_eq!(
            classify_connection("Commerce.Example.com", false, &domains).unwrap(),
            ConnectionKind::Application
        );
        assert!(classify_connection("other.example.com", false, &domains).is_err());
    }
}
