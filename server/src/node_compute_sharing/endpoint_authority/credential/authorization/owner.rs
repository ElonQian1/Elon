use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::*;
use crate::node_compute_sharing::endpoint_authority::{
    NodeEndpointOwnerCredentialMutationRequest, PreparedNodeEndpointOwnerReauthentication,
};

pub(crate) enum AuthorizedNodeEndpointCredentialMutation {
    Issue(AuthorizedFreshNodeEndpointCredentialIssuance),
    Rotate(AuthorizedNodeEndpointCredentialRotation),
    Recover(AuthorizedNodeEndpointCredentialRecovery),
    Revoke(AuthorizedNodeEndpointCredentialRevocation),
}

pub(crate) fn authorize_owner_credential_mutation(
    owner: &PreparedNodeEndpointOwnerReauthentication,
    request: &NodeEndpointOwnerCredentialMutationRequest,
    expected_current: Option<NodeEndpointCredentialBinding>,
    new_secret_hash: Option<[u8; 32]>,
    authorized_at: DateTime<Utc>,
) -> Result<AuthorizedNodeEndpointCredentialMutation> {
    let envelope = owner.envelope();
    let (_, request_digest) = request.canonical_json_and_digest()?;
    if envelope.authorization_action() != request.authorization_action()
        || envelope.authorization_issuance_request_id()
            != request.authorization_issuance_request_id()
        || envelope.credential_mutation_request_id() != request.credential_mutation_request_id()
        || envelope.credential_mutation_request_digest() != request_digest
        || envelope.agent_id() != request.agent_id()
        || envelope.install_id() != request.install_id()
    {
        bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_AUTHORIZATION_REQUEST_MISMATCH");
    }
    let expires_at = DateTime::parse_from_rfc3339(envelope.expires_at())?.with_timezone(&Utc);
    let recorded_at = DateTime::parse_from_rfc3339(envelope.recorded_at())?.with_timezone(&Utc);
    if authorized_at < recorded_at || authorized_at >= expires_at {
        bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_AUTHORIZATION_EXPIRED");
    }
    require_expected_current(request, envelope, expected_current.as_ref())?;
    let basis = NodeEndpointOwnerAuthorizationBasis::recent_reauthentication(
        envelope.reauthentication_receipt_id().to_string(),
        owner.receipt_digest().to_string(),
    )?;
    let owner_user_id = envelope.owner_user_id().to_string();
    let mutation_request_id = request.credential_mutation_request_id().to_string();

    match request.authorization_action() {
        "initial_registration" => {
            let secret_hash = new_secret_hash
                .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_CREDENTIAL_SECRET_REQUIRED"))?;
            Ok(AuthorizedNodeEndpointCredentialMutation::Issue(
                AuthorizedFreshNodeEndpointCredentialIssuance {
                    agent_id: request.agent_id().to_string(),
                    owner_user_id: owner_user_id.clone(),
                    install_id: request.install_id().to_string(),
                    new_secret_hash: secret_hash,
                    issuance_request_id: mutation_request_id,
                    issued_by_user_id: owner_user_id,
                    owner_authorization_basis: basis,
                    issued_at: authorized_at,
                },
            ))
        }
        "credential_rotation" => {
            let expected = expected_current.ok_or_else(|| {
                anyhow::anyhow!("NODE_ENDPOINT_OWNER_EXPECTED_CREDENTIAL_REQUIRED")
            })?;
            let secret_hash = new_secret_hash
                .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_CREDENTIAL_SECRET_REQUIRED"))?;
            Ok(AuthorizedNodeEndpointCredentialMutation::Rotate(
                AuthorizedNodeEndpointCredentialRotation {
                    expected,
                    new_secret_hash: secret_hash,
                    issuance_request_id: mutation_request_id,
                    issued_by_user_id: owner_user_id,
                    owner_authorization_basis: basis,
                    issued_at: authorized_at,
                },
            ))
        }
        "account_recovery" => {
            let expected = expected_current.ok_or_else(|| {
                anyhow::anyhow!("NODE_ENDPOINT_OWNER_EXPECTED_CREDENTIAL_REQUIRED")
            })?;
            let secret_hash = new_secret_hash
                .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_CREDENTIAL_SECRET_REQUIRED"))?;
            Ok(AuthorizedNodeEndpointCredentialMutation::Recover(
                AuthorizedNodeEndpointCredentialRecovery {
                    expected,
                    new_secret_hash: secret_hash,
                    issuance_request_id: mutation_request_id,
                    issued_by_user_id: owner_user_id,
                    owner_authorization_basis: basis,
                    issued_at: authorized_at,
                },
            ))
        }
        "owner_revocation" => {
            if new_secret_hash.is_some() {
                bail!("NODE_ENDPOINT_OWNER_REVOCATION_SECRET_FORBIDDEN");
            }
            let expected = expected_current.ok_or_else(|| {
                anyhow::anyhow!("NODE_ENDPOINT_OWNER_EXPECTED_CREDENTIAL_REQUIRED")
            })?;
            Ok(AuthorizedNodeEndpointCredentialMutation::Revoke(
                AuthorizedNodeEndpointCredentialRevocation {
                    expected,
                    revocation_kind: "owner_revoked".to_string(),
                    reason_code: request
                        .reason_code()
                        .ok_or_else(|| {
                            anyhow::anyhow!("NODE_ENDPOINT_OWNER_REVOCATION_REASON_REQUIRED")
                        })?
                        .to_string(),
                    mutation_request_id,
                    revoked_by_user_id: owner_user_id,
                    owner_authorization_basis: basis,
                    revoked_at: authorized_at,
                },
            ))
        }
        _ => bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_ACTION_INVALID"),
    }
}

fn require_expected_current(
    request: &NodeEndpointOwnerCredentialMutationRequest,
    envelope: &crate::node_compute_sharing::endpoint_authority::NodeEndpointOwnerReauthenticationEnvelope,
    current: Option<&NodeEndpointCredentialBinding>,
) -> Result<()> {
    match (request.expected(), current) {
        (None, None)
            if envelope.expected_credential_id().is_none()
                && envelope.expected_credential_revision().is_none()
                && envelope.expected_credential_digest().is_none() =>
        {
            Ok(())
        }
        (Some(expected), Some(current))
            if expected.credential_id() == current.credential_id()
                && expected.credential_revision() == current.credential_revision()
                && expected.credential_digest() == current.credential_digest()
                && envelope.expected_credential_id() == Some(current.credential_id())
                && envelope.expected_credential_revision()
                    == Some(current.credential_revision())
                && envelope.expected_credential_digest() == Some(current.credential_digest())
                && current.owner_user_id() == envelope.owner_user_id()
                && current.agent_id() == envelope.agent_id()
                && current.install_id() == envelope.install_id() =>
        {
            Ok(())
        }
        _ => bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_EXPECTED_CURRENT_MISMATCH"),
    }
}
