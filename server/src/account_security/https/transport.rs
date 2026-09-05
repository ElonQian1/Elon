use anyhow::{Context, Result};
use axum::{extract::ConnectInfo, Extension, Router};
use hyper::server::conn::http1;
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use rustls::ServerConfig;
use rustls_pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use std::{sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    sync::Semaphore,
    task::JoinSet,
    time::{interval, timeout},
};
use tokio_rustls::TlsAcceptor;

use super::config::Config;

pub(super) struct Server {
    config: Config,
    listener: TcpListener,
    acceptor: TlsAcceptor,
}

fn load(config: &Config) -> Result<TlsAcceptor> {
    let chain =
        CertificateDer::pem_file_iter(&config.certificate)?.collect::<Result<Vec<_>, _>>()?;
    let key = PrivateKeyDer::from_pem_file(&config.key)?;
    let mut tls =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(chain, key)?;
    tls.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(TlsAcceptor::from(Arc::new(tls)))
}

impl Server {
    pub async fn bind(config: Config) -> Result<Self> {
        let acceptor = load(&config).context("ACCOUNT_HTTPS_CERTIFICATE_INVALID")?;
        let listener = TcpListener::bind(config.listen)
            .await
            .context("ACCOUNT_HTTPS_BIND_FAILED")?;
        Ok(Self {
            config,
            listener,
            acceptor,
        })
    }

    pub async fn serve(mut self, app: Router) -> Result<()> {
        let slots = Arc::new(Semaphore::new(128));
        let mut tasks = JoinSet::new();
        let mut refresh = interval(Duration::from_secs(60));
        refresh.tick().await;
        tracing::info!(listen=%self.config.listen,"account-only native HTTPS listening");
        loop {
            tokio::select! {
                _=refresh.tick() => {
                    // Invalid renewal material never replaces the last working keypair.
                    match load(&self.config) {
                        Ok(acceptor) => self.acceptor=acceptor,
                        Err(_) => tracing::warn!("ACCOUNT_HTTPS_RELOAD_REJECTED"),
                    }
                },
                _=tasks.join_next(), if !tasks.is_empty() => {},
                accepted=self.listener.accept() => {
                    let (stream,peer)=accepted?;
                    let Ok(permit)=slots.clone().try_acquire_owned() else {continue};
                    let acceptor=self.acceptor.clone();
                    let app=app.clone().layer(Extension(ConnectInfo(peer)));
                    tasks.spawn(async move {
                        let _permit=permit;
                        let Ok(Ok(tls))=timeout(Duration::from_secs(10),acceptor.accept(stream)).await else {return};
                        let service=TowerToHyperService::new(app);
                        let mut builder=http1::Builder::new();
                        builder.max_buf_size(16 * 1024);
                        let _=timeout(Duration::from_secs(60),builder.serve_connection(TokioIo::new(tls),service)).await;
                    });
                },
            }
        }
    }
}
