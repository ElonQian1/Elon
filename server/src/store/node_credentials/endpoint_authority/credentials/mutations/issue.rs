use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::node_compute_sharing::endpoint_authority::AuthorizedFreshNodeEndpointCredentialIssuance;

use super::super::super::{credential_receipt, NodeEndpointCredentialMutationReceipt};
use super::super::{root, rows, write};
use super::support::{ensure_current_version, parse_recorded_at, require_root_for_version};

pub(in crate::store::node_credentials::endpoint_authority) fn issue_fresh_on(
    transaction: &rusqlite::Transaction<'_>,
    authorized: &AuthorizedFreshNodeEndpointCredentialIssuance,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    issue_fresh_at_on(transaction, authorized, Utc::now())
}

pub(in crate::store::node_credentials::endpoint_authority) fn issue_fresh_at_on(
    transaction: &rusqlite::Transaction<'_>,
    authorized: &AuthorizedFreshNodeEndpointCredentialIssuance,
    recorded_at: DateTime<Utc>,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    if let Some(stored) = rows::fresh_version_by_issuance_on(
        transaction,
        authorized.agent_id(),
        authorized.owner_user_id(),
        authorized.install_id(),
        authorized.issuance_request_id(),
    )? {
        let prepared = authorized.prepare(parse_recorded_at(stored.recorded_at())?)?;
        stored.ensure_exact(&prepared)?;
        let current = require_root_for_version(transaction, &prepared)?;
        return Ok(credential_receipt(
            current,
            Some(stored.into_envelope()),
            None,
            true,
        ));
    }

    let prepared = authorized.prepare(recorded_at)?;
    write::insert_version_on(transaction, &prepared)?;
    let current = root::insert_initial_root_on(transaction, &prepared)?;
    ensure_current_version(&current, &prepared)?;
    let stored = rows::version_by_issuance_on(
        transaction,
        prepared.envelope().credential_id(),
        prepared.envelope().issuance_request_id(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_VERSION_READBACK_MISSING"))?;
    stored.ensure_exact(&prepared)?;
    root::require_expected_active_on(transaction, &current)?;
    Ok(credential_receipt(
        current,
        Some(stored.into_envelope()),
        None,
        false,
    ))
}
