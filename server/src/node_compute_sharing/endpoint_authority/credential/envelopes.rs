use std::fmt;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::super::canonical::{
    ensure_canonical_readback, installation_binding_digest, parse_utc_nanos,
};
use super::super::types::{
    bounded_identifier, is_sha256, safe_positive, NodeEndpointOwnerAuthorizationBasis,
    CANONICALIZATION, CREDENTIAL_SCHEMA, DIGEST_ALGORITHM, REVOCATION_SCHEMA,
    SECRET_HASH_ALGORITHM,
};

pub(super) const CREDENTIAL_DIGEST_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_CREDENTIAL_V1";
pub(super) const REVOCATION_DIGEST_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_CREDENTIAL_REVOCATION_V1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeEndpointCredentialVersionEnvelope {
    schema: String,
    credential_id: String,
    credential_revision: u64,
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    installation_binding_digest: String,
    secret_verifier_digest: String,
    issuance_kind: String,
    issuance_request_id: String,
    issued_by_user_id: String,
    owner_authorization_basis: NodeEndpointOwnerAuthorizationBasis,
    previous_credential_revision: Option<u64>,
    previous_credential_digest: Option<String>,
    issued_at: String,
    recorded_at: String,
}

impl NodeEndpointCredentialVersionEnvelope {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        credential_id: String,
        credential_revision: u64,
        agent_id: String,
        owner_user_id: String,
        install_id: String,
        installation_binding_digest: String,
        secret_verifier_digest: String,
        issuance_kind: String,
        issuance_request_id: String,
        issued_by_user_id: String,
        owner_authorization_basis: NodeEndpointOwnerAuthorizationBasis,
        previous_credential_revision: Option<u64>,
        previous_credential_digest: Option<String>,
        issued_at: String,
        recorded_at: String,
    ) -> Result<Self> {
        let envelope = Self {
            schema: CREDENTIAL_SCHEMA.to_string(),
            credential_id,
            credential_revision,
            agent_id,
            owner_user_id,
            install_id,
            installation_binding_digest,
            secret_verifier_digest,
            issuance_kind,
            issuance_request_id,
            issued_by_user_id,
            owner_authorization_basis,
            previous_credential_revision,
            previous_credential_digest,
            issued_at,
            recorded_at,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub(crate) fn schema(&self) -> &str {
        &self.schema
    }
    pub(crate) fn credential_id(&self) -> &str {
        &self.credential_id
    }
    pub(crate) fn credential_revision(&self) -> u64 {
        self.credential_revision
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
    pub(crate) fn secret_verifier_digest(&self) -> &str {
        &self.secret_verifier_digest
    }
    pub(crate) fn issuance_kind(&self) -> &str {
        &self.issuance_kind
    }
    pub(crate) fn issuance_request_id(&self) -> &str {
        &self.issuance_request_id
    }
    pub(crate) fn issued_by_user_id(&self) -> &str {
        &self.issued_by_user_id
    }
    pub(crate) fn owner_authorization_basis(&self) -> &NodeEndpointOwnerAuthorizationBasis {
        &self.owner_authorization_basis
    }
    pub(crate) fn previous_credential_revision(&self) -> Option<u64> {
        self.previous_credential_revision
    }
    pub(crate) fn previous_credential_digest(&self) -> Option<&str> {
        self.previous_credential_digest.as_deref()
    }
    pub(crate) fn issued_at(&self) -> &str {
        &self.issued_at
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
        ensure_canonical_readback(CREDENTIAL_DIGEST_DOMAIN, self, stored_json, stored_digest)
    }

    fn validate(&self) -> Result<()> {
        if self.schema != CREDENTIAL_SCHEMA
            || !bounded_identifier(&self.credential_id, 160)
            || !safe_positive(self.credential_revision)
            || !bounded_identifier(&self.agent_id, 160)
            || !bounded_identifier(&self.owner_user_id, 160)
            || !bounded_identifier(&self.install_id, 512)
            || !is_sha256(&self.installation_binding_digest)
            || !is_sha256(&self.secret_verifier_digest)
            || !matches!(
                self.issuance_kind.as_str(),
                "initial_registration" | "credential_rotation" | "account_recovery"
            )
            || !bounded_identifier(&self.issuance_request_id, 160)
            || !bounded_identifier(&self.issued_by_user_id, 160)
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_ENVELOPE_INVALID");
        }
        self.owner_authorization_basis.validate()?;
        if installation_binding_digest(&self.agent_id, &self.owner_user_id, &self.install_id)?
            != self.installation_binding_digest
        {
            bail!("NODE_ENDPOINT_INSTALLATION_BINDING_DIGEST_MISMATCH");
        }
        let issued_at = parse_utc_nanos(&self.issued_at, "NODE_ENDPOINT_ISSUED_AT_INVALID")?;
        let recorded_at = parse_utc_nanos(&self.recorded_at, "NODE_ENDPOINT_RECORDED_AT_INVALID")?;
        if issued_at > recorded_at {
            bail!("NODE_ENDPOINT_ISSUED_AFTER_RECORDED");
        }
        match (
            self.credential_revision,
            self.previous_credential_revision,
            self.previous_credential_digest.as_deref(),
        ) {
            (1, None, None) if self.issuance_kind == "initial_registration" => {}
            (revision, Some(previous), Some(digest))
                if revision > 1
                    && previous.checked_add(1) == Some(revision)
                    && is_sha256(digest)
                    && self.issuance_kind != "initial_registration" => {}
            _ => bail!("NODE_ENDPOINT_CREDENTIAL_PREDECESSOR_INVALID"),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeEndpointCredentialRevocationEnvelope {
    schema: String,
    revocation_id: String,
    credential_id: String,
    credential_revision: u64,
    credential_digest: String,
    agent_id: String,
    owner_user_id: String,
    revocation_kind: String,
    reason_code: String,
    mutation_request_id: String,
    revoked_by_user_id: String,
    owner_authorization_basis: NodeEndpointOwnerAuthorizationBasis,
    revoked_at: String,
    recorded_at: String,
}

impl NodeEndpointCredentialRevocationEnvelope {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        revocation_id: String,
        credential_id: String,
        credential_revision: u64,
        credential_digest: String,
        agent_id: String,
        owner_user_id: String,
        revocation_kind: String,
        reason_code: String,
        mutation_request_id: String,
        revoked_by_user_id: String,
        owner_authorization_basis: NodeEndpointOwnerAuthorizationBasis,
        revoked_at: String,
        recorded_at: String,
    ) -> Result<Self> {
        let envelope = Self {
            schema: REVOCATION_SCHEMA.to_string(),
            revocation_id,
            credential_id,
            credential_revision,
            credential_digest,
            agent_id,
            owner_user_id,
            revocation_kind,
            reason_code,
            mutation_request_id,
            revoked_by_user_id,
            owner_authorization_basis,
            revoked_at,
            recorded_at,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub(crate) fn schema(&self) -> &str {
        &self.schema
    }
    pub(crate) fn revocation_id(&self) -> &str {
        &self.revocation_id
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
    pub(crate) fn revocation_kind(&self) -> &str {
        &self.revocation_kind
    }
    pub(crate) fn reason_code(&self) -> &str {
        &self.reason_code
    }
    pub(crate) fn mutation_request_id(&self) -> &str {
        &self.mutation_request_id
    }
    pub(crate) fn revoked_by_user_id(&self) -> &str {
        &self.revoked_by_user_id
    }
    pub(crate) fn owner_authorization_basis(&self) -> &NodeEndpointOwnerAuthorizationBasis {
        &self.owner_authorization_basis
    }
    pub(crate) fn revoked_at(&self) -> &str {
        &self.revoked_at
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
        ensure_canonical_readback(REVOCATION_DIGEST_DOMAIN, self, stored_json, stored_digest)
    }

    fn validate(&self) -> Result<()> {
        if self.schema != REVOCATION_SCHEMA
            || !bounded_identifier(&self.revocation_id, 160)
            || !bounded_identifier(&self.credential_id, 160)
            || !safe_positive(self.credential_revision)
            || !is_sha256(&self.credential_digest)
            || !bounded_identifier(&self.agent_id, 160)
            || !bounded_identifier(&self.owner_user_id, 160)
            || !matches!(
                self.revocation_kind.as_str(),
                "rotated" | "recovered" | "owner_revoked" | "security_revoked"
            )
            || !bounded_identifier(&self.reason_code, 160)
            || !bounded_identifier(&self.mutation_request_id, 160)
            || !bounded_identifier(&self.revoked_by_user_id, 160)
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_REVOCATION_INVALID");
        }
        self.owner_authorization_basis.validate()?;
        let revoked_at = parse_utc_nanos(&self.revoked_at, "NODE_ENDPOINT_REVOKED_AT_INVALID")?;
        let recorded_at = parse_utc_nanos(&self.recorded_at, "NODE_ENDPOINT_RECORDED_AT_INVALID")?;
        if revoked_at > recorded_at {
            bail!("NODE_ENDPOINT_REVOKED_AFTER_RECORDED");
        }
        Ok(())
    }
}

pub(crate) struct PreparedNodeEndpointCredentialVersion {
    pub(super) envelope: NodeEndpointCredentialVersionEnvelope,
    pub(super) credential_json: String,
    pub(super) credential_digest: String,
    pub(super) secret_hash: String,
}

impl fmt::Debug for PreparedNodeEndpointCredentialVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedNodeEndpointCredentialVersion")
            .field("credential_id", &self.envelope.credential_id)
            .field("credential_revision", &self.envelope.credential_revision)
            .field("credential_digest", &self.credential_digest)
            .field("secret_hash", &"<redacted>")
            .finish()
    }
}

impl PreparedNodeEndpointCredentialVersion {
    pub(crate) fn envelope(&self) -> &NodeEndpointCredentialVersionEnvelope {
        &self.envelope
    }
    pub(crate) fn credential_json(&self) -> &str {
        &self.credential_json
    }
    pub(crate) fn credential_digest(&self) -> &str {
        &self.credential_digest
    }
    pub(crate) fn secret_hash(&self) -> &str {
        &self.secret_hash
    }
    pub(crate) fn canonicalization(&self) -> &'static str {
        CANONICALIZATION
    }
    pub(crate) fn digest_algorithm(&self) -> &'static str {
        DIGEST_ALGORITHM
    }
    pub(crate) fn secret_hash_algorithm(&self) -> &'static str {
        SECRET_HASH_ALGORITHM
    }
}

#[derive(Debug)]
pub(crate) struct PreparedNodeEndpointCredentialRevocation {
    pub(super) envelope: NodeEndpointCredentialRevocationEnvelope,
    pub(super) revocation_json: String,
    pub(super) revocation_digest: String,
}

impl PreparedNodeEndpointCredentialRevocation {
    pub(crate) fn envelope(&self) -> &NodeEndpointCredentialRevocationEnvelope {
        &self.envelope
    }
    pub(crate) fn revocation_json(&self) -> &str {
        &self.revocation_json
    }
    pub(crate) fn revocation_digest(&self) -> &str {
        &self.revocation_digest
    }
    pub(crate) fn canonicalization(&self) -> &'static str {
        CANONICALIZATION
    }
    pub(crate) fn digest_algorithm(&self) -> &'static str {
        DIGEST_ALGORITHM
    }
}
