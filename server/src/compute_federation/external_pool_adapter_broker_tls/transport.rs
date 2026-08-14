use std::{net::SocketAddr, sync::Arc, time::Instant};

use anyhow::{anyhow, bail, Context, Result};
use ring::constant_time;
use rsa::pkcs8::der::{asn1::AnyRef, Decode, Reader, Tag, Tagged};
use rustls::{ClientConfig, RootCertStore};
use rustls_pki_types::ServerName;
use sha2::{Digest, Sha256};
use tokio::net::{lookup_host, TcpStream};
use tokio_rustls::{client::TlsStream, TlsConnector};

use super::{address_policy::validate_and_order_dns_answers, ExternalPoolAdapterBrokerTlsTarget};

const CHANNEL_MAX_AGE_SECONDS: u64 = 30;

/// A server-owned, authenticated transport authority. It deliberately exposes no I/O methods.
pub(crate) struct ExternalPoolAdapterBrokerTlsChannel {
    stream: TlsStream<TcpStream>,
    target: ExternalPoolAdapterBrokerTlsTarget,
    selected_address: SocketAddr,
    connected_at: Instant,
    application_exchange_used: bool,
}

impl ExternalPoolAdapterBrokerTlsChannel {
    pub(crate) fn target(&self) -> &ExternalPoolAdapterBrokerTlsTarget {
        &self.target
    }

    pub(crate) fn selected_address(&self) -> SocketAddr {
        self.selected_address
    }

    pub(crate) fn is_current(&self) -> bool {
        let (_, session) = self.stream.get_ref();
        session.protocol_version() == Some(rustls::ProtocolVersion::TLSv1_3)
            && self.connected_at.elapsed().as_secs() <= CHANNEL_MAX_AGE_SECONDS
    }

    pub(super) fn begin_application_exchange(&mut self) -> Result<&mut TlsStream<TcpStream>> {
        if self.application_exchange_used || !self.is_current() {
            bail!("broker TLS application exchange authority rejected");
        }
        self.application_exchange_used = true;
        Ok(&mut self.stream)
    }
}

pub(crate) async fn connect_external_pool_adapter_broker_tls(
    target: ExternalPoolAdapterBrokerTlsTarget,
) -> Result<ExternalPoolAdapterBrokerTlsChannel> {
    let lookup = tokio::time::timeout(
        target.dns_timeout(),
        lookup_host((target.hostname(), target.port())),
    )
    .await
    .map_err(|_| anyhow!("broker DNS resolution timed out"))??;
    let raw_addresses = lookup
        .take(target.max_dns_answers().saturating_add(1))
        .collect::<Vec<_>>();
    let addresses =
        validate_and_order_dns_answers(raw_addresses, target.port(), target.max_dns_answers())?;
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    connect_resolved(target, addresses, roots).await
}

async fn connect_resolved(
    target: ExternalPoolAdapterBrokerTlsTarget,
    addresses: Vec<SocketAddr>,
    roots: RootCertStore,
) -> Result<ExternalPoolAdapterBrokerTlsChannel> {
    if addresses.is_empty() {
        bail!("broker TLS has no validated address");
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let server_name = ServerName::try_from(target.server_name().to_owned())
        .context("broker TLS server name rejected")?;
    let mut last_connect_error = None;

    for address in addresses.into_iter().take(target.max_connect_attempts()) {
        let tcp = match tokio::time::timeout(target.connect_timeout(), TcpStream::connect(address))
            .await
        {
            Ok(Ok(stream)) => stream,
            Ok(Err(error)) => {
                last_connect_error = Some(anyhow!(error).context("broker TCP connect failed"));
                continue;
            }
            Err(_) => {
                last_connect_error = Some(anyhow!("broker TCP connect timed out"));
                continue;
            }
        };
        tcp.set_nodelay(true)?;
        let stream = tokio::time::timeout(
            target.tls_handshake_timeout(),
            connector.connect(server_name.clone(), tcp),
        )
        .await
        .map_err(|_| anyhow!("broker TLS handshake timed out"))??;
        verify_tls_session(&stream, &target)?;
        return Ok(ExternalPoolAdapterBrokerTlsChannel {
            stream,
            target,
            selected_address: address,
            connected_at: Instant::now(),
            application_exchange_used: false,
        });
    }

    Err(last_connect_error.unwrap_or_else(|| anyhow!("broker TCP connect attempts exhausted")))
}

fn verify_tls_session(
    stream: &TlsStream<TcpStream>,
    target: &ExternalPoolAdapterBrokerTlsTarget,
) -> Result<()> {
    let (_, session) = stream.get_ref();
    if session.protocol_version() != Some(rustls::ProtocolVersion::TLSv1_3) {
        bail!("broker TLS version rejected");
    }
    let leaf = session
        .peer_certificates()
        .and_then(|certificates| certificates.first())
        .ok_or_else(|| anyhow!("broker TLS leaf certificate missing"))?;
    let observed = leaf_spki_sha256(leaf.as_ref())?;
    constant_time::verify_slices_are_equal(
        observed.as_slice(),
        target.expected_leaf_spki_sha256().as_slice(),
    )
    .map_err(|_| anyhow!("broker TLS leaf SPKI pin mismatch"))?;
    Ok(())
}

pub(super) fn leaf_spki_sha256(certificate_der: &[u8]) -> Result<[u8; 32]> {
    let certificate = AnyRef::from_der(certificate_der)
        .context("broker TLS leaf certificate is not valid DER")?;
    let subject_public_key_info_der = certificate
        .sequence(|certificate_reader| {
            let tbs_certificate: AnyRef<'_> = certificate_reader.decode()?;
            let _: AnyRef<'_> = certificate_reader.decode()?;
            let _: AnyRef<'_> = certificate_reader.decode()?;
            tbs_certificate.sequence(|tbs_reader| {
                if tbs_reader.peek_tag()?.is_context_specific() {
                    let _: AnyRef<'_> = tbs_reader.decode()?;
                }
                for _ in 0..5 {
                    let _: AnyRef<'_> = tbs_reader.decode()?;
                }
                let subject_public_key_info_der = tbs_reader.tlv_bytes()?;
                let subject_public_key_info = AnyRef::from_der(subject_public_key_info_der)?;
                subject_public_key_info.tag().assert_eq(Tag::Sequence)?;
                while !tbs_reader.is_finished() {
                    let _: AnyRef<'_> = tbs_reader.decode()?;
                }
                Ok(subject_public_key_info_der)
            })
        })
        .context("broker TLS leaf SubjectPublicKeyInfo is not valid DER")?;
    Ok(Sha256::digest(subject_public_key_info_der).into())
}

#[cfg(test)]
pub(super) async fn connect_external_pool_adapter_broker_tls_for_test(
    target: ExternalPoolAdapterBrokerTlsTarget,
    addresses: Vec<SocketAddr>,
    roots: RootCertStore,
) -> Result<ExternalPoolAdapterBrokerTlsChannel> {
    if addresses.is_empty()
        || addresses.len() > target.max_dns_answers()
        || addresses
            .iter()
            .any(|address| address.port() != target.port())
    {
        bail!("broker TLS test address rejected");
    }
    connect_resolved(target, addresses, roots).await
}
