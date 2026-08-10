use std::fmt;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::super::{
    canonical::canonical_domain_json_and_digest,
    session::VerifiedDirectTlsConnectionEvidence,
    types::{bounded_identifier, is_sha256, safe_positive},
};

const OWNER_API_AUDIENCE_BINDING_DOMAIN: &[u8] =
    b"ELON_NODE_ENDPOINT_OWNER_API_AUDIENCE_BINDING_V1";
const OWNER_API_AUDIENCE_SCHEMA: &str = "elon.node_endpoint.owner_api_audience_binding.v1";
const OWNER_API_AUDIENCE: &str = "owner_api_https";
const OWNER_API_TRANSPORT_LIFETIME_SECONDS: i64 = 30;
const ISSUE_PATH: &str = "/api/me/node-endpoint-credentials/issue";
const CREDENTIAL_PATH_PREFIX: &str = "/api/me/node-endpoint-credentials/";

/// Request-scoped HTTPS evidence. Ownership of neutral TLS evidence is consumed here, so endpoint
/// WSS evidence can never be converted into this audience after the fact.
#[must_use = "secure owner-API transport evidence must be consumed by owner authorization"]
pub(crate) struct VerifiedSecureOwnerApiTransport {
    source: String,
    evidence_schema: String,
    evidence_id: String,
    evidence_digest: String,
    verifier_revision: u64,
    verifier_digest: String,
    server_instance_id: String,
    request_binding_digest: String,
    request_method: String,
    request_path: String,
    request_audience_binding_digest: String,
    verified_at: DateTime<Utc>,
}

/// Linear permission to deliver the result on the response paired with the exact bound request.
/// It is intentionally neither cloneable nor deserializable.
#[must_use = "the permit must be consumed by the response paired with this owner-API request"]
pub(crate) struct OwnerApiResponsePermit {
    evidence_id: String,
    evidence_digest: String,
    server_instance_id: String,
    request_method: String,
    request_path: String,
    canonical_mutation_digest: String,
    request_audience_binding_digest: String,
    verified_at: DateTime<Utc>,
}

/// Consumes one direct-TLS connection into the owner-API audience and a response permit. The
/// caller must pass the parsed URI path, not a normalized route template or caller-supplied URL.
pub(crate) fn bind_direct_tls_owner_api_transport(
    evidence: VerifiedDirectTlsConnectionEvidence,
    request_method: &str,
    exact_path: &str,
    canonical_mutation_digest: &str,
) -> Result<(VerifiedSecureOwnerApiTransport, OwnerApiResponsePermit)> {
    validate_request_shape(request_method, exact_path, canonical_mutation_digest)?;
    if evidence.source() != "direct_tls"
        || !bounded_identifier(evidence.evidence_schema(), 160)
        || !bounded_identifier(evidence.evidence_id(), 160)
        || !is_sha256(evidence.evidence_digest())
        || !safe_positive(evidence.verifier_revision())
        || !is_sha256(evidence.verifier_digest())
        || !bounded_identifier(evidence.server_instance_id(), 160)
    {
        bail!("NODE_ENDPOINT_OWNER_API_DIRECT_TLS_EVIDENCE_INVALID");
    }
    let audience_digest = derive_request_audience_binding_digest(
        evidence.evidence_id(),
        evidence.evidence_digest(),
        evidence.server_instance_id(),
        request_method,
        exact_path,
        canonical_mutation_digest,
    )?;
    let transport = VerifiedSecureOwnerApiTransport {
        source: evidence.source().to_string(),
        evidence_schema: evidence.evidence_schema().to_string(),
        evidence_id: evidence.evidence_id().to_string(),
        evidence_digest: evidence.evidence_digest().to_string(),
        verifier_revision: evidence.verifier_revision(),
        verifier_digest: evidence.verifier_digest().to_string(),
        server_instance_id: evidence.server_instance_id().to_string(),
        request_binding_digest: canonical_mutation_digest.to_string(),
        request_method: request_method.to_string(),
        request_path: exact_path.to_string(),
        request_audience_binding_digest: audience_digest.clone(),
        verified_at: evidence.verified_at(),
    };
    let response_permit = OwnerApiResponsePermit {
        evidence_id: evidence.evidence_id().to_string(),
        evidence_digest: evidence.evidence_digest().to_string(),
        server_instance_id: evidence.server_instance_id().to_string(),
        request_method: request_method.to_string(),
        request_path: exact_path.to_string(),
        canonical_mutation_digest: canonical_mutation_digest.to_string(),
        request_audience_binding_digest: audience_digest,
        verified_at: evidence.verified_at(),
    };
    Ok((transport, response_permit))
}

impl VerifiedSecureOwnerApiTransport {
    pub(super) fn source(&self) -> &str {
        &self.source
    }
    pub(super) fn evidence_schema(&self) -> &str {
        &self.evidence_schema
    }
    pub(super) fn evidence_id(&self) -> &str {
        &self.evidence_id
    }
    pub(super) fn evidence_digest(&self) -> &str {
        &self.evidence_digest
    }
    pub(super) fn verifier_revision(&self) -> u64 {
        self.verifier_revision
    }
    pub(super) fn verifier_digest(&self) -> &str {
        &self.verifier_digest
    }
    pub(super) fn server_instance_id(&self) -> &str {
        &self.server_instance_id
    }
    pub(super) fn request_binding_digest(&self) -> &str {
        &self.request_binding_digest
    }
    pub(super) fn verified_at(&self) -> DateTime<Utc> {
        self.verified_at.to_owned()
    }

    pub(crate) fn validate_for_mutation(
        &self,
        authorization_action: &str,
        agent_id: &str,
        canonical_mutation_digest: &str,
    ) -> Result<()> {
        let expected_path = expected_path(authorization_action, agent_id)?;
        validate_request_shape(
            &self.request_method,
            &self.request_path,
            canonical_mutation_digest,
        )?;
        let expected_audience_digest = derive_request_audience_binding_digest(
            &self.evidence_id,
            &self.evidence_digest,
            &self.server_instance_id,
            &self.request_method,
            &self.request_path,
            canonical_mutation_digest,
        )?;
        if self.request_path != expected_path
            || self.request_binding_digest != canonical_mutation_digest
            || self.request_audience_binding_digest != expected_audience_digest
        {
            bail!("NODE_ENDPOINT_OWNER_API_REQUEST_BINDING_MISMATCH");
        }
        Ok(())
    }

    pub(crate) fn ensure_fresh_at(&self, observed_at: DateTime<Utc>) -> Result<()> {
        ensure_transport_fresh(self.verified_at, observed_at)
    }
}

impl OwnerApiResponsePermit {
    pub(crate) fn validate_pair(&self, transport: &VerifiedSecureOwnerApiTransport) -> Result<()> {
        if self.evidence_id != transport.evidence_id
            || self.evidence_digest != transport.evidence_digest
            || self.server_instance_id != transport.server_instance_id
            || self.request_method != transport.request_method
            || self.request_path != transport.request_path
            || self.canonical_mutation_digest != transport.request_binding_digest
            || self.request_audience_binding_digest != transport.request_audience_binding_digest
            || self.verified_at != transport.verified_at
        {
            bail!("NODE_ENDPOINT_OWNER_API_RESPONSE_PERMIT_PAIR_MISMATCH");
        }
        Ok(())
    }

    pub(crate) fn ensure_fresh_at(&self, observed_at: DateTime<Utc>) -> Result<()> {
        ensure_transport_fresh(self.verified_at, observed_at)
    }

    /// Consumes the permit only for the response to the same parsed request and mutation body.
    pub(crate) fn consume_for_response(
        self,
        request_method: &str,
        exact_path: &str,
        canonical_mutation_digest: &str,
    ) -> Result<()> {
        self.ensure_fresh_at(Utc::now())?;
        validate_request_shape(request_method, exact_path, canonical_mutation_digest)?;
        let expected = derive_request_audience_binding_digest(
            &self.evidence_id,
            &self.evidence_digest,
            &self.server_instance_id,
            request_method,
            exact_path,
            canonical_mutation_digest,
        )?;
        if self.request_method != request_method
            || self.request_path != exact_path
            || self.canonical_mutation_digest != canonical_mutation_digest
            || self.request_audience_binding_digest != expected
        {
            bail!("NODE_ENDPOINT_OWNER_API_RESPONSE_PERMIT_MISMATCH");
        }
        Ok(())
    }
}

fn ensure_transport_fresh(verified_at: DateTime<Utc>, observed_at: DateTime<Utc>) -> Result<()> {
    let expires_at = verified_at
        .checked_add_signed(chrono::Duration::seconds(
            OWNER_API_TRANSPORT_LIFETIME_SECONDS,
        ))
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_API_TRANSPORT_EXPIRY_OVERFLOW"))?;
    if observed_at < verified_at || observed_at > expires_at {
        bail!("NODE_ENDPOINT_OWNER_API_TRANSPORT_EXPIRED");
    }
    Ok(())
}

impl fmt::Debug for VerifiedSecureOwnerApiTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedSecureOwnerApiTransport")
            .field("audience", &OWNER_API_AUDIENCE)
            .field("request", &"<sealed>")
            .finish()
    }
}

impl fmt::Debug for OwnerApiResponsePermit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerApiResponsePermit")
            .field("audience", &OWNER_API_AUDIENCE)
            .field("response", &"<sealed>")
            .finish()
    }
}

fn validate_request_shape(
    request_method: &str,
    exact_path: &str,
    canonical_mutation_digest: &str,
) -> Result<()> {
    if request_method != "POST"
        || !is_sha256(canonical_mutation_digest)
        || !is_supported_exact_path(exact_path)
    {
        bail!("NODE_ENDPOINT_OWNER_API_REQUEST_SHAPE_INVALID");
    }
    Ok(())
}

fn is_supported_exact_path(path: &str) -> bool {
    if path == ISSUE_PATH {
        return true;
    }
    let Some(remainder) = path.strip_prefix(CREDENTIAL_PATH_PREFIX) else {
        return false;
    };
    let mut segments = remainder.split('/');
    let (Some(agent_id), Some(action), None) = (segments.next(), segments.next(), segments.next())
    else {
        return false;
    };
    valid_agent_path_segment(agent_id) && matches!(action, "rotate" | "recover" | "revoke")
}

fn expected_path(authorization_action: &str, agent_id: &str) -> Result<String> {
    let suffix = match authorization_action {
        "initial_registration" => return Ok(ISSUE_PATH.to_string()),
        "credential_rotation" => "rotate",
        "account_recovery" => "recover",
        "owner_revocation" => "revoke",
        _ => bail!("NODE_ENDPOINT_OWNER_API_AUTHORIZATION_ACTION_INVALID"),
    };
    if !valid_agent_path_segment(agent_id) {
        bail!("NODE_ENDPOINT_OWNER_API_AGENT_PATH_SEGMENT_INVALID");
    }
    Ok(format!("{CREDENTIAL_PATH_PREFIX}{agent_id}/{suffix}"))
}

fn valid_agent_path_segment(value: &str) -> bool {
    bounded_identifier(value, 160)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
}

fn derive_request_audience_binding_digest(
    evidence_id: &str,
    evidence_digest: &str,
    server_instance_id: &str,
    request_method: &str,
    exact_path: &str,
    canonical_mutation_digest: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Binding<'a> {
        schema: &'static str,
        audience: &'static str,
        evidence_id: &'a str,
        evidence_digest: &'a str,
        server_instance_id: &'a str,
        request_method: &'a str,
        exact_path: &'a str,
        canonical_mutation_digest: &'a str,
    }
    canonical_domain_json_and_digest(
        OWNER_API_AUDIENCE_BINDING_DOMAIN,
        &Binding {
            schema: OWNER_API_AUDIENCE_SCHEMA,
            audience: OWNER_API_AUDIENCE,
            evidence_id,
            evidence_digest,
            server_instance_id,
            request_method,
            exact_path,
            canonical_mutation_digest,
        },
    )
    .map(|(_, digest)| digest)
}
