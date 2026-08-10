use std::fmt;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rustls::{HandshakeKind, ProtocolVersion, ServerConnection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    node_compute_sharing::endpoint_authority::{
        canonical::canonical_domain_json_and_digest,
        types::{bounded_identifier, is_sha256, safe_positive},
    },
    node_endpoint_transport::DirectTlsVerifierSeal,
};

const VERIFIER_DIGEST_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_DIRECT_TLS_VERIFIER_V2";
const EVIDENCE_DIGEST_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_DIRECT_TLS_EVIDENCE_V2";
const VERIFIER_SCHEMA: &str = "elon.node_endpoint.direct_tls_verifier.v2";
const EVIDENCE_SCHEMA: &str = "elon.node_endpoint.direct_tls_handshake.v2";
const REQUIRED_ALPN: &[u8] = b"http/1.1";

/// Audience-neutral facts from one completed direct-TLS handshake. Ownership must be consumed by
/// exactly one audience binder; this type is intentionally neither cloneable nor deserializable.
#[must_use = "direct-TLS evidence must be consumed by exactly one audience binder"]
pub(crate) struct VerifiedDirectTlsConnectionEvidence {
    source: String,
    evidence_schema: String,
    evidence_id: String,
    evidence_digest: String,
    verifier_revision: u64,
    verifier_digest: String,
    server_instance_id: String,
    verified_at: DateTime<Utc>,
}

impl VerifiedDirectTlsConnectionEvidence {
    pub(crate) fn source(&self) -> &str {
        &self.source
    }
    pub(crate) fn evidence_schema(&self) -> &str {
        &self.evidence_schema
    }
    pub(crate) fn evidence_id(&self) -> &str {
        &self.evidence_id
    }
    pub(crate) fn evidence_digest(&self) -> &str {
        &self.evidence_digest
    }
    pub(crate) fn verifier_revision(&self) -> u64 {
        self.verifier_revision
    }
    pub(crate) fn verifier_digest(&self) -> &str {
        &self.verifier_digest
    }
    pub(crate) fn server_instance_id(&self) -> &str {
        &self.server_instance_id
    }
    pub(crate) fn verified_at(&self) -> DateTime<Utc> {
        self.verified_at.to_owned()
    }

    fn validate(&self) -> Result<()> {
        if self.source != "direct_tls"
            || self.evidence_schema != EVIDENCE_SCHEMA
            || !bounded_identifier(&self.evidence_id, 160)
            || !is_sha256(&self.evidence_digest)
            || !safe_positive(self.verifier_revision)
            || !is_sha256(&self.verifier_digest)
            || !bounded_identifier(&self.server_instance_id, 160)
        {
            bail!("NODE_ENDPOINT_DIRECT_TLS_CONNECTION_EVIDENCE_INVALID");
        }
        Ok(())
    }
}

impl fmt::Debug for VerifiedDirectTlsConnectionEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedDirectTlsConnectionEvidence")
            .field("source", &self.source)
            .field("evidence", &"<sealed>")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeEndpointSecureTransportBinding {
    transport_scheme: String,
    transport_security_source: String,
    transport_security_evidence_schema: String,
    transport_security_evidence_id: String,
    transport_security_evidence_digest: String,
    transport_verifier_revision: u64,
    transport_verifier_digest: String,
    transport_verified_at: String,
}

impl NodeEndpointSecureTransportBinding {
    pub(crate) fn transport_scheme(&self) -> &str {
        &self.transport_scheme
    }
    pub(crate) fn transport_security_source(&self) -> &str {
        &self.transport_security_source
    }
    pub(crate) fn transport_security_evidence_schema(&self) -> &str {
        &self.transport_security_evidence_schema
    }
    pub(crate) fn transport_security_evidence_id(&self) -> &str {
        &self.transport_security_evidence_id
    }
    pub(crate) fn transport_security_evidence_digest(&self) -> &str {
        &self.transport_security_evidence_digest
    }
    pub(crate) fn transport_verifier_revision(&self) -> u64 {
        self.transport_verifier_revision
    }
    pub(crate) fn transport_verifier_digest(&self) -> &str {
        &self.transport_verifier_digest
    }
    pub(crate) fn transport_verified_at(&self) -> &str {
        &self.transport_verified_at
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.transport_scheme != "wss"
            || !matches!(
                self.transport_security_source.as_str(),
                "direct_tls" | "trusted_reverse_proxy_tls"
            )
            || !bounded_identifier(&self.transport_security_evidence_schema, 160)
            || !bounded_identifier(&self.transport_security_evidence_id, 160)
            || !is_sha256(&self.transport_security_evidence_digest)
            || !safe_positive(self.transport_verifier_revision)
            || !is_sha256(&self.transport_verifier_digest)
        {
            bail!("NODE_ENDPOINT_SECURE_TRANSPORT_BINDING_INVALID");
        }
        Ok(())
    }
}

#[must_use = "secure endpoint transport evidence must be consumed by session authentication"]
pub(crate) struct VerifiedSecureNodeEndpointTransport {
    binding: NodeEndpointSecureTransportBinding,
    server_instance_id: String,
    verified_at: DateTime<Utc>,
}

/// Consumes neutral handshake evidence into the endpoint-WSS audience. Owner-API code has a
/// separate consuming binder and cannot convert this proof into its HTTPS audience.
pub(crate) fn bind_direct_tls_node_endpoint_transport(
    evidence: VerifiedDirectTlsConnectionEvidence,
) -> Result<VerifiedSecureNodeEndpointTransport> {
    evidence.validate()?;
    let binding = NodeEndpointSecureTransportBinding {
        transport_scheme: "wss".to_string(),
        transport_security_source: evidence.source,
        transport_security_evidence_schema: evidence.evidence_schema,
        transport_security_evidence_id: evidence.evidence_id,
        transport_security_evidence_digest: evidence.evidence_digest,
        transport_verifier_revision: evidence.verifier_revision,
        transport_verifier_digest: evidence.verifier_digest,
        transport_verified_at: super::super::canonical::utc_nanos(evidence.verified_at.to_owned()),
    };
    binding.validate()?;
    Ok(VerifiedSecureNodeEndpointTransport {
        binding,
        server_instance_id: evidence.server_instance_id,
        verified_at: evidence.verified_at,
    })
}

impl VerifiedSecureNodeEndpointTransport {
    pub(crate) fn binding(&self) -> &NodeEndpointSecureTransportBinding {
        &self.binding
    }
    pub(crate) fn server_instance_id(&self) -> &str {
        &self.server_instance_id
    }
    pub(crate) fn verified_at(&self) -> DateTime<Utc> {
        self.verified_at
    }
}

impl fmt::Debug for VerifiedSecureNodeEndpointTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedSecureNodeEndpointTransport")
            .field("transport_scheme", &self.binding.transport_scheme)
            .field("evidence", &"<sealed>")
            .finish()
    }
}

pub(crate) fn canonical_direct_tls_verifier_digest(
    server_instance_id: &str,
    listener_id: &str,
    verifier_revision: u64,
    leaf_certificate_digest: &str,
) -> Result<String> {
    if !bounded_identifier(server_instance_id, 160)
        || !bounded_identifier(listener_id, 160)
        || !safe_positive(verifier_revision)
        || !is_sha256(leaf_certificate_digest)
    {
        bail!("NODE_ENDPOINT_DIRECT_TLS_VERIFIER_CONFIG_INVALID");
    }
    #[derive(Serialize)]
    struct VerifierEnvelope<'a> {
        schema: &'static str,
        server_instance_id: &'a str,
        listener_id: &'a str,
        transport_security_source: &'static str,
        allowed_audiences: [&'static str; 2],
        tls_protocol: &'static str,
        allowed_cipher_suites: [u64; 3],
        required_alpn: &'static str,
        leaf_certificate_digest: &'a str,
        verifier_revision: u64,
    }
    canonical_domain_json_and_digest(
        VERIFIER_DIGEST_DOMAIN,
        &VerifierEnvelope {
            schema: VERIFIER_SCHEMA,
            server_instance_id,
            listener_id,
            transport_security_source: "direct_tls",
            allowed_audiences: ["node_endpoint_wss", "owner_api_https"],
            tls_protocol: "tls1.3",
            allowed_cipher_suites: [0x1301, 0x1302, 0x1303],
            required_alpn: "http/1.1",
            leaf_certificate_digest,
            verifier_revision,
        },
    )
    .map(|(_, digest)| digest)
}

/// Seals audience-neutral facts only after rustls reports a completed TLS 1.3 handshake with the
/// exact ALPN.
/// Request headers, URI schemes, SNI, peer addresses, and proxy metadata never enter this path.
pub(crate) fn seal_direct_tls_connection(
    connection: &ServerConnection,
    verifier: &DirectTlsVerifierSeal,
    verified_at: DateTime<Utc>,
) -> Result<VerifiedDirectTlsConnectionEvidence> {
    let verifier_digest = canonical_direct_tls_verifier_digest(
        verifier.server_instance_id(),
        verifier.listener_id(),
        verifier.verifier_revision(),
        verifier.leaf_certificate_digest(),
    )?;
    if verifier_digest != verifier.verifier_digest() {
        bail!("NODE_ENDPOINT_DIRECT_TLS_VERIFIER_DIGEST_MISMATCH");
    }
    if connection.is_handshaking()
        || connection.protocol_version() != Some(ProtocolVersion::TLSv1_3)
        || connection.alpn_protocol() != Some(REQUIRED_ALPN)
    {
        bail!("NODE_ENDPOINT_DIRECT_TLS_HANDSHAKE_INCOMPLETE");
    }
    let cipher_suite = connection
        .negotiated_cipher_suite()
        .map(|suite| u64::from(u16::from(suite.suite())))
        .filter(|suite| matches!(*suite, 0x1301..=0x1303))
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_DIRECT_TLS_CIPHER_INVALID"))?;
    let handshake_kind = match connection.handshake_kind() {
        Some(HandshakeKind::Full) => "full",
        Some(HandshakeKind::FullWithHelloRetryRequest) => "full_with_hello_retry_request",
        Some(HandshakeKind::Resumed) => "resumed",
        None => bail!("NODE_ENDPOINT_DIRECT_TLS_HANDSHAKE_KIND_MISSING"),
    };
    let evidence_id = format!("netls_{}", Uuid::new_v4().simple());
    let verified_at_text = super::super::canonical::utc_nanos(verified_at);
    #[derive(Serialize)]
    struct EvidenceEnvelope<'a> {
        schema: &'static str,
        evidence_id: &'a str,
        server_instance_id: &'a str,
        listener_id: &'a str,
        transport_security_source: &'static str,
        tls_protocol: &'static str,
        cipher_suite: u64,
        alpn: &'static str,
        handshake_kind: &'a str,
        leaf_certificate_digest: &'a str,
        verifier_revision: u64,
        verifier_digest: &'a str,
        verified_at: &'a str,
    }
    let (_, evidence_digest) = canonical_domain_json_and_digest(
        EVIDENCE_DIGEST_DOMAIN,
        &EvidenceEnvelope {
            schema: EVIDENCE_SCHEMA,
            evidence_id: &evidence_id,
            server_instance_id: verifier.server_instance_id(),
            listener_id: verifier.listener_id(),
            transport_security_source: "direct_tls",
            tls_protocol: "tls1.3",
            cipher_suite,
            alpn: "http/1.1",
            handshake_kind,
            leaf_certificate_digest: verifier.leaf_certificate_digest(),
            verifier_revision: verifier.verifier_revision(),
            verifier_digest: verifier.verifier_digest(),
            verified_at: &verified_at_text,
        },
    )?;
    let evidence = VerifiedDirectTlsConnectionEvidence {
        source: "direct_tls".to_string(),
        evidence_schema: EVIDENCE_SCHEMA.to_string(),
        evidence_id,
        evidence_digest,
        verifier_revision: verifier.verifier_revision(),
        verifier_digest: verifier.verifier_digest().to_string(),
        server_instance_id: verifier.server_instance_id().to_string(),
        verified_at,
    };
    evidence.validate()?;
    Ok(evidence)
}
