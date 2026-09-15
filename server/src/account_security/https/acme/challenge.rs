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
        Self::serve(listener, ip, digest)
    }

    fn serve(listener: TcpListener, ip: IpAddr, digest: &[u8]) -> Result<Self> {
        let mut params = CertificateParams::new(vec![ip.to_string()])?;
        params.distinguished_name = DistinguishedName::new();
        params
            .custom_extensions
            .push(CustomExtension::new_acme_identifier(digest));
        let key = KeyPair::generate()?;
        let cert = params.self_signed(&key)?;
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let signing_key = provider.key_provider.load_private_key(
            rustls_pki_types::PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
        )?;
        // ACME requires a critical extension that the normal WebPKI certificate
        // loader rejects. This ephemeral cert was just signed by this same key;
        // use the challenge resolver without applying business-certificate rules.
        anyhow::ensure!(
            signing_key
                .public_key()
                .is_some_and(|spki| spki.as_ref() == key.public_key_der()),
            "challenge signing key mismatch"
        );
        let certified = rustls::sign::CertifiedKey::new(vec![cert.der().clone()], signing_key);
        let resolver = rustls::sign::SingleCertAndKey::from(certified);
        let mut tls = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(resolver));
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

#[cfg(test)]
#[path = "challenge_tests.rs"]
mod tests;
