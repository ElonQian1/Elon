use anyhow::{ensure, Result};
use rusqlite::types::Value;

use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    canonical_task_production_event_poll_json_and_digest,
    canonical_task_production_reconcile_poll_json_and_digest,
    ExternalPoolAdapterTaskEventPollEnvelope, ExternalPoolAdapterTaskPollCommandBinding,
    ExternalPoolAdapterTaskReconcilePollEnvelope,
};

use super::{canonical_value, integer, optional_text, text};
use crate::store::compute_external_pool_adapter_task_delivery::types::PollClaimProjection;

pub(in crate::store::compute_external_pool_adapter_task_delivery) fn reconcile_poll_values(
    envelope: &ExternalPoolAdapterTaskReconcilePollEnvelope,
    claim: &PollClaimProjection,
) -> Result<Vec<Value>> {
    let canonical = canonical_task_production_reconcile_poll_json_and_digest(envelope)?.0;
    let poll = &envelope.poll;
    let mut values = Vec::with_capacity(39);
    values.extend([
        text(&envelope.reconcile_poll_id),
        text(&envelope.schema),
        text(&envelope.reconcile_poll_digest),
        Value::Text(canonical),
        text(&envelope.canonicalization),
        text(&envelope.digest_algorithm),
        optional_text(poll.lineage.predecessor_id.as_deref()),
        optional_text(poll.lineage.predecessor_digest.as_deref()),
        integer(poll.lineage.poll_ordinal)?,
        text(&poll.uncertain_exchange_attempt_id),
        text(&poll.uncertain_exchange_attempt_digest),
    ]);
    values.extend(command_values(&poll.command)?);
    values.extend([
        optional_text(poll.remote.remote_execution_id.as_deref()),
        text(&poll.remote.remote_identity_digest),
        text(&poll.remote.remote_execution_state),
        optional_text(poll.authenticated_subject_sha256.as_deref()),
        text(&poll.request_digest),
        text(&poll.not_before),
        text(&poll.not_after),
        text(&poll.created_at),
        text(&poll.boundary.authority_status),
        canonical_value(&poll.boundary.effects)?,
        canonical_value(&poll.boundary.readiness)?,
    ]);
    values.extend(claim_values(claim)?);
    ensure!(
        values.len() == 39,
        "V273 reconcile poll mapping is not 39 columns"
    );
    Ok(values)
}

pub(in crate::store::compute_external_pool_adapter_task_delivery) fn event_poll_values(
    envelope: &ExternalPoolAdapterTaskEventPollEnvelope,
    claim: &PollClaimProjection,
) -> Result<Vec<Value>> {
    let canonical = canonical_task_production_event_poll_json_and_digest(envelope)?.0;
    let poll = &envelope.poll;
    let mut values = Vec::with_capacity(42);
    values.extend([
        text(&envelope.event_poll_id),
        text(&envelope.schema),
        text(&envelope.event_poll_digest),
        Value::Text(canonical),
        text(&envelope.canonicalization),
        text(&envelope.digest_algorithm),
        optional_text(poll.lineage.predecessor_id.as_deref()),
        optional_text(poll.lineage.predecessor_digest.as_deref()),
        integer(poll.lineage.poll_ordinal)?,
        text(&poll.source_exchange_receipt_id),
        text(&poll.source_exchange_receipt_digest),
    ]);
    values.extend(command_values(&poll.command)?);
    values.extend([
        optional_text(poll.remote.remote_execution_id.as_deref()),
        text(&poll.remote.remote_identity_digest),
        text(&poll.remote.remote_execution_state),
        text(&poll.authenticated_subject_sha256),
        integer(poll.requested_cursor.remote_sequence)?,
        optional_text(poll.requested_cursor.previous_event_root.as_deref()),
        text(&poll.requested_cursor.cursor_digest),
        text(&poll.request_digest),
        text(&poll.not_before),
        text(&poll.not_after),
        text(&poll.created_at),
        text(&poll.boundary.authority_status),
        canonical_value(&poll.boundary.effects)?,
        canonical_value(&poll.boundary.readiness)?,
    ]);
    values.extend(claim_values(claim)?);
    ensure!(
        values.len() == 42,
        "V273 event poll mapping is not 42 columns"
    );
    Ok(values)
}

fn command_values(command: &ExternalPoolAdapterTaskPollCommandBinding) -> Result<Vec<Value>> {
    Ok(vec![
        text(&command.command_id),
        text(&command.command_digest),
        text(&command.outbox_id),
        text(&command.outbox_digest),
        text(&command.send_attempt_id),
        text(&command.send_attempt_digest),
        text(&command.route_authorization_id),
        text(&command.route_authorization_digest),
        text(&command.executor_binding_digest),
        integer(command.fencing_generation)?,
        text(&command.fence_digest),
    ])
}

fn claim_values(claim: &PollClaimProjection) -> Result<[Value; 6]> {
    Ok([
        text(&claim.status),
        integer(claim.revision)?,
        integer(claim.generation)?,
        optional_text(claim.owner_id.as_deref()),
        optional_text(claim.token_digest.as_deref()),
        optional_text(claim.expires_at.as_deref()),
    ])
}
