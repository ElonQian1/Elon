use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::node_compute_sharing::endpoint_authority::{
    derive_node_endpoint_secret_verifier_digest, NodeEndpointCredentialBinding,
    NodeEndpointCredentialRevocationEnvelope, NodeEndpointCredentialVersionEnvelope,
    PreparedNodeEndpointCredentialRevocation, PreparedNodeEndpointCredentialVersion,
    PresentedNodeEndpointCredentialSecret,
};

use super::super::secret::{ensure_secret_hash_exact, verify_presented_secret};

pub(super) struct StoredCredentialVersion {
    envelope: NodeEndpointCredentialVersionEnvelope,
    credential_json: String,
    credential_digest: String,
    secret_hash: String,
    canonicalization: String,
    digest_algorithm: String,
    secret_hash_algorithm: String,
}

impl StoredCredentialVersion {
    pub(super) fn recorded_at(&self) -> &str {
        self.envelope.recorded_at()
    }

    pub(super) fn envelope(&self) -> &NodeEndpointCredentialVersionEnvelope {
        &self.envelope
    }

    pub(super) fn ensure_binding(&self, binding: &NodeEndpointCredentialBinding) -> Result<()> {
        if self.envelope.credential_id() != binding.credential_id()
            || self.envelope.credential_revision() != binding.credential_revision()
            || self.credential_digest != binding.credential_digest()
            || self.envelope.agent_id() != binding.agent_id()
            || self.envelope.owner_user_id() != binding.owner_user_id()
            || self.envelope.install_id() != binding.install_id()
            || self.envelope.installation_binding_digest() != binding.installation_binding_digest()
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_VERSION_BINDING_MISMATCH");
        }
        Ok(())
    }

    pub(super) fn into_envelope(self) -> NodeEndpointCredentialVersionEnvelope {
        self.envelope
    }

    pub(super) fn ensure_exact(
        &self,
        prepared: &PreparedNodeEndpointCredentialVersion,
    ) -> Result<()> {
        if &self.envelope != prepared.envelope()
            || self.credential_json != prepared.credential_json()
            || self.credential_digest != prepared.credential_digest()
            || self.canonicalization != prepared.canonicalization()
            || self.digest_algorithm != prepared.digest_algorithm()
            || self.secret_hash_algorithm != prepared.secret_hash_algorithm()
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_REPLAY_MISMATCH");
        }
        ensure_secret_hash_exact(&self.secret_hash, prepared.secret_hash())
    }
}

pub(super) struct StoredCredentialRevocation {
    envelope: NodeEndpointCredentialRevocationEnvelope,
    revocation_json: String,
    revocation_digest: String,
    canonicalization: String,
    digest_algorithm: String,
}

impl StoredCredentialRevocation {
    pub(super) fn envelope(&self) -> &NodeEndpointCredentialRevocationEnvelope {
        &self.envelope
    }

    pub(super) fn recorded_at(&self) -> &str {
        self.envelope.recorded_at()
    }

    pub(super) fn into_envelope(self) -> NodeEndpointCredentialRevocationEnvelope {
        self.envelope
    }

    pub(super) fn ensure_exact(
        &self,
        prepared: &PreparedNodeEndpointCredentialRevocation,
    ) -> Result<()> {
        if &self.envelope != prepared.envelope()
            || self.revocation_json != prepared.revocation_json()
            || self.revocation_digest != prepared.revocation_digest()
            || self.canonicalization != prepared.canonicalization()
            || self.digest_algorithm != prepared.digest_algorithm()
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_REVOCATION_REPLAY_MISMATCH");
        }
        Ok(())
    }
}

struct RawCredentialVersion {
    credential_id: String,
    credential_revision: i64,
    credential_schema: String,
    credential_digest: String,
    credential_json: String,
    canonicalization: String,
    digest_algorithm: String,
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    installation_binding_digest: String,
    secret_hash: String,
    secret_verifier_digest: String,
    secret_hash_algorithm: String,
    issuance_kind: String,
    issuance_request_id: String,
    issued_by_user_id: String,
    basis_kind: String,
    basis_id: String,
    basis_digest: String,
    previous_revision: Option<i64>,
    previous_digest: Option<String>,
    issued_at: String,
    recorded_at: String,
}

impl RawCredentialVersion {
    fn validate(self) -> Result<StoredCredentialVersion> {
        let envelope: NodeEndpointCredentialVersionEnvelope =
            serde_json::from_str(&self.credential_json)?;
        envelope.validate_store_readback(&self.credential_json, &self.credential_digest)?;
        let previous_revision = self.previous_revision.map(u64::try_from).transpose()?;
        let basis = envelope.owner_authorization_basis();
        let mut secret_hash = [0_u8; 32];
        if hex::decode_to_slice(&self.secret_hash, &mut secret_hash).is_err()
            || hex::encode(secret_hash) != self.secret_hash
            || derive_node_endpoint_secret_verifier_digest(&secret_hash)
                != envelope.secret_verifier_digest()
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_SECRET_VERIFIER_READBACK_MISMATCH");
        }
        if self.credential_id != envelope.credential_id()
            || u64::try_from(self.credential_revision)? != envelope.credential_revision()
            || self.credential_schema != envelope.schema()
            || self.agent_id != envelope.agent_id()
            || self.owner_user_id != envelope.owner_user_id()
            || self.install_id != envelope.install_id()
            || self.installation_binding_digest != envelope.installation_binding_digest()
            || self.secret_verifier_digest != envelope.secret_verifier_digest()
            || self.canonicalization != "rfc8785_jcs"
            || self.digest_algorithm != "sha256"
            || self.secret_hash_algorithm != "sha256"
            || self.issuance_kind != envelope.issuance_kind()
            || self.issuance_request_id != envelope.issuance_request_id()
            || self.issued_by_user_id != envelope.issued_by_user_id()
            || self.basis_kind != basis.kind()
            || self.basis_id != basis.basis_id()
            || self.basis_digest != basis.basis_digest()
            || previous_revision != envelope.previous_credential_revision()
            || self.previous_digest.as_deref() != envelope.previous_credential_digest()
            || self.issued_at != envelope.issued_at()
            || self.recorded_at != envelope.recorded_at()
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_PROJECTION_READBACK_MISMATCH");
        }
        Ok(StoredCredentialVersion {
            envelope,
            credential_json: self.credential_json,
            credential_digest: self.credential_digest,
            secret_hash: self.secret_hash,
            canonicalization: self.canonicalization,
            digest_algorithm: self.digest_algorithm,
            secret_hash_algorithm: self.secret_hash_algorithm,
        })
    }
}

struct RawCredentialRevocation {
    revocation_id: String,
    revocation_schema: String,
    revocation_digest: String,
    revocation_json: String,
    canonicalization: String,
    digest_algorithm: String,
    credential_id: String,
    credential_revision: i64,
    credential_digest: String,
    agent_id: String,
    owner_user_id: String,
    revocation_kind: String,
    reason_code: String,
    mutation_request_id: String,
    revoked_by_user_id: String,
    basis_kind: String,
    basis_id: String,
    basis_digest: String,
    revoked_at: String,
    recorded_at: String,
}

impl RawCredentialRevocation {
    fn validate(self) -> Result<StoredCredentialRevocation> {
        let envelope: NodeEndpointCredentialRevocationEnvelope =
            serde_json::from_str(&self.revocation_json)?;
        envelope.validate_store_readback(&self.revocation_json, &self.revocation_digest)?;
        let basis = envelope.owner_authorization_basis();
        if self.revocation_id != envelope.revocation_id()
            || self.revocation_schema != envelope.schema()
            || self.canonicalization != "rfc8785_jcs"
            || self.digest_algorithm != "sha256"
            || self.credential_id != envelope.credential_id()
            || u64::try_from(self.credential_revision)? != envelope.credential_revision()
            || self.credential_digest != envelope.credential_digest()
            || self.agent_id != envelope.agent_id()
            || self.owner_user_id != envelope.owner_user_id()
            || self.revocation_kind != envelope.revocation_kind()
            || self.reason_code != envelope.reason_code()
            || self.mutation_request_id != envelope.mutation_request_id()
            || self.revoked_by_user_id != envelope.revoked_by_user_id()
            || self.basis_kind != basis.kind()
            || self.basis_id != basis.basis_id()
            || self.basis_digest != basis.basis_digest()
            || self.revoked_at != envelope.revoked_at()
            || self.recorded_at != envelope.recorded_at()
        {
            bail!("NODE_ENDPOINT_REVOCATION_PROJECTION_READBACK_MISMATCH");
        }
        Ok(StoredCredentialRevocation {
            envelope,
            revocation_json: self.revocation_json,
            revocation_digest: self.revocation_digest,
            canonicalization: self.canonicalization,
            digest_algorithm: self.digest_algorithm,
        })
    }
}

pub(super) fn credential_root_on(
    connection: &Connection,
    credential_id: &str,
) -> Result<Option<NodeEndpointCredentialBinding>> {
    let raw = connection
        .query_row(
            "SELECT credential_id, agent_id, owner_user_id, install_id,
                    installation_binding_digest, current_credential_revision,
                    current_credential_digest, status
               FROM node_endpoint_credentials WHERE credential_id=?1",
            params![credential_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()?;
    raw.map(|value| {
        NodeEndpointCredentialBinding::from_store_readback(
            value.0,
            value.1,
            value.2,
            value.3,
            value.4,
            u64::try_from(value.5)?,
            value.6,
            value.7,
        )
    })
    .transpose()
}

pub(super) fn version_by_issuance_on(
    connection: &Connection,
    credential_id: &str,
    issuance_request_id: &str,
) -> Result<Option<StoredCredentialVersion>> {
    query_version_on(
        connection,
        "WHERE credential_id=?1 AND issuance_request_id=?2",
        params![credential_id, issuance_request_id],
    )
}

pub(super) fn fresh_version_by_issuance_on(
    connection: &Connection,
    agent_id: &str,
    owner_user_id: &str,
    install_id: &str,
    issuance_request_id: &str,
) -> Result<Option<StoredCredentialVersion>> {
    query_version_on(
        connection,
        "WHERE agent_id=?1 AND owner_user_id=?2 AND install_id=?3
           AND issuance_request_id=?4",
        params![agent_id, owner_user_id, install_id, issuance_request_id],
    )
}

pub(super) fn version_exact_on(
    connection: &Connection,
    binding: &NodeEndpointCredentialBinding,
) -> Result<Option<StoredCredentialVersion>> {
    query_version_on(
        connection,
        "WHERE credential_id=?1 AND credential_revision=?2 AND credential_digest=?3",
        params![
            binding.credential_id(),
            binding.credential_revision(),
            binding.credential_digest()
        ],
    )
}

pub(super) fn version_revision_on(
    connection: &Connection,
    credential_id: &str,
    credential_revision: u64,
) -> Result<Option<StoredCredentialVersion>> {
    query_version_on(
        connection,
        "WHERE credential_id=?1 AND credential_revision=?2",
        params![credential_id, credential_revision],
    )
}

fn query_version_on<P: rusqlite::Params>(
    connection: &Connection,
    predicate: &str,
    parameters: P,
) -> Result<Option<StoredCredentialVersion>> {
    let sql = format!(
        "SELECT credential_id, credential_revision, credential_schema, credential_digest,
                credential_json, canonicalization,
                digest_algorithm, agent_id, owner_user_id, install_id,
                installation_binding_digest, secret_hash, secret_verifier_digest,
                secret_hash_algorithm, issuance_kind, issuance_request_id,
                issued_by_user_id, owner_authorization_basis_kind,
                owner_authorization_basis_id, owner_authorization_basis_digest,
                previous_credential_revision, previous_credential_digest, issued_at, recorded_at
           FROM node_endpoint_credential_versions {predicate}"
    );
    connection
        .query_row(&sql, parameters, map_version)
        .optional()?
        .map(RawCredentialVersion::validate)
        .transpose()
}

fn map_version(row: &Row<'_>) -> rusqlite::Result<RawCredentialVersion> {
    Ok(RawCredentialVersion {
        credential_id: row.get(0)?,
        credential_revision: row.get(1)?,
        credential_schema: row.get(2)?,
        credential_digest: row.get(3)?,
        credential_json: row.get(4)?,
        canonicalization: row.get(5)?,
        digest_algorithm: row.get(6)?,
        agent_id: row.get(7)?,
        owner_user_id: row.get(8)?,
        install_id: row.get(9)?,
        installation_binding_digest: row.get(10)?,
        secret_hash: row.get(11)?,
        secret_verifier_digest: row.get(12)?,
        secret_hash_algorithm: row.get(13)?,
        issuance_kind: row.get(14)?,
        issuance_request_id: row.get(15)?,
        issued_by_user_id: row.get(16)?,
        basis_kind: row.get(17)?,
        basis_id: row.get(18)?,
        basis_digest: row.get(19)?,
        previous_revision: row.get(20)?,
        previous_digest: row.get(21)?,
        issued_at: row.get(22)?,
        recorded_at: row.get(23)?,
    })
}

pub(super) fn revocation_for_version_on(
    connection: &Connection,
    credential_id: &str,
    revision: u64,
) -> Result<Option<StoredCredentialRevocation>> {
    connection
        .query_row(
            "SELECT revocation_id, revocation_schema, revocation_digest, revocation_json,
                    canonicalization,
                    digest_algorithm, credential_id, credential_revision, credential_digest,
                    agent_id, owner_user_id, revocation_kind, reason_code, mutation_request_id,
                    revoked_by_user_id, owner_authorization_basis_kind,
                    owner_authorization_basis_id, owner_authorization_basis_digest,
                    revoked_at, recorded_at
               FROM node_endpoint_credential_revocations
              WHERE credential_id=?1 AND credential_revision=?2",
            params![credential_id, revision],
            |row| {
                Ok(RawCredentialRevocation {
                    revocation_id: row.get(0)?,
                    revocation_schema: row.get(1)?,
                    revocation_digest: row.get(2)?,
                    revocation_json: row.get(3)?,
                    canonicalization: row.get(4)?,
                    digest_algorithm: row.get(5)?,
                    credential_id: row.get(6)?,
                    credential_revision: row.get(7)?,
                    credential_digest: row.get(8)?,
                    agent_id: row.get(9)?,
                    owner_user_id: row.get(10)?,
                    revocation_kind: row.get(11)?,
                    reason_code: row.get(12)?,
                    mutation_request_id: row.get(13)?,
                    revoked_by_user_id: row.get(14)?,
                    basis_kind: row.get(15)?,
                    basis_id: row.get(16)?,
                    basis_digest: row.get(17)?,
                    revoked_at: row.get(18)?,
                    recorded_at: row.get(19)?,
                })
            },
        )
        .optional()?
        .map(RawCredentialRevocation::validate)
        .transpose()
}

pub(super) fn verify_secret_on(
    connection: &Connection,
    expected: &NodeEndpointCredentialBinding,
    presented: &PresentedNodeEndpointCredentialSecret,
) -> Result<()> {
    let version = version_exact_on(connection, expected)?;
    verify_presented_secret(
        version.as_ref().map(|value| value.secret_hash.as_str()),
        presented,
    )?;
    let version =
        version.ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_AUTHENTICATION_FAILED"))?;
    let envelope = version.envelope();
    if envelope.agent_id() != expected.agent_id()
        || envelope.owner_user_id() != expected.owner_user_id()
        || envelope.install_id() != expected.install_id()
        || envelope.installation_binding_digest() != expected.installation_binding_digest()
    {
        bail!("NODE_ENDPOINT_CREDENTIAL_AUTHENTICATION_FAILED");
    }
    Ok(())
}
