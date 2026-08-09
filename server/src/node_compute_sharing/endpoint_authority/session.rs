use std::fmt;

use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use super::canonical::{
    canonical_domain_json_and_digest, deterministic_identifier, ensure_canonical_readback,
    ensure_time_order, installation_binding_digest, parse_utc_nanos, utc_nanos,
};
use super::credential::PresentedNodeEndpointCredentialSecret;
use super::types::{
    bounded_identifier, is_sha256, safe_positive, NodeEndpointCredentialBinding,
    NodeEndpointSessionHeadSnapshot, CANONICALIZATION, DIGEST_ALGORITHM, MAX_IJSON_SAFE_INTEGER,
    SESSION_AUTH_SCHEMA,
};

const AUTHENTICATION_ID_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_SESSION_AUTHENTICATION_ID_V1";
const AUTHENTICATION_DIGEST_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_SESSION_AUTHENTICATION_RECEIPT_V1";
const CAPABILITY_SET_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_SESSION_CAPABILITY_SET_V1";
const SESSION_LIFETIME_MINUTES: i64 = 15;
const MAX_CAPABILITIES: usize = 256;

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

    fn validate(&self) -> Result<()> {
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

pub(crate) struct VerifiedSecureNodeEndpointTransport {
    binding: NodeEndpointSecureTransportBinding,
    verified_at: DateTime<Utc>,
}

impl VerifiedSecureNodeEndpointTransport {
    pub(crate) fn binding(&self) -> &NodeEndpointSecureTransportBinding {
        &self.binding
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

pub(crate) struct NodeEndpointSessionOpenRequest {
    agent_id: String,
    session_id: String,
    server_instance_id: String,
    protocol_version: u64,
    agent_version: String,
    capabilities: Vec<String>,
    presented_secret: PresentedNodeEndpointCredentialSecret,
    authenticated_at: DateTime<Utc>,
}

impl NodeEndpointSessionOpenRequest {
    pub(crate) fn agent_id(&self) -> &str {
        &self.agent_id
    }
    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }
    pub(crate) fn server_instance_id(&self) -> &str {
        &self.server_instance_id
    }
    pub(crate) fn presented_secret(&self) -> &PresentedNodeEndpointCredentialSecret {
        &self.presented_secret
    }
    pub(crate) fn authenticated_at(&self) -> DateTime<Utc> {
        self.authenticated_at
    }

    pub(crate) fn prepare(
        &self,
        credential: &NodeEndpointCredentialBinding,
        predecessor: Option<&NodeEndpointSessionHeadSnapshot>,
        transport: &VerifiedSecureNodeEndpointTransport,
        recorded_at: DateTime<Utc>,
    ) -> Result<PreparedNodeEndpointSessionAuthentication> {
        credential.validate()?;
        transport.binding.validate()?;
        if credential.status() != "active"
            || credential.agent_id() != self.agent_id
            || !bounded_identifier(&self.session_id, 160)
            || !bounded_identifier(&self.server_instance_id, 160)
            || !safe_positive(self.protocol_version)
            || !bounded_identifier(&self.agent_version, 160)
        {
            bail!("NODE_ENDPOINT_SESSION_OPEN_REQUEST_INVALID");
        }
        ensure_time_order(
            transport.verified_at,
            self.authenticated_at,
            "NODE_ENDPOINT_TRANSPORT_VERIFIED_AFTER_AUTHENTICATION",
        )?;
        ensure_time_order(
            self.authenticated_at,
            recorded_at,
            "NODE_ENDPOINT_AUTHENTICATED_AFTER_RECORDED",
        )?;
        if utc_nanos(transport.verified_at) != transport.binding.transport_verified_at {
            bail!("NODE_ENDPOINT_TRANSPORT_TIME_PROJECTION_MISMATCH");
        }
        let capabilities = canonical_capabilities(&self.capabilities)?;
        let (capability_set_json, capability_set_digest) =
            canonical_domain_json_and_digest(CAPABILITY_SET_DOMAIN, &capabilities)?;
        let (session_generation, previous_id, previous_digest) = match predecessor {
            None => (1, None, None),
            Some(previous) => {
                if previous.binding().agent_id() != self.agent_id {
                    bail!("NODE_ENDPOINT_SESSION_PREDECESSOR_AGENT_MISMATCH");
                }
                let generation = previous
                    .binding()
                    .session_generation()
                    .checked_add(1)
                    .filter(|value| *value <= MAX_IJSON_SAFE_INTEGER)
                    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_GENERATION_EXHAUSTED"))?;
                (
                    generation,
                    Some(previous.binding().authentication_receipt_id().to_string()),
                    Some(previous.binding().authentication_digest().to_string()),
                )
            }
        };
        #[derive(Serialize)]
        struct Identity<'a> {
            agent_id: &'a str,
            credential_id: &'a str,
            credential_revision: u64,
            session_id: &'a str,
            session_generation: u64,
            server_instance_id: &'a str,
        }
        let authentication_receipt_id = deterministic_identifier(
            "neauth_",
            AUTHENTICATION_ID_DOMAIN,
            &Identity {
                agent_id: &self.agent_id,
                credential_id: credential.credential_id(),
                credential_revision: credential.credential_revision(),
                session_id: &self.session_id,
                session_generation,
                server_instance_id: &self.server_instance_id,
            },
        )?;
        let expires_at = self
            .authenticated_at
            .checked_add_signed(Duration::minutes(SESSION_LIFETIME_MINUTES))
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_EXPIRY_OVERFLOW"))?;
        let envelope = NodeEndpointSessionAuthenticationReceiptEnvelope {
            schema: SESSION_AUTH_SCHEMA.to_string(),
            authentication_receipt_id,
            credential_id: credential.credential_id().to_string(),
            credential_revision: credential.credential_revision(),
            credential_digest: credential.credential_digest().to_string(),
            agent_id: credential.agent_id().to_string(),
            owner_user_id: credential.owner_user_id().to_string(),
            install_id: credential.install_id().to_string(),
            installation_binding_digest: credential.installation_binding_digest().to_string(),
            session_id: self.session_id.clone(),
            session_generation,
            previous_authentication_receipt_id: previous_id,
            previous_authentication_digest: previous_digest,
            server_instance_id: self.server_instance_id.clone(),
            authentication_method: "bearer_sha256".to_string(),
            protocol_version: self.protocol_version,
            agent_version: self.agent_version.clone(),
            capability_count: capabilities.len() as u64,
            capability_set_digest,
            transport: transport.binding.clone(),
            authenticated_at: utc_nanos(self.authenticated_at),
            expires_at: utc_nanos(expires_at),
            recorded_at: utc_nanos(recorded_at),
        };
        envelope.validate()?;
        let (authentication_json, authentication_digest) =
            canonical_domain_json_and_digest(AUTHENTICATION_DIGEST_DOMAIN, &envelope)?;
        Ok(PreparedNodeEndpointSessionAuthentication {
            envelope,
            authentication_json,
            authentication_digest,
            capability_set_json,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeEndpointSessionAuthenticationReceiptEnvelope {
    schema: String,
    authentication_receipt_id: String,
    credential_id: String,
    credential_revision: u64,
    credential_digest: String,
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    installation_binding_digest: String,
    session_id: String,
    session_generation: u64,
    previous_authentication_receipt_id: Option<String>,
    previous_authentication_digest: Option<String>,
    server_instance_id: String,
    authentication_method: String,
    protocol_version: u64,
    agent_version: String,
    capability_count: u64,
    capability_set_digest: String,
    transport: NodeEndpointSecureTransportBinding,
    authenticated_at: String,
    expires_at: String,
    recorded_at: String,
}

impl NodeEndpointSessionAuthenticationReceiptEnvelope {
    pub(crate) fn schema(&self) -> &str {
        &self.schema
    }
    pub(crate) fn authentication_receipt_id(&self) -> &str {
        &self.authentication_receipt_id
    }
    pub(crate) fn credential_id(&self) -> &str {
        &self.credential_id
    }
    pub(crate) fn credential_revision(&self) -> u64 {
        self.credential_revision
    }
    pub(crate) fn credential_digest(&self) -> &str {
        &self.credential_digest
    }
    pub(crate) fn agent_id(&self) -> &str {
        &self.agent_id
    }
    pub(crate) fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }
    pub(crate) fn install_id(&self) -> &str {
        &self.install_id
    }
    pub(crate) fn installation_binding_digest(&self) -> &str {
        &self.installation_binding_digest
    }
    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }
    pub(crate) fn session_generation(&self) -> u64 {
        self.session_generation
    }
    pub(crate) fn previous_authentication_receipt_id(&self) -> Option<&str> {
        self.previous_authentication_receipt_id.as_deref()
    }
    pub(crate) fn previous_authentication_digest(&self) -> Option<&str> {
        self.previous_authentication_digest.as_deref()
    }
    pub(crate) fn server_instance_id(&self) -> &str {
        &self.server_instance_id
    }
    pub(crate) fn authentication_method(&self) -> &str {
        &self.authentication_method
    }
    pub(crate) fn protocol_version(&self) -> u64 {
        self.protocol_version
    }
    pub(crate) fn agent_version(&self) -> &str {
        &self.agent_version
    }
    pub(crate) fn capability_count(&self) -> u64 {
        self.capability_count
    }
    pub(crate) fn capability_set_digest(&self) -> &str {
        &self.capability_set_digest
    }
    pub(crate) fn transport(&self) -> &NodeEndpointSecureTransportBinding {
        &self.transport
    }
    pub(crate) fn authenticated_at(&self) -> &str {
        &self.authenticated_at
    }
    pub(crate) fn expires_at(&self) -> &str {
        &self.expires_at
    }
    pub(crate) fn recorded_at(&self) -> &str {
        &self.recorded_at
    }

    pub(crate) fn validate_store_readback(
        &self,
        stored_json: &str,
        stored_digest: &str,
    ) -> Result<()> {
        self.validate()?;
        ensure_canonical_readback(
            AUTHENTICATION_DIGEST_DOMAIN,
            self,
            stored_json,
            stored_digest,
        )
    }

    fn validate(&self) -> Result<()> {
        if self.schema != SESSION_AUTH_SCHEMA
            || !bounded_identifier(&self.authentication_receipt_id, 160)
            || !bounded_identifier(&self.credential_id, 160)
            || !safe_positive(self.credential_revision)
            || !is_sha256(&self.credential_digest)
            || !bounded_identifier(&self.agent_id, 160)
            || !bounded_identifier(&self.owner_user_id, 160)
            || !bounded_identifier(&self.install_id, 512)
            || !is_sha256(&self.installation_binding_digest)
            || !bounded_identifier(&self.session_id, 160)
            || !safe_positive(self.session_generation)
            || !bounded_identifier(&self.server_instance_id, 160)
            || self.authentication_method != "bearer_sha256"
            || !safe_positive(self.protocol_version)
            || !bounded_identifier(&self.agent_version, 160)
            || self.capability_count > MAX_CAPABILITIES as u64
            || !is_sha256(&self.capability_set_digest)
        {
            bail!("NODE_ENDPOINT_SESSION_AUTHENTICATION_INVALID");
        }
        match (
            self.session_generation,
            self.previous_authentication_receipt_id.as_deref(),
            self.previous_authentication_digest.as_deref(),
        ) {
            (1, None, None) => {}
            (generation, Some(id), Some(digest))
                if generation > 1 && bounded_identifier(id, 160) && is_sha256(digest) => {}
            _ => bail!("NODE_ENDPOINT_SESSION_PREDECESSOR_INVALID"),
        }
        if installation_binding_digest(&self.agent_id, &self.owner_user_id, &self.install_id)?
            != self.installation_binding_digest
        {
            bail!("NODE_ENDPOINT_SESSION_INSTALLATION_BINDING_MISMATCH");
        }
        self.transport.validate()?;
        let verified_at = parse_utc_nanos(
            self.transport.transport_verified_at(),
            "NODE_ENDPOINT_TRANSPORT_VERIFIED_AT_INVALID",
        )?;
        let authenticated_at = parse_utc_nanos(
            &self.authenticated_at,
            "NODE_ENDPOINT_AUTHENTICATED_AT_INVALID",
        )?;
        let expires_at = parse_utc_nanos(&self.expires_at, "NODE_ENDPOINT_EXPIRES_AT_INVALID")?;
        let recorded_at = parse_utc_nanos(&self.recorded_at, "NODE_ENDPOINT_RECORDED_AT_INVALID")?;
        if verified_at > authenticated_at
            || authenticated_at >= recorded_at
            || recorded_at >= expires_at
            || expires_at
                != authenticated_at
                    .checked_add_signed(Duration::minutes(SESSION_LIFETIME_MINUTES))
                    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_EXPIRY_OVERFLOW"))?
        {
            bail!("NODE_ENDPOINT_SESSION_TIME_BINDING_INVALID");
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct PreparedNodeEndpointSessionAuthentication {
    envelope: NodeEndpointSessionAuthenticationReceiptEnvelope,
    authentication_json: String,
    authentication_digest: String,
    capability_set_json: String,
}

impl PreparedNodeEndpointSessionAuthentication {
    pub(crate) fn envelope(&self) -> &NodeEndpointSessionAuthenticationReceiptEnvelope {
        &self.envelope
    }
    pub(crate) fn authentication_json(&self) -> &str {
        &self.authentication_json
    }
    pub(crate) fn authentication_digest(&self) -> &str {
        &self.authentication_digest
    }
    pub(crate) fn capability_set_json(&self) -> &str {
        &self.capability_set_json
    }
    pub(crate) fn canonicalization(&self) -> &'static str {
        CANONICALIZATION
    }
    pub(crate) fn digest_algorithm(&self) -> &'static str {
        DIGEST_ALGORITHM
    }
}

fn canonical_capabilities(values: &[String]) -> Result<Vec<String>> {
    if values.len() > MAX_CAPABILITIES {
        bail!("NODE_ENDPOINT_CAPABILITY_SET_TOO_LARGE");
    }
    let mut capabilities = values.to_vec();
    capabilities.sort();
    capabilities.dedup();
    if capabilities.len() != values.len()
        || capabilities
            .iter()
            .any(|value| !bounded_identifier(value, 160))
    {
        bail!("NODE_ENDPOINT_CAPABILITY_SET_INVALID");
    }
    Ok(capabilities)
}

pub(crate) fn canonical_node_endpoint_capability_set(
    values: &[String],
) -> Result<(String, String)> {
    let capabilities = canonical_capabilities(values)?;
    canonical_domain_json_and_digest(CAPABILITY_SET_DOMAIN, &capabilities)
}
