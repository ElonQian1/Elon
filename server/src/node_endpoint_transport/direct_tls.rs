use std::{net::IpAddr, sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use hyper::server::conn::http1;
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use rustls::ServerConfig;
use rustls_pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};
use tokio::{net::TcpListener, time::timeout};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, info};
use uuid::Uuid;

use crate::node_compute_sharing::endpoint_authority::{
    canonical_direct_tls_verifier_digest, seal_direct_tls_connection,
};
use crate::types::AppState;

use super::{
    config::DirectTlsTransportConfig, evidence_slot::VerifiedSecureTransportSlot, secure_router,
};

const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const SECURE_HTTP_CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);

/// Unforgeable within the crate: construction is private to the module that creates the exact
/// rustls ServerConfig and TlsAcceptor. Domain code receives only read accessors.
pub(crate) struct DirectTlsVerifierSeal {
    server_instance_id: String,
    listener_id: String,
    verifier_revision: u64,
    verifier_digest: String,
    leaf_certificate_digest: String,
}

/// Direct socket peer used only for process-local abuse controls. It is not transport authority
/// and never enters endpoint credential or session receipts.
#[derive(Clone, Copy)]
pub(super) struct DirectTlsPeerAddress(IpAddr);

impl DirectTlsPeerAddress {
    fn from_socket(address: std::net::SocketAddr) -> Self {
        let address = match address.ip() {
            IpAddr::V6(value) => value
                .to_ipv4_mapped()
                .map(IpAddr::V4)
                .unwrap_or(IpAddr::V6(value)),
            value => value,
        };
        Self(address)
    }

    pub(super) fn rate_limit_key(self) -> String {
        self.0.to_string()
    }
}

impl DirectTlsVerifierSeal {
    fn from_loaded_server_config(
        server_instance_id: String,
        listener_id: String,
        verifier_revision: u64,
        leaf_certificate_digest: String,
    ) -> Result<Self> {
        let verifier_digest = canonical_direct_tls_verifier_digest(
            &server_instance_id,
            &listener_id,
            verifier_revision,
            &leaf_certificate_digest,
        )?;
        Ok(Self {
            server_instance_id,
            listener_id,
            verifier_revision,
            verifier_digest,
            leaf_certificate_digest,
        })
    }

    pub(crate) fn server_instance_id(&self) -> &str {
        &self.server_instance_id
    }
    pub(crate) fn listener_id(&self) -> &str {
        &self.listener_id
    }
    pub(crate) fn verifier_revision(&self) -> u64 {
        self.verifier_revision
    }
    pub(crate) fn verifier_digest(&self) -> &str {
        &self.verifier_digest
    }
    pub(crate) fn leaf_certificate_digest(&self) -> &str {
        &self.leaf_certificate_digest
    }
}

pub(super) struct DirectTlsServer {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    verifier: Arc<DirectTlsVerifierSeal>,
    owner_credential_api_enabled: bool,
    owner_bootstrap_api_enabled: bool,
}

impl DirectTlsServer {
    pub(super) async fn bind(config: DirectTlsTransportConfig) -> Result<Self> {
        let certificates = load_certificates(&config.certificate_chain_path)?;
        let private_key = load_single_private_key(&config.private_key_path)?;
        let leaf_certificate_digest = hex::encode(Sha256::digest(
            certificates
                .first()
                .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_DIRECT_TLS_CERTIFICATE_MISSING"))?
                .as_ref(),
        ));
        let server_instance_id = format!("nesrv_{}", Uuid::new_v4().simple());
        let listener_id = format!("node_endpoint_direct_tls:{}", config.listen_addr);
        let verifier = Arc::new(DirectTlsVerifierSeal::from_loaded_server_config(
            server_instance_id,
            listener_id,
            config.verifier_revision,
            leaf_certificate_digest,
        )?);

        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut server_config = ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(certificates, private_key)?;
        server_config.alpn_protocols = vec![b"http/1.1".to_vec()];

        let listener = TcpListener::bind(config.listen_addr).await?;
        info!(
            listen_addr = %config.listen_addr,
            verifier_revision = config.verifier_revision,
            "node endpoint direct TLS evidence listener bound"
        );
        Ok(Self {
            listener,
            acceptor: TlsAcceptor::from(Arc::new(server_config)),
            verifier,
            owner_credential_api_enabled: config.owner_credential_api_enabled,
            owner_bootstrap_api_enabled: config.owner_bootstrap_api_enabled,
        })
    }

    pub(super) async fn serve(self, state: Arc<AppState>) -> Result<()> {
        loop {
            let (stream, peer_address) = self.listener.accept().await?;
            let acceptor = self.acceptor.clone();
            let verifier = Arc::clone(&self.verifier);
            let state = Arc::clone(&state);
            let owner_credential_api_enabled = self.owner_credential_api_enabled;
            let owner_bootstrap_api_enabled = self.owner_bootstrap_api_enabled;
            tokio::spawn(async move {
                if let Err(error) = serve_connection(
                    stream,
                    acceptor,
                    verifier,
                    state,
                    owner_credential_api_enabled,
                    owner_bootstrap_api_enabled,
                    DirectTlsPeerAddress::from_socket(peer_address),
                )
                .await
                {
                    debug!(%error, "node endpoint direct TLS connection rejected");
                }
            });
        }
    }
}

async fn serve_connection(
    stream: tokio::net::TcpStream,
    acceptor: TlsAcceptor,
    verifier: Arc<DirectTlsVerifierSeal>,
    state: Arc<AppState>,
    owner_credential_api_enabled: bool,
    owner_bootstrap_api_enabled: bool,
    peer_address: DirectTlsPeerAddress,
) -> Result<()> {
    stream.set_nodelay(true)?;
    let tls_stream = timeout(TLS_HANDSHAKE_TIMEOUT, acceptor.accept(stream))
        .await
        .context("NODE_ENDPOINT_DIRECT_TLS_HANDSHAKE_TIMEOUT")??;
    let (_, connection) = tls_stream.get_ref();
    let evidence = seal_direct_tls_connection(connection, &verifier, Utc::now())?;
    let app = secure_router::build(
        VerifiedSecureTransportSlot::new(evidence),
        state,
        owner_credential_api_enabled,
        owner_bootstrap_api_enabled,
        peer_address,
    );
    let service = TowerToHyperService::new(app);
    let io = TokioIo::new(tls_stream);
    let mut builder = http1::Builder::new();
    builder.keep_alive(false);
    let connection = builder.serve_connection(io, service);
    timeout(SECURE_HTTP_CONNECTION_TIMEOUT, connection)
        .await
        .context("NODE_ENDPOINT_DIRECT_TLS_HTTP_TIMEOUT")??;
    Ok(())
}

fn load_certificates(path: &std::path::Path) -> Result<Vec<CertificateDer<'static>>> {
    let certificates = CertificateDer::pem_file_iter(path)
        .with_context(|| "NODE_ENDPOINT_DIRECT_TLS_CERTIFICATE_READ_FAILED")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| "NODE_ENDPOINT_DIRECT_TLS_CERTIFICATE_PARSE_FAILED")?;
    if certificates.is_empty() {
        bail!("NODE_ENDPOINT_DIRECT_TLS_CERTIFICATE_MISSING");
    }
    Ok(certificates)
}

fn load_single_private_key(path: &std::path::Path) -> Result<PrivateKeyDer<'static>> {
    let mut keys = PrivateKeyDer::pem_file_iter(path)
        .with_context(|| "NODE_ENDPOINT_DIRECT_TLS_PRIVATE_KEY_READ_FAILED")?;
    let key = keys
        .next()
        .transpose()
        .with_context(|| "NODE_ENDPOINT_DIRECT_TLS_PRIVATE_KEY_PARSE_FAILED")?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_DIRECT_TLS_PRIVATE_KEY_MISSING"))?;
    if keys
        .next()
        .transpose()
        .with_context(|| "NODE_ENDPOINT_DIRECT_TLS_PRIVATE_KEY_PARSE_FAILED")?
        .is_some()
    {
        bail!("NODE_ENDPOINT_DIRECT_TLS_PRIVATE_KEY_AMBIGUOUS");
    }
    Ok(key)
}
