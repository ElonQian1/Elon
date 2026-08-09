use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::TransactionBehavior;

use crate::node_compute_sharing::endpoint_authority::{
    AuthorizedFreshNodeEndpointCredentialIssuance, AuthorizedNodeEndpointCredentialRecovery,
    AuthorizedNodeEndpointCredentialRevocation, AuthorizedNodeEndpointCredentialRotation,
    NodeEndpointCredentialBinding, PreparedNodeEndpointCredentialRevocation,
    PreparedNodeEndpointCredentialVersion, PresentedNodeEndpointCredentialSecret,
};

use super::{credential_receipt, sessions, NodeEndpointCredentialMutationReceipt, Store};

mod root;
mod rows;
mod write;

pub(super) fn issue_fresh(
    store: &Store,
    authorized: &AuthorizedFreshNodeEndpointCredentialIssuance,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

    if let Some(stored) = rows::fresh_version_by_issuance_on(
        &transaction,
        authorized.agent_id(),
        authorized.owner_user_id(),
        authorized.install_id(),
        authorized.issuance_request_id(),
    )? {
        let prepared = authorized.prepare(parse_recorded_at(stored.recorded_at())?)?;
        stored.ensure_exact(&prepared)?;
        let current = require_root_for_version(&transaction, &prepared)?;
        let receipt = credential_receipt(current, Some(stored.into_envelope()), None, true);
        transaction.commit()?;
        return Ok(receipt);
    }

    let prepared = authorized.prepare(Utc::now())?;
    write::insert_version_on(&transaction, &prepared)?;
    let current = root::insert_initial_root_on(&transaction, &prepared)?;
    ensure_current_version(&current, &prepared)?;
    let stored = rows::version_by_issuance_on(
        &transaction,
        prepared.envelope().credential_id(),
        prepared.envelope().issuance_request_id(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_VERSION_READBACK_MISSING"))?;
    stored.ensure_exact(&prepared)?;
    root::require_expected_active_on(&transaction, &current)?;
    let receipt = credential_receipt(current, Some(stored.into_envelope()), None, false);
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
    rows::verify_secret_on(&transaction, authorized.expected(), presented)?;

    if let Some(stored) = rows::version_by_issuance_on(
        &transaction,
        authorized.expected().credential_id(),
        authorized.issuance_request_id(),
    )? {
        let recorded_at = parse_recorded_at(stored.recorded_at())?;
        let prepared = authorized.prepare(recorded_at)?;
        let revocation = authorized.prepare_revocation(recorded_at)?;
        stored.ensure_exact(&prepared)?;
        let stored_revocation = require_exact_revocation(&transaction, &revocation)?;
        let current = require_root_for_version(&transaction, &prepared)?;
        let receipt = credential_receipt(
            current,
            Some(stored.into_envelope()),
            Some(stored_revocation.into_envelope()),
            true,
        );
        transaction.commit()?;
        return Ok(receipt);
    }

    let recorded_at = Utc::now();
    let prepared = authorized.prepare(recorded_at)?;
    let revocation = authorized.prepare_revocation(recorded_at)?;
    let receipt = replace_current(&transaction, authorized.expected(), &prepared, &revocation)?;
    transaction.commit()?;
    Ok(receipt)
}

pub(super) fn recover(
    store: &Store,
    authorized: &AuthorizedNodeEndpointCredentialRecovery,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

    if let Some(stored) = rows::version_by_issuance_on(
        &transaction,
        authorized.expected().credential_id(),
        authorized.issuance_request_id(),
    )? {
        let recorded_at = parse_recorded_at(stored.recorded_at())?;
        let prepared = authorized.prepare(recorded_at)?;
        let revocation = authorized.prepare_revocation(recorded_at)?;
        stored.ensure_exact(&prepared)?;
        let stored_revocation = match revocation.as_ref() {
            Some(revocation) => require_exact_revocation(&transaction, revocation)?,
            None => require_terminal_recovery_revocation(&transaction, authorized.expected())?,
        };
        let current = require_root_for_version(&transaction, &prepared)?;
        let receipt = credential_receipt(
            current,
            Some(stored.into_envelope()),
            Some(stored_revocation.into_envelope()),
            true,
        );
        transaction.commit()?;
        return Ok(receipt);
    }

    let recorded_at = Utc::now();
    let prepared = authorized.prepare(recorded_at)?;
    let revocation = authorized.prepare_revocation(recorded_at)?;
    let receipt = match revocation.as_ref() {
        Some(revocation) => {
            replace_current(&transaction, authorized.expected(), &prepared, revocation)?
        }
        None => recover_revoked_current(&transaction, authorized.expected(), &prepared)?,
    };
    transaction.commit()?;
    Ok(receipt)
}

pub(super) fn revoke(
    store: &Store,
    authorized: &AuthorizedNodeEndpointCredentialRevocation,
) -> Result<NodeEndpointCredentialMutationReceipt> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

    if let Some(stored) = rows::revocation_for_version_on(
        &transaction,
        authorized.expected().credential_id(),
        authorized.expected().credential_revision(),
    )? {
        let prepared = authorized.prepare(parse_recorded_at(stored.recorded_at())?)?;
        stored.ensure_exact(&prepared)?;
        let current =
            rows::credential_root_on(&transaction, authorized.expected().credential_id())?
                .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_ROOT_READBACK_MISSING"))?;
        root::require_expected_on(&transaction, &current)?;
        let receipt = credential_receipt(current, None, Some(stored.into_envelope()), true);
        transaction.commit()?;
        return Ok(receipt);
    }

    let prepared = authorized.prepare(Utc::now())?;
    root::require_expected_active_on(&transaction, authorized.expected())?;
    sessions::close_active_head_for_credential_mutation_on(
        &transaction,
        authorized.expected(),
        &prepared,
    )?;
    write::insert_revocation_on(&transaction, &prepared)?;
    let current = root::revoke_root_on(
        &transaction,
        authorized.expected(),
        prepared.envelope().recorded_at(),
    )?;
    if current.status() != "revoked"
        || current.credential_revision() != authorized.expected().credential_revision()
        || current.credential_digest() != authorized.expected().credential_digest()
    {
        bail!("NODE_ENDPOINT_CREDENTIAL_REVOKE_READBACK_MISMATCH");
    }
    let stored = require_exact_revocation(&transaction, &prepared)?;
    root::require_expected_on(&transaction, &current)?;
    let receipt = credential_receipt(current, None, Some(stored.into_envelope()), false);
    transaction.commit()?;
    Ok(receipt)
}

fn replace_current(
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

fn require_exact_revocation(
    connection: &rusqlite::Connection,
    prepared: &PreparedNodeEndpointCredentialRevocation,
) -> Result<rows::StoredCredentialRevocation> {
    let envelope = prepared.envelope();
    let stored = rows::revocation_for_version_on(
        connection,
        envelope.credential_id(),
        envelope.credential_revision(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_REVOCATION_READBACK_MISSING"))?;
    stored.ensure_exact(prepared)?;
    Ok(stored)
}

fn require_terminal_recovery_revocation(
    connection: &rusqlite::Connection,
    expected: &NodeEndpointCredentialBinding,
) -> Result<rows::StoredCredentialRevocation> {
    let stored = rows::revocation_for_version_on(
        connection,
        expected.credential_id(),
        expected.credential_revision(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_RECOVERY_REVOCATION_MISSING"))?;
    let envelope = stored.envelope();
    if envelope.credential_digest() != expected.credential_digest()
        || envelope.agent_id() != expected.agent_id()
        || !matches!(
            envelope.revocation_kind(),
            "owner_revoked" | "security_revoked"
        )
    {
        bail!("NODE_ENDPOINT_CREDENTIAL_RECOVERY_REVOCATION_MISMATCH");
    }
    Ok(stored)
}

fn require_root_for_version(
    connection: &rusqlite::Connection,
    prepared: &PreparedNodeEndpointCredentialVersion,
) -> Result<NodeEndpointCredentialBinding> {
    let current = rows::credential_root_on(connection, prepared.envelope().credential_id())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_ROOT_READBACK_MISSING"))?;
    root::require_expected_on(connection, &current)?;
    Ok(current)
}

fn ensure_current_version(
    current: &NodeEndpointCredentialBinding,
    prepared: &PreparedNodeEndpointCredentialVersion,
) -> Result<()> {
    let envelope = prepared.envelope();
    if current.credential_id() != envelope.credential_id()
        || current.agent_id() != envelope.agent_id()
        || current.owner_user_id() != envelope.owner_user_id()
        || current.install_id() != envelope.install_id()
        || current.installation_binding_digest() != envelope.installation_binding_digest()
        || current.credential_revision() != envelope.credential_revision()
        || current.credential_digest() != prepared.credential_digest()
        || current.status() != "active"
    {
        bail!("NODE_ENDPOINT_CREDENTIAL_CURRENT_READBACK_MISMATCH");
    }
    Ok(())
}

fn parse_recorded_at(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
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
