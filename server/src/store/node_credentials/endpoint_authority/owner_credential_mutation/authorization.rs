use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::Transaction;

use crate::node_compute_sharing::endpoint_authority::{
    authorize_owner_credential_mutation, AuthorizedNodeEndpointCredentialMutation,
    NodeEndpointCredentialBinding, NodeEndpointCredentialMutationResultBinding,
    NodeEndpointOwnerCredentialMutationRequest, PreparedNodeEndpointOwnerReauthentication,
};

use super::super::credentials;

pub(super) struct PreparedOwnerCredentialMutation {
    authorized: AuthorizedNodeEndpointCredentialMutation,
    result: NodeEndpointCredentialMutationResultBinding,
}

impl PreparedOwnerCredentialMutation {
    pub(super) fn into_parts(
        self,
    ) -> (
        AuthorizedNodeEndpointCredentialMutation,
        NodeEndpointCredentialMutationResultBinding,
    ) {
        (self.authorized, self.result)
    }
}

pub(super) fn prepare_on(
    transaction: &Transaction<'_>,
    owner: &PreparedNodeEndpointOwnerReauthentication,
    request: &NodeEndpointOwnerCredentialMutationRequest,
    expected_current: Option<NodeEndpointCredentialBinding>,
    new_secret_hash: Option<[u8; 32]>,
    recorded_at: DateTime<Utc>,
) -> Result<PreparedOwnerCredentialMutation> {
    let authorized = authorize_owner_credential_mutation(
        owner,
        request,
        expected_current.clone(),
        new_secret_hash,
        recorded_at,
    )?;
    let result = match &authorized {
        AuthorizedNodeEndpointCredentialMutation::Issue(value) => {
            let issued = value.prepare(recorded_at)?;
            let current = current_from_issued(&issued)?;
            NodeEndpointCredentialMutationResultBinding::from_prepared_mutation(
                &current,
                Some(&issued),
                None,
            )?
        }
        AuthorizedNodeEndpointCredentialMutation::Rotate(value) => {
            let issued = value.prepare(recorded_at)?;
            let revocation = value.prepare_revocation(recorded_at)?;
            let current = current_from_issued(&issued)?;
            NodeEndpointCredentialMutationResultBinding::from_prepared_mutation(
                &current,
                Some(&issued),
                Some(&revocation),
            )?
        }
        AuthorizedNodeEndpointCredentialMutation::Recover(value) => {
            let issued = value.prepare(recorded_at)?;
            let current = current_from_issued(&issued)?;
            match value.prepare_revocation(recorded_at)? {
                Some(revocation) => {
                    NodeEndpointCredentialMutationResultBinding::from_prepared_mutation(
                        &current,
                        Some(&issued),
                        Some(&revocation),
                    )?
                }
                None => {
                    let expected = expected_current.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("NODE_ENDPOINT_RECOVERY_CURRENT_BINDING_MISSING")
                    })?;
                    let terminal = credentials::revocation_for_current_on(transaction, expected)?
                        .ok_or_else(|| {
                        anyhow::anyhow!("NODE_ENDPOINT_RECOVERY_TERMINAL_REVOCATION_MISSING")
                    })?;
                    NodeEndpointCredentialMutationResultBinding::from_store_readback(
                        &current,
                        Some((
                            issued.envelope(),
                            issued.credential_json(),
                            issued.credential_digest(),
                        )),
                        Some((&terminal.0, &terminal.1, &terminal.2)),
                    )?
                }
            }
        }
        AuthorizedNodeEndpointCredentialMutation::Revoke(value) => {
            let expected = expected_current.as_ref().ok_or_else(|| {
                anyhow::anyhow!("NODE_ENDPOINT_REVOCATION_CURRENT_BINDING_MISSING")
            })?;
            let current = binding_with_status(expected, "revoked")?;
            let revocation = value.prepare(recorded_at)?;
            NodeEndpointCredentialMutationResultBinding::from_prepared_mutation(
                &current,
                None,
                Some(&revocation),
            )?
        }
    };
    Ok(PreparedOwnerCredentialMutation { authorized, result })
}

fn current_from_issued(
    issued: &crate::node_compute_sharing::endpoint_authority::PreparedNodeEndpointCredentialVersion,
) -> Result<NodeEndpointCredentialBinding> {
    let envelope = issued.envelope();
    NodeEndpointCredentialBinding::from_store_readback(
        envelope.credential_id().to_string(),
        envelope.agent_id().to_string(),
        envelope.owner_user_id().to_string(),
        envelope.install_id().to_string(),
        envelope.installation_binding_digest().to_string(),
        envelope.credential_revision(),
        issued.credential_digest().to_string(),
        "active".to_string(),
    )
}

fn binding_with_status(
    source: &NodeEndpointCredentialBinding,
    status: &str,
) -> Result<NodeEndpointCredentialBinding> {
    if status != "revoked" {
        bail!("NODE_ENDPOINT_CREDENTIAL_RESULT_STATUS_INVALID");
    }
    NodeEndpointCredentialBinding::from_store_readback(
        source.credential_id().to_string(),
        source.agent_id().to_string(),
        source.owner_user_id().to_string(),
        source.install_id().to_string(),
        source.installation_binding_digest().to_string(),
        source.credential_revision(),
        source.credential_digest().to_string(),
        status.to_string(),
    )
}
