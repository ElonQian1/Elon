use anyhow::Result;
use rusqlite::TransactionBehavior;

use crate::node_compute_sharing::endpoint_authority::{
    AuthorizedFreshNodeEndpointCredentialIssuance, AuthorizedNodeEndpointCredentialRecovery,
    AuthorizedNodeEndpointCredentialRevocation, AuthorizedNodeEndpointCredentialRotation,
    NodeEndpointCredentialBinding, PresentedNodeEndpointCredentialSecret,
};

use super::{NodeEndpointCredentialMutationReceipt, Store};

mod mutations;
mod root;
mod rows;
mod write;

pub(super) use mutations::{
    issue_fresh_at_on, issue_fresh_on, recover_at_on, recover_on, revoke_at_on, revoke_on,
    rotate_at_on, rotate_on,
};

pub(super) fn issue_fresh(
    store: &Store,
    authorized: &AuthorizedFreshNodeEndpointCredentialIssuance,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let receipt = issue_fresh_on(&transaction, authorized)?;
    transaction.commit()?;
    Ok(receipt)
}

pub(super) fn rotate(
    store: &Store,
    authorized: &AuthorizedNodeEndpointCredentialRotation,
    presented: &PresentedNodeEndpointCredentialSecret,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let receipt = rotate_on(&transaction, authorized, presented)?;
    transaction.commit()?;
    Ok(receipt)
}

pub(super) fn recover(
    store: &Store,
    authorized: &AuthorizedNodeEndpointCredentialRecovery,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let receipt = recover_on(&transaction, authorized)?;
    transaction.commit()?;
    Ok(receipt)
}

pub(super) fn revoke(
    store: &Store,
    authorized: &AuthorizedNodeEndpointCredentialRevocation,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let receipt = revoke_on(&transaction, authorized)?;
    transaction.commit()?;
    Ok(receipt)
}

pub(super) fn authenticate_current_for_agent_on(
    connection: &rusqlite::Connection,
    agent_id: &str,
    presented: &PresentedNodeEndpointCredentialSecret,
) -> Result<NodeEndpointCredentialBinding> {
    let current = root::credential_root_by_agent_on(connection, agent_id)?;
    let Some(current) = current else {
        super::secret::verify_presented_secret(None, presented)?;
        unreachable!("missing credentials never pass constant-time secret verification");
    };
    rows::verify_secret_on(connection, &current, presented)?;
    root::require_expected_active_on(connection, &current)?;
    Ok(current)
}

pub(super) fn verify_bound_secret_on(
    connection: &rusqlite::Connection,
    binding: &NodeEndpointCredentialBinding,
    presented: &PresentedNodeEndpointCredentialSecret,
) -> Result<()> {
    rows::verify_secret_on(connection, binding, presented)
}

pub(super) fn require_current_binding_on(
    connection: &rusqlite::Connection,
    binding: &NodeEndpointCredentialBinding,
) -> Result<()> {
    root::require_expected_active_on(connection, binding)
}

pub(super) fn current_binding_by_agent_on(
    connection: &rusqlite::Connection,
    agent_id: &str,
) -> Result<Option<NodeEndpointCredentialBinding>> {
    root::credential_root_by_agent_on(connection, agent_id)
}

pub(super) fn revocation_for_current_on(
    connection: &rusqlite::Connection,
    current: &NodeEndpointCredentialBinding,
) -> Result<
    Option<(
        crate::node_compute_sharing::endpoint_authority::NodeEndpointCredentialRevocationEnvelope,
        String,
        String,
    )>,
> {
    rows::revocation_for_version_on(
        connection,
        current.credential_id(),
        current.credential_revision(),
    )?
    .map(|stored| {
        let json = stored.revocation_json().to_string();
        let digest = stored.revocation_digest().to_string();
        Ok((stored.into_envelope(), json, digest))
    })
    .transpose()
}
