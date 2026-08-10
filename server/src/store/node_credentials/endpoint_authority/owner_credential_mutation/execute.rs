use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::Transaction;

use crate::node_compute_sharing::endpoint_authority::{
    AuthorizedNodeEndpointCredentialMutation, PresentedNodeEndpointCredentialSecret,
};

use super::super::{credentials, NodeEndpointCredentialMutationReceipt};

pub(super) fn persist_at_on(
    transaction: &Transaction<'_>,
    authorized: &AuthorizedNodeEndpointCredentialMutation,
    presented_endpoint_secret: Option<&PresentedNodeEndpointCredentialSecret>,
    recorded_at: DateTime<Utc>,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    match (authorized, presented_endpoint_secret) {
        (AuthorizedNodeEndpointCredentialMutation::Issue(value), None) => {
            credentials::issue_fresh_at_on(transaction, value, recorded_at)
        }
        (AuthorizedNodeEndpointCredentialMutation::Rotate(value), Some(presented)) => {
            credentials::rotate_at_on(transaction, value, presented, recorded_at)
        }
        (AuthorizedNodeEndpointCredentialMutation::Recover(value), None) => {
            credentials::recover_at_on(transaction, value, recorded_at)
        }
        (AuthorizedNodeEndpointCredentialMutation::Revoke(value), None) => {
            credentials::revoke_at_on(transaction, value, recorded_at)
        }
        _ => bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_POSSESSION_SHAPE_INVALID"),
    }
}
