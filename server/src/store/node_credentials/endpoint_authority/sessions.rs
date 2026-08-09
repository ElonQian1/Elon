use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::node_compute_sharing::endpoint_authority::{
    NodeEndpointCredentialBinding, NodeEndpointSessionBinding, NodeEndpointSessionHeadSnapshot,
    NodeEndpointSessionOpenRequest, PreparedNodeEndpointCredentialRevocation,
    VerifiedSecureNodeEndpointTransport,
};

use super::{credentials, verified_session, Store, VerifiedCurrentNodeEndpointSession};

mod head_rows;
mod receipt_rows;

pub(super) fn authenticate(
    store: &Store,
    request: &NodeEndpointSessionOpenRequest,
    transport: &VerifiedSecureNodeEndpointTransport,
) -> Result<VerifiedCurrentNodeEndpointSession> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

    if let Some(stored) =
        receipt_rows::receipt_by_session_id_on(&transaction, request.session_id())?
    {
        let credential = stored.credential_binding()?;
        credentials::verify_bound_secret_on(&transaction, &credential, request.presented_secret())?;
        let predecessor = predecessor_for_receipt_on(&transaction, stored.envelope())?;
        let prepared = request.prepare(
            &credential,
            predecessor.as_ref(),
            transport,
            parse_timestamp(stored.recorded_at())?,
        )?;
        stored.ensure_exact(&prepared)?;
        let current = current_session_on(&transaction, &stored.session_binding()?, true)?;
        transaction.commit()?;
        return Ok(current);
    }

    let credential = credentials::authenticate_current_for_agent_on(
        &transaction,
        request.agent_id(),
        request.presented_secret(),
    )?;
    let predecessor = head_rows::head_by_agent_on(&transaction, request.agent_id())?;
    if let Some(previous) = predecessor.as_ref() {
        validate_head_on(&transaction, previous)?;
    }
    let prepared = request.prepare(&credential, predecessor.as_ref(), transport, Utc::now())?;

    if let Some(previous) = predecessor.as_ref().filter(|head| head.state() == "active") {
        head_rows::terminate_exact_head_on(
            &transaction,
            previous.binding(),
            "stale",
            prepared.envelope().authenticated_at(),
            "superseded_by_session_authentication",
        )?;
    }
    receipt_rows::insert_receipt_on(&transaction, &prepared)?;
    head_rows::insert_or_replace_head_on(&transaction, &prepared, predecessor.as_ref())?;
    let stored = receipt_rows::receipt_by_session_id_on(&transaction, request.session_id())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_RECEIPT_READBACK_MISSING"))?;
    stored.ensure_exact(&prepared)?;
    let current = current_session_on(&transaction, &stored.session_binding()?, false)?;
    transaction.commit()?;
    Ok(current)
}

pub(super) fn close(
    store: &Store,
    current: &VerifiedCurrentNodeEndpointSession,
) -> Result<NodeEndpointSessionHeadSnapshot> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let binding = current.head().binding();
    current_session_on(&transaction, binding, false)?;
    let closed_at = next_timestamp(current.head().updated_at())?;
    let closed = head_rows::terminate_exact_head_on(
        &transaction,
        binding,
        "closed",
        &closed_at,
        "secure_transport_closed",
    )?;
    transaction.commit()?;
    Ok(closed)
}

pub(super) fn inspect_currentness(
    store: &Store,
    binding: &NodeEndpointSessionBinding,
) -> Result<VerifiedCurrentNodeEndpointSession> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current = current_session_on(&transaction, binding, false)?;
    transaction.commit()?;
    Ok(current)
}

pub(super) fn restart(store: &Store) -> Result<Vec<NodeEndpointSessionHeadSnapshot>> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let heads = head_rows::all_heads_on(&transaction)?;
    let mut recovered = Vec::new();
    for head in heads.iter().filter(|head| head.state() == "active") {
        validate_head_on(&transaction, head)?;
        let closed_at = next_timestamp(head.updated_at())?;
        recovered.push(head_rows::terminate_exact_head_on(
            &transaction,
            head.binding(),
            "stale",
            &closed_at,
            "server_restart",
        )?);
    }
    transaction.commit()?;
    Ok(recovered)
}

pub(super) fn recover_heads(store: &Store) -> Result<Vec<NodeEndpointSessionHeadSnapshot>> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let heads = head_rows::all_heads_on(&transaction)?;
    for head in &heads {
        validate_head_on(&transaction, head)?;
    }
    transaction.commit()?;
    Ok(heads)
}

fn validate_head_on(connection: &Connection, head: &NodeEndpointSessionHeadSnapshot) -> Result<()> {
    let receipt = receipt_rows::receipt_by_binding_on(connection, head.binding())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_RECEIPT_READBACK_MISSING"))?;
    ensure_head_receipt_projection(head, &receipt)
}

pub(super) fn close_active_head_for_credential_mutation_on(
    transaction: &Transaction<'_>,
    expected: &NodeEndpointCredentialBinding,
    revocation: &PreparedNodeEndpointCredentialRevocation,
) -> Result<()> {
    let Some(head) = head_rows::head_by_agent_on(transaction, expected.agent_id())? else {
        return Ok(());
    };
    if head.state() != "active" {
        return Ok(());
    }
    let binding = head.binding();
    if binding.credential_id() != expected.credential_id()
        || binding.credential_revision() != expected.credential_revision()
        || binding.credential_digest() != expected.credential_digest()
    {
        bail!("NODE_ENDPOINT_SESSION_CREDENTIAL_CURRENTNESS_MISMATCH");
    }
    let envelope = revocation.envelope();
    let state = if envelope.revocation_kind() == "rotated" {
        "credential_rotated"
    } else {
        "credential_revoked"
    };
    head_rows::terminate_exact_head_on(
        transaction,
        binding,
        state,
        envelope.revoked_at(),
        envelope.reason_code(),
    )?;
    Ok(())
}

pub(super) fn ensure_no_active_head_for_credential_on(
    connection: &Connection,
    expected: &NodeEndpointCredentialBinding,
) -> Result<()> {
    if head_rows::head_by_agent_on(connection, expected.agent_id())?
        .is_some_and(|head| head.state() == "active")
    {
        bail!("NODE_ENDPOINT_SESSION_ACTIVE_DURING_CREDENTIAL_RECOVERY");
    }
    Ok(())
}

fn current_session_on(
    connection: &Connection,
    binding: &NodeEndpointSessionBinding,
    replayed: bool,
) -> Result<VerifiedCurrentNodeEndpointSession> {
    let head = head_rows::head_by_agent_on(connection, binding.agent_id())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_CURRENTNESS_MISMATCH"))?;
    if head.binding() != binding || head.state() != "active" {
        bail!("NODE_ENDPOINT_SESSION_CURRENTNESS_MISMATCH");
    }
    if parse_timestamp(head.expires_at())? <= Utc::now() {
        bail!("NODE_ENDPOINT_SESSION_EXPIRED");
    }
    let receipt = receipt_rows::receipt_by_binding_on(connection, binding)?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_RECEIPT_READBACK_MISSING"))?;
    ensure_head_receipt_projection(&head, &receipt)?;
    credentials::require_current_binding_on(connection, &receipt.credential_binding()?)?;
    Ok(verified_session(receipt.into_envelope(), head, replayed))
}

fn ensure_head_receipt_projection(
    head: &NodeEndpointSessionHeadSnapshot,
    receipt: &receipt_rows::StoredSessionReceipt,
) -> Result<()> {
    let envelope = receipt.envelope();
    if receipt.session_binding()? != *head.binding()
        || head.authenticated_at() != envelope.authenticated_at()
        || head.expires_at() != envelope.expires_at()
        || head.created_at() != envelope.recorded_at()
        || (head.state() == "active" && head.updated_at() != envelope.recorded_at())
    {
        bail!("NODE_ENDPOINT_SESSION_HEAD_RECEIPT_PROJECTION_MISMATCH");
    }
    Ok(())
}

fn predecessor_for_receipt_on(
    connection: &Connection,
    envelope: &crate::node_compute_sharing::endpoint_authority::NodeEndpointSessionAuthenticationReceiptEnvelope,
) -> Result<Option<NodeEndpointSessionHeadSnapshot>> {
    match (
        envelope.previous_authentication_receipt_id(),
        envelope.previous_authentication_digest(),
    ) {
        (None, None) => Ok(None),
        (Some(receipt_id), Some(digest)) => {
            let receipt = receipt_rows::receipt_by_id_digest_on(connection, receipt_id, digest)?
                .ok_or_else(|| {
                    anyhow::anyhow!("NODE_ENDPOINT_SESSION_PREDECESSOR_READBACK_MISSING")
                })?;
            Ok(Some(receipt.predecessor_snapshot()?))
        }
        _ => bail!("NODE_ENDPOINT_SESSION_PREDECESSOR_READBACK_MISMATCH"),
    }
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

fn next_timestamp(previous: &str) -> Result<String> {
    let floor = parse_timestamp(previous)?
        .checked_add_signed(Duration::nanoseconds(1))
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_TIMESTAMP_EXHAUSTED"))?;
    Ok(std::cmp::max(Utc::now(), floor).to_rfc3339_opts(SecondsFormat::Nanos, true))
}
