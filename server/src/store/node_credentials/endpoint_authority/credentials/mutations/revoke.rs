use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use crate::node_compute_sharing::endpoint_authority::AuthorizedNodeEndpointCredentialRevocation;

use super::super::super::{credential_receipt, sessions, NodeEndpointCredentialMutationReceipt};
use super::super::{root, rows, write};
use super::support::{parse_recorded_at, require_exact_revocation};

pub(in crate::store::node_credentials::endpoint_authority) fn revoke_on(
    transaction: &rusqlite::Transaction<'_>,
    authorized: &AuthorizedNodeEndpointCredentialRevocation,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    revoke_at_on(transaction, authorized, Utc::now())
}

pub(in crate::store::node_credentials::endpoint_authority) fn revoke_at_on(
    transaction: &rusqlite::Transaction<'_>,
    authorized: &AuthorizedNodeEndpointCredentialRevocation,
    recorded_at: DateTime<Utc>,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    if let Some(stored) = rows::revocation_for_version_on(
        transaction,
        authorized.expected().credential_id(),
        authorized.expected().credential_revision(),
    )? {
        let prepared = authorized.prepare(parse_recorded_at(stored.recorded_at())?)?;
        stored.ensure_exact(&prepared)?;
        let current = rows::credential_root_on(transaction, authorized.expected().credential_id())?
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_ROOT_READBACK_MISSING"))?;
        root::require_expected_on(transaction, &current)?;
        return Ok(credential_receipt(
            current,
            None,
            Some(stored.into_envelope()),
            true,
        ));
    }

    let prepared = authorized.prepare(recorded_at)?;
    root::require_expected_active_on(transaction, authorized.expected())?;
    sessions::close_active_head_for_credential_mutation_on(
        transaction,
        authorized.expected(),
        &prepared,
    )?;
    write::insert_revocation_on(transaction, &prepared)?;
    let current = root::revoke_root_on(
        transaction,
        authorized.expected(),
        prepared.envelope().recorded_at(),
    )?;
    if current.status() != "revoked"
        || current.credential_revision() != authorized.expected().credential_revision()
        || current.credential_digest() != authorized.expected().credential_digest()
    {
        bail!("NODE_ENDPOINT_CREDENTIAL_REVOKE_READBACK_MISMATCH");
    }
    let stored = require_exact_revocation(transaction, &prepared)?;
    root::require_expected_on(transaction, &current)?;
    Ok(credential_receipt(
        current,
        None,
        Some(stored.into_envelope()),
        false,
    ))
}
