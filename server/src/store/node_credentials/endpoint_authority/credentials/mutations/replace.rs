use anyhow::Result;

use crate::node_compute_sharing::endpoint_authority::{
    NodeEndpointCredentialBinding, PreparedNodeEndpointCredentialRevocation,
    PreparedNodeEndpointCredentialVersion,
};

use super::super::super::{credential_receipt, sessions, NodeEndpointCredentialMutationReceipt};
use super::super::{root, rows, write};
use super::support::{ensure_current_version, require_exact_revocation};

pub(super) fn replace_current(
    transaction: &rusqlite::Transaction<'_>,
    expected: &NodeEndpointCredentialBinding,
    prepared: &PreparedNodeEndpointCredentialVersion,
    revocation: &PreparedNodeEndpointCredentialRevocation,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    root::require_expected_active_on(transaction, expected)?;
    sessions::close_active_head_for_credential_mutation_on(transaction, expected, revocation)?;
    write::insert_version_on(transaction, prepared)?;
    write::insert_revocation_on(transaction, revocation)?;
    let current = root::advance_root_on(transaction, expected, prepared)?;
    ensure_current_version(&current, prepared)?;

    let stored_version = rows::version_by_issuance_on(
        transaction,
        prepared.envelope().credential_id(),
        prepared.envelope().issuance_request_id(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_VERSION_READBACK_MISSING"))?;
    stored_version.ensure_exact(prepared)?;
    let stored_revocation = require_exact_revocation(transaction, revocation)?;
    root::require_expected_active_on(transaction, &current)?;
    Ok(credential_receipt(
        current,
        Some(stored_version.into_envelope()),
        Some(stored_revocation.into_envelope()),
        false,
    ))
}
