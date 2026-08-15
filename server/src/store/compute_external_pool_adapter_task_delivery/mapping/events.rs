use anyhow::{ensure, Result};
use rusqlite::types::Value;

use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    canonical_task_production_event_batch_json_and_digest,
    canonical_task_production_event_json_and_digest, ExternalPoolAdapterTaskEventBatchEnvelope,
    ExternalPoolAdapterTaskEventEnvelope,
};

use super::{canonical_value, integer, optional_text, text};

pub(super) fn event_batch_values(
    envelope: &ExternalPoolAdapterTaskEventBatchEnvelope,
) -> Result<Vec<Value>> {
    let canonical = canonical_task_production_event_batch_json_and_digest(envelope)?.0;
    let batch = &envelope.batch;
    let values = vec![
        text(&envelope.event_batch_id),
        text(&envelope.schema),
        text(&envelope.event_batch_digest),
        Value::Text(canonical),
        text(&envelope.canonicalization),
        text(&envelope.digest_algorithm),
        text(&batch.event_poll_id),
        text(&batch.event_poll_digest),
        text(&batch.exchange_receipt_id),
        text(&batch.exchange_receipt_digest),
        optional_text(batch.predecessor_event_batch_id.as_deref()),
        optional_text(batch.predecessor_event_batch_digest.as_deref()),
        optional_text(batch.remote.remote_execution_id.as_deref()),
        text(&batch.remote.remote_identity_digest),
        text(&batch.remote.executor_binding_digest),
        text(&batch.remote.remote_execution_state),
        text(&batch.authenticated_observation_sha256),
        integer(batch.cursor_before.remote_sequence)?,
        optional_text(batch.cursor_before.previous_event_root.as_deref()),
        text(&batch.cursor_before.cursor_digest),
        integer(batch.cursor_after.remote_sequence)?,
        optional_text(batch.cursor_after.previous_event_root.as_deref()),
        text(&batch.cursor_after.cursor_digest),
        optional_text(batch.previous_batch_root.as_deref()),
        text(&batch.batch_root),
        text(&batch.replay_classification),
        integer(batch.event_count)?,
        canonical_value(&batch.event_roots)?,
        text(&batch.event_inventory_digest),
        text(&batch.authenticated_at),
        text(&batch.received_at),
        text(&batch.recorded_at),
        text(&batch.boundary.authority_status),
        canonical_value(&batch.boundary.effects)?,
        canonical_value(&batch.boundary.readiness)?,
    ];
    ensure!(
        values.len() == 35,
        "V273 event batch mapping is not 35 columns"
    );
    Ok(values)
}

pub(super) fn event_values(envelope: &ExternalPoolAdapterTaskEventEnvelope) -> Result<Vec<Value>> {
    let canonical = canonical_task_production_event_json_and_digest(envelope)?.0;
    let event = &envelope.event;
    let values = vec![
        text(&envelope.event_id),
        text(&envelope.schema),
        text(&envelope.event_digest),
        Value::Text(canonical),
        text(&envelope.canonicalization),
        text(&envelope.digest_algorithm),
        text(&event.event_batch_id),
        text(&event.event_batch_digest),
        text(&event.remote_identity_digest),
        integer(event.event_ordinal)?,
        text(&event.remote_event_id),
        text(&event.event_type),
        integer(event.remote_sequence)?,
        optional_text(event.previous_event_root.as_deref()),
        text(&event.event_root),
        text(&event.canonical_event_digest),
        text(&event.observed_at),
        text(&event.recorded_at),
        text(&event.boundary.authority_status),
        canonical_value(&event.boundary.effects)?,
        canonical_value(&event.boundary.readiness)?,
    ];
    ensure!(values.len() == 21, "V273 event mapping is not 21 columns");
    Ok(values)
}
