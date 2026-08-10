use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::node_compute_sharing::endpoint_authority::{
    AuthorizedNodeEndpointCredentialRotation, PresentedNodeEndpointCredentialSecret,
};

use super::super::super::{credential_receipt, NodeEndpointCredentialMutationReceipt};
use super::super::rows;
use super::replace::replace_current;
use super::support::{parse_recorded_at, require_exact_revocation, require_root_for_version};

pub(in crate::store::node_credentials::endpoint_authority) fn rotate_on(
    transaction: &rusqlite::Transaction<'_>,
    authorized: &AuthorizedNodeEndpointCredentialRotation,
    presented: &PresentedNodeEndpointCredentialSecret,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    rotate_at_on(transaction, authorized, presented, Utc::now())
}

pub(in crate::store::node_credentials::endpoint_authority) fn rotate_at_on(
    transaction: &rusqlite::Transaction<'_>,
    authorized: &AuthorizedNodeEndpointCredentialRotation,
    presented: &PresentedNodeEndpointCredentialSecret,
    recorded_at: DateTime<Utc>,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    rows::verify_secret_on(transaction, authorized.expected(), presented)?;

    if let Some(stored) = rows::version_by_issuance_on(
        transaction,
        authorized.expected().credential_id(),
        authorized.issuance_request_id(),
    )? {
        let recorded_at = parse_recorded_at(stored.recorded_at())?;
        let prepared = authorized.prepare(recorded_at)?;
        let revocation = authorized.prepare_revocation(recorded_at)?;
        stored.ensure_exact(&prepared)?;
        let stored_revocation = require_exact_revocation(transaction, &revocation)?;
        let current = require_root_for_version(transaction, &prepared)?;
        return Ok(credential_receipt(
            current,
            Some(stored.into_envelope()),
            Some(stored_revocation.into_envelope()),
            true,
        ));
    }

    let prepared = authorized.prepare(recorded_at)?;
    let revocation = authorized.prepare_revocation(recorded_at)?;
    replace_current(transaction, authorized.expected(), &prepared, &revocation)
}
