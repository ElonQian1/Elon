use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

pub(super) const MAX_IJSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
pub(super) const CREDENTIAL_SCHEMA: &str = "elon.node_endpoint.credential.v1";
pub(super) const REVOCATION_SCHEMA: &str = "elon.node_endpoint.credential_revocation.v1";
pub(super) const SESSION_AUTH_SCHEMA: &str = "elon.node_endpoint.session_authentication_receipt.v1";
pub(super) const CANONICALIZATION: &str = "rfc8785_jcs";
pub(super) const DIGEST_ALGORITHM: &str = "sha256";
pub(super) const SECRET_HASH_ALGORITHM: &str = "sha256";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeEndpointOwnerAuthorizationBasis {
    kind: String,
    basis_id: String,
    basis_digest: String,
}

impl NodeEndpointOwnerAuthorizationBasis {
    pub(crate) fn kind(&self) -> &str {
        &self.kind
    }

    pub(crate) fn basis_id(&self) -> &str {
        &self.basis_id
    }

    pub(crate) fn basis_digest(&self) -> &str {
        &self.basis_digest
    }

    pub(super) fn validate(&self) -> Result<()> {
        if !matches!(
            self.kind.as_str(),
            "future_owner_session" | "recent_reauthentication" | "security_operator"
        ) || !bounded_identifier(&self.basis_id, 160)
            || !is_sha256(&self.basis_digest)
        {
            bail!("NODE_ENDPOINT_OWNER_AUTHORIZATION_BASIS_INVALID");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct NodeEndpointCredentialBinding {
    credential_id: String,
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    installation_binding_digest: String,
    credential_revision: u64,
    credential_digest: String,
    status: String,
}

impl NodeEndpointCredentialBinding {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_store_readback(
        credential_id: String,
        agent_id: String,
        owner_user_id: String,
        install_id: String,
        installation_binding_digest: String,
        credential_revision: u64,
        credential_digest: String,
        status: String,
    ) -> Result<Self> {
        let binding = Self {
            credential_id,
            agent_id,
            owner_user_id,
            install_id,
            installation_binding_digest,
            credential_revision,
            credential_digest,
            status,
        };
        binding.validate()?;
        Ok(binding)
    }

    pub(crate) fn credential_id(&self) -> &str {
        &self.credential_id
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

    pub(crate) fn credential_revision(&self) -> u64 {
        self.credential_revision
    }

    pub(crate) fn credential_digest(&self) -> &str {
        &self.credential_digest
    }

    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if !bounded_identifier(&self.credential_id, 160)
            || !bounded_identifier(&self.agent_id, 160)
            || !bounded_identifier(&self.owner_user_id, 160)
            || !bounded_identifier(&self.install_id, 512)
            || !is_sha256(&self.installation_binding_digest)
            || !safe_positive(self.credential_revision)
            || !is_sha256(&self.credential_digest)
            || !matches!(self.status.as_str(), "active" | "revoked")
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_BINDING_INVALID");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct NodeEndpointSessionBinding {
    agent_id: String,
    credential_id: String,
    credential_revision: u64,
    credential_digest: String,
    authentication_receipt_id: String,
    authentication_digest: String,
    session_id: String,
    session_generation: u64,
    server_instance_id: String,
}

impl NodeEndpointSessionBinding {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_store_readback(
        agent_id: String,
        credential_id: String,
        credential_revision: u64,
        credential_digest: String,
        authentication_receipt_id: String,
        authentication_digest: String,
        session_id: String,
        session_generation: u64,
        server_instance_id: String,
    ) -> Result<Self> {
        let binding = Self {
            agent_id,
            credential_id,
            credential_revision,
            credential_digest,
            authentication_receipt_id,
            authentication_digest,
            session_id,
            session_generation,
            server_instance_id,
        };
        binding.validate()?;
        Ok(binding)
    }

    pub(crate) fn agent_id(&self) -> &str {
        &self.agent_id
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
    pub(crate) fn authentication_receipt_id(&self) -> &str {
        &self.authentication_receipt_id
    }
    pub(crate) fn authentication_digest(&self) -> &str {
        &self.authentication_digest
    }
    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }
    pub(crate) fn session_generation(&self) -> u64 {
        self.session_generation
    }
    pub(crate) fn server_instance_id(&self) -> &str {
        &self.server_instance_id
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if !bounded_identifier(&self.agent_id, 160)
            || !bounded_identifier(&self.credential_id, 160)
            || !safe_positive(self.credential_revision)
            || !is_sha256(&self.credential_digest)
            || !bounded_identifier(&self.authentication_receipt_id, 160)
            || !is_sha256(&self.authentication_digest)
            || !bounded_identifier(&self.session_id, 160)
            || !safe_positive(self.session_generation)
            || !bounded_identifier(&self.server_instance_id, 160)
        {
            bail!("NODE_ENDPOINT_SESSION_BINDING_INVALID");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct NodeEndpointSessionHeadSnapshot {
    binding: NodeEndpointSessionBinding,
    state: String,
    authenticated_at: String,
    expires_at: String,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    close_reason_code: Option<String>,
}

impl NodeEndpointSessionHeadSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_store_readback(
        binding: NodeEndpointSessionBinding,
        state: String,
        authenticated_at: String,
        expires_at: String,
        created_at: String,
        updated_at: String,
        closed_at: Option<String>,
        close_reason_code: Option<String>,
    ) -> Result<Self> {
        let snapshot = Self {
            binding,
            state,
            authenticated_at,
            expires_at,
            created_at,
            updated_at,
            closed_at,
            close_reason_code,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub(crate) fn binding(&self) -> &NodeEndpointSessionBinding {
        &self.binding
    }
    pub(crate) fn state(&self) -> &str {
        &self.state
    }
    pub(crate) fn authenticated_at(&self) -> &str {
        &self.authenticated_at
    }
    pub(crate) fn expires_at(&self) -> &str {
        &self.expires_at
    }
    pub(crate) fn created_at(&self) -> &str {
        &self.created_at
    }
    pub(crate) fn updated_at(&self) -> &str {
        &self.updated_at
    }
    pub(crate) fn closed_at(&self) -> Option<&str> {
        self.closed_at.as_deref()
    }
    pub(crate) fn close_reason_code(&self) -> Option<&str> {
        self.close_reason_code.as_deref()
    }

    fn validate(&self) -> Result<()> {
        self.binding.validate()?;
        let terminal = self.state != "active";
        if !matches!(
            self.state.as_str(),
            "active" | "closed" | "stale" | "credential_rotated" | "credential_revoked"
        ) || terminal != self.closed_at.is_some()
            || terminal != self.close_reason_code.is_some()
        {
            bail!("NODE_ENDPOINT_SESSION_HEAD_SHAPE_INVALID");
        }
        let authenticated_at = parse_utc_nanos(&self.authenticated_at)?;
        let expires_at = parse_utc_nanos(&self.expires_at)?;
        let created_at = parse_utc_nanos(&self.created_at)?;
        let updated_at = parse_utc_nanos(&self.updated_at)?;
        if authenticated_at > created_at
            || created_at > updated_at
            || authenticated_at >= expires_at
        {
            bail!("NODE_ENDPOINT_SESSION_HEAD_TIME_INVALID");
        }
        if let Some(closed_at) = self.closed_at.as_deref() {
            let closed_at = parse_utc_nanos(closed_at)?;
            if closed_at != updated_at || closed_at < authenticated_at {
                bail!("NODE_ENDPOINT_SESSION_HEAD_CLOSE_TIME_INVALID");
            }
        }
        Ok(())
    }
}

fn parse_utc_nanos(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_SESSION_HEAD_TIMESTAMP_INVALID"))?
        .with_timezone(&Utc);
    if parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value {
        bail!("NODE_ENDPOINT_SESSION_HEAD_TIMESTAMP_INVALID");
    }
    Ok(parsed)
}

pub(super) fn bounded_identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn safe_positive(value: u64) -> bool {
    value > 0 && value <= MAX_IJSON_SAFE_INTEGER
}
