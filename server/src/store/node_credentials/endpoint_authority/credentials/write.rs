use anyhow::Result;
use rusqlite::{params, Transaction};

use crate::node_compute_sharing::endpoint_authority::{
    PreparedNodeEndpointCredentialRevocation, PreparedNodeEndpointCredentialVersion,
};

pub(super) fn insert_version_on(
    transaction: &Transaction<'_>,
    prepared: &PreparedNodeEndpointCredentialVersion,
) -> Result<()> {
    let envelope = prepared.envelope();
    let basis = envelope.owner_authorization_basis();
    transaction.execute(
        "INSERT INTO node_endpoint_credential_versions (
            credential_id, credential_revision, credential_schema, credential_digest,
            credential_json, canonicalization, digest_algorithm, agent_id, owner_user_id,
            install_id, installation_binding_digest, secret_hash, secret_verifier_digest,
            secret_hash_algorithm, issuance_kind, issuance_request_id, issued_by_user_id,
            owner_authorization_basis_kind, owner_authorization_basis_id,
            owner_authorization_basis_digest, previous_credential_revision,
            previous_credential_digest, issued_at, recorded_at
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,
            ?20,?21,?22,?23,?24
         )",
        params![
            envelope.credential_id(),
            envelope.credential_revision(),
            envelope.schema(),
            prepared.credential_digest(),
            prepared.credential_json(),
            prepared.canonicalization(),
            prepared.digest_algorithm(),
            envelope.agent_id(),
            envelope.owner_user_id(),
            envelope.install_id(),
            envelope.installation_binding_digest(),
            prepared.secret_hash(),
            envelope.secret_verifier_digest(),
            prepared.secret_hash_algorithm(),
            envelope.issuance_kind(),
            envelope.issuance_request_id(),
            envelope.issued_by_user_id(),
            basis.kind(),
            basis.basis_id(),
            basis.basis_digest(),
            envelope.previous_credential_revision(),
            envelope.previous_credential_digest(),
            envelope.issued_at(),
            envelope.recorded_at(),
        ],
    )?;
    Ok(())
}

pub(super) fn insert_revocation_on(
    transaction: &Transaction<'_>,
    prepared: &PreparedNodeEndpointCredentialRevocation,
) -> Result<()> {
    let envelope = prepared.envelope();
    let basis = envelope.owner_authorization_basis();
    transaction.execute(
        "INSERT INTO node_endpoint_credential_revocations (
            revocation_id, revocation_schema, revocation_digest, revocation_json,
            canonicalization, digest_algorithm, credential_id, credential_revision,
            credential_digest, agent_id, owner_user_id, revocation_kind, reason_code,
            mutation_request_id, revoked_by_user_id, owner_authorization_basis_kind,
            owner_authorization_basis_id, owner_authorization_basis_digest, revoked_at, recorded_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        params![
            envelope.revocation_id(),
            envelope.schema(),
            prepared.revocation_digest(),
            prepared.revocation_json(),
            prepared.canonicalization(),
            prepared.digest_algorithm(),
            envelope.credential_id(),
            envelope.credential_revision(),
            envelope.credential_digest(),
            envelope.agent_id(),
            envelope.owner_user_id(),
            envelope.revocation_kind(),
            envelope.reason_code(),
            envelope.mutation_request_id(),
            envelope.revoked_by_user_id(),
            basis.kind(),
            basis.basis_id(),
            basis.basis_digest(),
            envelope.revoked_at(),
            envelope.recorded_at(),
        ],
    )?;
    Ok(())
}
