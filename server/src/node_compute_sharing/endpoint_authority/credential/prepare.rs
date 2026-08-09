use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::super::canonical::{
    canonical_domain_json_and_digest, deterministic_identifier, ensure_time_order,
    installation_binding_digest, secret_verifier_digest, utc_nanos,
};
use super::super::types::{NodeEndpointCredentialBinding, NodeEndpointOwnerAuthorizationBasis};
use super::envelopes::{
    NodeEndpointCredentialRevocationEnvelope, NodeEndpointCredentialVersionEnvelope,
    PreparedNodeEndpointCredentialRevocation, PreparedNodeEndpointCredentialVersion,
    CREDENTIAL_DIGEST_DOMAIN, REVOCATION_DIGEST_DOMAIN,
};

const CREDENTIAL_ID_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_CREDENTIAL_ID_V1";
const REVOCATION_ID_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_CREDENTIAL_REVOCATION_ID_V1";

pub(super) fn fresh_credential_id(
    agent_id: &str,
    owner_user_id: &str,
    install_id: &str,
    issuance_request_id: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Identity<'a> {
        agent_id: &'a str,
        owner_user_id: &'a str,
        install_id: &'a str,
        issuance_request_id: &'a str,
    }
    deterministic_identifier(
        "necred_",
        CREDENTIAL_ID_DOMAIN,
        &Identity {
            agent_id,
            owner_user_id,
            install_id,
            issuance_request_id,
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_version(
    credential_id: String,
    credential_revision: u64,
    agent_id: &str,
    owner_user_id: &str,
    install_id: &str,
    secret_hash: [u8; 32],
    issuance_kind: &str,
    issuance_request_id: &str,
    issued_by_user_id: &str,
    owner_authorization_basis: NodeEndpointOwnerAuthorizationBasis,
    previous_credential_revision: Option<u64>,
    previous_credential_digest: Option<String>,
    issued_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
) -> Result<PreparedNodeEndpointCredentialVersion> {
    ensure_time_order(
        issued_at,
        recorded_at,
        "NODE_ENDPOINT_ISSUED_AFTER_RECORDED",
    )?;
    let envelope = NodeEndpointCredentialVersionEnvelope::new(
        credential_id,
        credential_revision,
        agent_id.to_string(),
        owner_user_id.to_string(),
        install_id.to_string(),
        installation_binding_digest(agent_id, owner_user_id, install_id)?,
        secret_verifier_digest(&secret_hash),
        issuance_kind.to_string(),
        issuance_request_id.to_string(),
        issued_by_user_id.to_string(),
        owner_authorization_basis,
        previous_credential_revision,
        previous_credential_digest,
        utc_nanos(issued_at),
        utc_nanos(recorded_at),
    )?;
    let (credential_json, credential_digest) =
        canonical_domain_json_and_digest(CREDENTIAL_DIGEST_DOMAIN, &envelope)?;
    Ok(PreparedNodeEndpointCredentialVersion {
        envelope,
        credential_json,
        credential_digest,
        secret_hash: hex::encode(secret_hash),
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_revocation(
    expected: &NodeEndpointCredentialBinding,
    revocation_kind: &str,
    reason_code: &str,
    mutation_request_id: &str,
    revoked_by_user_id: &str,
    owner_authorization_basis: NodeEndpointOwnerAuthorizationBasis,
    revoked_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
) -> Result<PreparedNodeEndpointCredentialRevocation> {
    expected.validate()?;
    ensure_time_order(
        revoked_at,
        recorded_at,
        "NODE_ENDPOINT_REVOKED_AFTER_RECORDED",
    )?;
    #[derive(Serialize)]
    struct Identity<'a> {
        credential_id: &'a str,
        credential_revision: u64,
        mutation_request_id: &'a str,
        revocation_kind: &'a str,
    }
    let revocation_id = deterministic_identifier(
        "nerev_",
        REVOCATION_ID_DOMAIN,
        &Identity {
            credential_id: expected.credential_id(),
            credential_revision: expected.credential_revision(),
            mutation_request_id,
            revocation_kind,
        },
    )?;
    let envelope = NodeEndpointCredentialRevocationEnvelope::new(
        revocation_id,
        expected.credential_id().to_string(),
        expected.credential_revision(),
        expected.credential_digest().to_string(),
        expected.agent_id().to_string(),
        expected.owner_user_id().to_string(),
        revocation_kind.to_string(),
        reason_code.to_string(),
        mutation_request_id.to_string(),
        revoked_by_user_id.to_string(),
        owner_authorization_basis,
        utc_nanos(revoked_at),
        utc_nanos(recorded_at),
    )?;
    let (revocation_json, revocation_digest) =
        canonical_domain_json_and_digest(REVOCATION_DIGEST_DOMAIN, &envelope)?;
    Ok(PreparedNodeEndpointCredentialRevocation {
        envelope,
        revocation_json,
        revocation_digest,
    })
}
