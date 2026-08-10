use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use crate::node_compute_sharing::endpoint_authority::{
    AuthorizedNodeEndpointCredentialRecovery, NodeEndpointCredentialBinding,
    PreparedNodeEndpointCredentialVersion,
};

use super::super::super::{credential_receipt, sessions, NodeEndpointCredentialMutationReceipt};
use super::super::{root, rows, write};
use super::replace::replace_current;
use super::support::{
    ensure_current_version, parse_recorded_at, require_exact_revocation, require_root_for_version,
    require_terminal_recovery_revocation,
};

pub(in crate::store::node_credentials::endpoint_authority) fn recover_on(
    transaction: &rusqlite::Transaction<'_>,
    authorized: &AuthorizedNodeEndpointCredentialRecovery,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    recover_at_on(transaction, authorized, Utc::now())
}

pub(in crate::store::node_credentials::endpoint_authority) fn recover_at_on(
    transaction: &rusqlite::Transaction<'_>,
    authorized: &AuthorizedNodeEndpointCredentialRecovery,
    recorded_at: DateTime<Utc>,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    if let Some(stored) = rows::version_by_issuance_on(
        transaction,
        authorized.expected().credential_id(),
        authorized.issuance_request_id(),
    )? {
        let recorded_at = parse_recorded_at(stored.recorded_at())?;
        let prepared = authorized.prepare(recorded_at)?;
        let revocation = authorized.prepare_revocation(recorded_at)?;
        stored.ensure_exact(&prepared)?;
        let stored_revocation = match revocation.as_ref() {
            Some(revocation) => require_exact_revocation(transaction, revocation)?,
            None => require_terminal_recovery_revocation(transaction, authorized.expected())?,
        };
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
    match revocation.as_ref() {
        Some(revocation) => {
            replace_current(transaction, authorized.expected(), &prepared, revocation)
        }
        None => recover_revoked_current(transaction, authorized.expected(), &prepared),
    }
}

fn recover_revoked_current(
    transaction: &rusqlite::Transaction<'_>,
    expected: &NodeEndpointCredentialBinding,
    prepared: &PreparedNodeEndpointCredentialVersion,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    if expected.status() != "revoked" {
        bail!("NODE_ENDPOINT_CREDENTIAL_RECOVERY_SOURCE_MISMATCH");
    }
    root::require_expected_on(transaction, expected)?;
    sessions::ensure_no_active_head_for_credential_on(transaction, expected)?;
    let terminal = require_terminal_recovery_revocation(transaction, expected)?;
    write::insert_version_on(transaction, prepared)?;
    let current = root::advance_root_on(transaction, expected, prepared)?;
    ensure_current_version(&current, prepared)?;
    let stored_version = rows::version_by_issuance_on(
        transaction,
        prepared.envelope().credential_id(),
        prepared.envelope().issuance_request_id(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_VERSION_READBACK_MISSING"))?;
    stored_version.ensure_exact(prepared)?;
    root::require_expected_active_on(transaction, &current)?;
    Ok(credential_receipt(
        current,
        Some(stored_version.into_envelope()),
        Some(terminal.into_envelope()),
        false,
    ))
}
