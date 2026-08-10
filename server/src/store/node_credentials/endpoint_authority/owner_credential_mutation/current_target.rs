use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::node_compute_sharing::endpoint_authority::{
    NodeEndpointCredentialBinding, NodeEndpointOwnerCredentialMutationRequest,
};

use super::{super::credentials, current_account::CurrentOwnerAccountSource};

pub(super) fn require_current_target_on(
    transaction: &Transaction<'_>,
    account: &CurrentOwnerAccountSource,
    request: &NodeEndpointOwnerCredentialMutationRequest,
) -> Result<Option<NodeEndpointCredentialBinding>> {
    let legacy = transaction
        .query_row(
            "SELECT owner_user_id, install_id FROM node_credentials WHERE agent_id=?1",
            params![request.agent_id()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_TARGET_NOT_FOUND"))?;
    if legacy.0 != account.owner_user_id() || legacy.1.as_deref() != Some(request.install_id()) {
        bail!("NODE_ENDPOINT_OWNER_TARGET_NOT_FOUND");
    }

    let current = credentials::current_binding_by_agent_on(transaction, request.agent_id())?;
    match (request.authorization_action(), request.expected(), current) {
        ("initial_registration", None, None) => {
            let owner_install_exists = transaction.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM node_endpoint_credentials
                     WHERE owner_user_id=?1 AND install_id=?2
                 )",
                params![account.owner_user_id(), request.install_id()],
                |row| row.get::<_, bool>(0),
            )?;
            if owner_install_exists {
                bail!("NODE_ENDPOINT_OWNER_TARGET_NOT_FOUND");
            }
            Ok(None)
        }
        (action, Some(expected), Some(current))
            if expected.credential_id() == current.credential_id()
                && expected.credential_revision() == current.credential_revision()
                && expected.credential_digest() == current.credential_digest()
                && current.owner_user_id() == account.owner_user_id()
                && current.install_id() == request.install_id()
                && ((action == "account_recovery"
                    && matches!(current.status(), "active" | "revoked"))
                    || (matches!(action, "credential_rotation" | "owner_revocation")
                        && current.status() == "active")) =>
        {
            Ok(Some(current))
        }
        _ => bail!("NODE_ENDPOINT_OWNER_TARGET_NOT_FOUND"),
    }
}
