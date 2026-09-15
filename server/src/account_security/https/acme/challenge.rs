use anyhow::Result;
use rcgen::{CertificateParams, CustomExtension, DistinguishedName, KeyPair};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::{net::TcpListener, task::JoinHandle, time::timeout};
use tokio_rustls::TlsAcceptor;

pub(super) struct ChallengeServer(JoinHandle<()>);
impl Drop for ChallengeServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}
impl ChallengeServer {
    pub async fn bind(listen: SocketAddr, ip: IpAddr, digest: &[u8]) -> Result<Self> {
        let listener = TcpListener::bind(listen).await?;
        let mut params = CertificateParams::new(vec![ip.to_string()])?;
        params.distinguished_name = DistinguishedName::new();
        params
            .custom_extensions
            .push(CustomExtension::new_acme_identifier(digest));
        let key = KeyPair::generate()?;
        let cert = params.self_signed(&key)?;
        let mut tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.der().clone()],
            rustls_pki_types::PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
        )?;
        tls.alpn_protocols = vec![b"acme-tls/1".to_vec()];
        let acceptor = TlsAcceptor::from(Arc::new(tls));
        Ok(Self(tokio::spawn(async move {
            // Verification only: no HTTP routes, tokens, or application payloads on this socket.
            let mut tasks = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    _ = tasks.join_next(), if !tasks.is_empty() => {},
                    result = listener.accept() => {
                        let Ok((stream, _)) = result else { break };
                        if tasks.len() >= 32 { continue; }
                        let a = acceptor.clone();
                        tasks.spawn(async move { let _ = timeout(Duration::from_secs(10), a.accept(stream)).await; });
                    }
                }
            }
        })))
    }
}
