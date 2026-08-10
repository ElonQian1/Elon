use anyhow::{bail, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use super::prepare::{build_revocation, build_version, fresh_credential_id};
use super::{PreparedNodeEndpointCredentialRevocation, PreparedNodeEndpointCredentialVersion};
use crate::node_compute_sharing::endpoint_authority::types::{
    NodeEndpointCredentialBinding, NodeEndpointOwnerAuthorizationBasis, MAX_IJSON_SAFE_INTEGER,
};

pub(crate) struct PresentedNodeEndpointCredentialSecret {
    secret_hash: [u8; 32],
}

struct ZeroizingBytes(Vec<u8>);

impl ZeroizingBytes {
    fn as_slice(&self) -> &[u8] {
        &self.0
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl Drop for ZeroizingBytes {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

impl PresentedNodeEndpointCredentialSecret {
    pub(crate) fn from_secret_hash(secret_hash: [u8; 32]) -> Self {
        Self { secret_hash }
    }

    pub(crate) fn from_endpoint_bearer(endpoint_bearer: String) -> Result<Self> {
        let endpoint_bearer = ZeroizingBytes(endpoint_bearer.into_bytes());
        if endpoint_bearer.0.len() != 43 || !endpoint_bearer.as_slice().is_ascii() {
            bail!("NODE_ENDPOINT_CREDENTIAL_BEARER_INVALID");
        }
        let decoded = ZeroizingBytes(
            URL_SAFE_NO_PAD
                .decode(endpoint_bearer.as_slice())
                .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_BEARER_INVALID"))?,
        );
        let mut canonical = ZeroizingBytes(vec![0_u8; 43]);
        let encoded_len = URL_SAFE_NO_PAD
            .encode_slice(decoded.as_slice(), canonical.as_mut_slice())
            .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_BEARER_INVALID"))?;
        if decoded.0.len() != 32
            || encoded_len != endpoint_bearer.0.len()
            || canonical.as_slice() != endpoint_bearer.as_slice()
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_BEARER_INVALID");
        }
        Ok(Self {
            secret_hash: Sha256::digest(endpoint_bearer.as_slice()).into(),
        })
    }

    pub(crate) fn secret_hash(&self) -> &[u8; 32] {
        &self.secret_hash
    }
}

mod owner;

pub(crate) use owner::{
    authorize_owner_credential_mutation, AuthorizedNodeEndpointCredentialMutation,
};

pub(crate) struct AuthorizedFreshNodeEndpointCredentialIssuance {
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    new_secret_hash: [u8; 32],
    issuance_request_id: String,
    issued_by_user_id: String,
    owner_authorization_basis: NodeEndpointOwnerAuthorizationBasis,
    issued_at: DateTime<Utc>,
}

impl AuthorizedFreshNodeEndpointCredentialIssuance {
    pub(crate) fn agent_id(&self) -> &str {
        &self.agent_id
    }
    pub(crate) fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }
    pub(crate) fn install_id(&self) -> &str {
        &self.install_id
    }
    pub(crate) fn issuance_request_id(&self) -> &str {
        &self.issuance_request_id
    }

    pub(crate) fn prepare(
        &self,
        recorded_at: DateTime<Utc>,
    ) -> Result<PreparedNodeEndpointCredentialVersion> {
        let credential_id = fresh_credential_id(
            &self.agent_id,
            &self.owner_user_id,
            &self.install_id,
            &self.issuance_request_id,
        )?;
        build_version(
            credential_id,
            1,
            &self.agent_id,
            &self.owner_user_id,
            &self.install_id,
            self.new_secret_hash,
            "initial_registration",
            &self.issuance_request_id,
            &self.issued_by_user_id,
            self.owner_authorization_basis.clone(),
            None,
            None,
            self.issued_at,
            recorded_at,
        )
    }
}

pub(crate) struct AuthorizedNodeEndpointCredentialRotation {
    expected: NodeEndpointCredentialBinding,
    new_secret_hash: [u8; 32],
    issuance_request_id: String,
    issued_by_user_id: String,
    owner_authorization_basis: NodeEndpointOwnerAuthorizationBasis,
    issued_at: DateTime<Utc>,
}

impl AuthorizedNodeEndpointCredentialRotation {
    pub(crate) fn expected(&self) -> &NodeEndpointCredentialBinding {
        &self.expected
    }
    pub(crate) fn issuance_request_id(&self) -> &str {
        &self.issuance_request_id
    }
    pub(crate) fn prepare(
        &self,
        recorded_at: DateTime<Utc>,
    ) -> Result<PreparedNodeEndpointCredentialVersion> {
        let revision = successor_revision(&self.expected)?;
        build_version(
            self.expected.credential_id().to_string(),
            revision,
            self.expected.agent_id(),
            self.expected.owner_user_id(),
            self.expected.install_id(),
            self.new_secret_hash,
            "credential_rotation",
            &self.issuance_request_id,
            &self.issued_by_user_id,
            self.owner_authorization_basis.clone(),
            Some(self.expected.credential_revision()),
            Some(self.expected.credential_digest().to_string()),
            self.issued_at,
            recorded_at,
        )
    }
    pub(crate) fn prepare_revocation(
        &self,
        recorded_at: DateTime<Utc>,
    ) -> Result<PreparedNodeEndpointCredentialRevocation> {
        build_revocation(
            &self.expected,
            "rotated",
            "credential_rotation",
            &self.issuance_request_id,
            &self.issued_by_user_id,
            self.owner_authorization_basis.clone(),
            self.issued_at,
            recorded_at,
        )
    }
}

pub(crate) struct AuthorizedNodeEndpointCredentialRecovery {
    expected: NodeEndpointCredentialBinding,
    new_secret_hash: [u8; 32],
    issuance_request_id: String,
    issued_by_user_id: String,
    owner_authorization_basis: NodeEndpointOwnerAuthorizationBasis,
    issued_at: DateTime<Utc>,
}

impl AuthorizedNodeEndpointCredentialRecovery {
    pub(crate) fn expected(&self) -> &NodeEndpointCredentialBinding {
        &self.expected
    }
    pub(crate) fn issuance_request_id(&self) -> &str {
        &self.issuance_request_id
    }
    pub(crate) fn prepare(
        &self,
        recorded_at: DateTime<Utc>,
    ) -> Result<PreparedNodeEndpointCredentialVersion> {
        let revision = successor_revision(&self.expected)?;
        build_version(
            self.expected.credential_id().to_string(),
            revision,
            self.expected.agent_id(),
            self.expected.owner_user_id(),
            self.expected.install_id(),
            self.new_secret_hash,
            "account_recovery",
            &self.issuance_request_id,
            &self.issued_by_user_id,
            self.owner_authorization_basis.clone(),
            Some(self.expected.credential_revision()),
            Some(self.expected.credential_digest().to_string()),
            self.issued_at,
            recorded_at,
        )
    }
    pub(crate) fn prepare_revocation(
        &self,
        recorded_at: DateTime<Utc>,
    ) -> Result<Option<PreparedNodeEndpointCredentialRevocation>> {
        match self.expected.status() {
            "active" => build_revocation(
                &self.expected,
                "recovered",
                "account_recovery",
                &self.issuance_request_id,
                &self.issued_by_user_id,
                self.owner_authorization_basis.clone(),
                self.issued_at,
                recorded_at,
            )
            .map(Some),
            "revoked" => Ok(None),
            _ => bail!("NODE_ENDPOINT_RECOVERY_SOURCE_INVALID"),
        }
    }
}

pub(crate) struct AuthorizedNodeEndpointCredentialRevocation {
    expected: NodeEndpointCredentialBinding,
    revocation_kind: String,
    reason_code: String,
    mutation_request_id: String,
    revoked_by_user_id: String,
    owner_authorization_basis: NodeEndpointOwnerAuthorizationBasis,
    revoked_at: DateTime<Utc>,
}

impl AuthorizedNodeEndpointCredentialRevocation {
    pub(crate) fn expected(&self) -> &NodeEndpointCredentialBinding {
        &self.expected
    }
    pub(crate) fn mutation_request_id(&self) -> &str {
        &self.mutation_request_id
    }
    pub(crate) fn prepare(
        &self,
        recorded_at: DateTime<Utc>,
    ) -> Result<PreparedNodeEndpointCredentialRevocation> {
        if !matches!(
            self.revocation_kind.as_str(),
            "owner_revoked" | "security_revoked"
        ) {
            bail!("NODE_ENDPOINT_REVOCATION_KIND_INVALID");
        }
        build_revocation(
            &self.expected,
            &self.revocation_kind,
            &self.reason_code,
            &self.mutation_request_id,
            &self.revoked_by_user_id,
            self.owner_authorization_basis.clone(),
            self.revoked_at,
            recorded_at,
        )
    }
}

fn successor_revision(binding: &NodeEndpointCredentialBinding) -> Result<u64> {
    binding.validate()?;
    binding
        .credential_revision()
        .checked_add(1)
        .filter(|value| *value <= MAX_IJSON_SAFE_INTEGER)
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_REVISION_EXHAUSTED"))
}
