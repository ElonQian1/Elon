use anyhow::Result;
use rusqlite::Connection;

use super::install_projection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    install_projection(
        conn,
        "v273_task_event_batch_projection",
        "compute_external_pool_adapter_task_event_batches",
        "event_batch_json",
        &batch_fields(),
    )?;
    install_projection(
        conn,
        "v273_task_event_projection",
        "compute_external_pool_adapter_task_events",
        "event_json",
        &event_fields(),
    )
}

fn batch_fields() -> Vec<(&'static str, &'static str)> {
    vec![
        ("event_batch_schema", "$.schema"),
        ("event_batch_id", "$.event_batch_id"),
        ("event_batch_digest", "$.event_batch_digest"),
        ("canonicalization", "$.canonicalization"),
        ("digest_algorithm", "$.digest_algorithm"),
        ("event_poll_id", "$.batch.event_poll_id"),
        ("event_poll_digest", "$.batch.event_poll_digest"),
        ("exchange_receipt_id", "$.batch.exchange_receipt_id"),
        ("exchange_receipt_digest", "$.batch.exchange_receipt_digest"),
        (
            "predecessor_event_batch_id",
            "$.batch.predecessor_event_batch_id",
        ),
        (
            "predecessor_event_batch_digest",
            "$.batch.predecessor_event_batch_digest",
        ),
        ("remote_execution_id", "$.batch.remote.remote_execution_id"),
        (
            "remote_identity_digest",
            "$.batch.remote.remote_identity_digest",
        ),
        (
            "executor_binding_digest",
            "$.batch.remote.executor_binding_digest",
        ),
        (
            "remote_execution_state",
            "$.batch.remote.remote_execution_state",
        ),
        (
            "authenticated_observation_sha256",
            "$.batch.authenticated_observation_sha256",
        ),
        (
            "cursor_before_remote_sequence",
            "$.batch.cursor_before.remote_sequence",
        ),
        (
            "cursor_before_previous_event_root",
            "$.batch.cursor_before.previous_event_root",
        ),
        (
            "cursor_before_digest",
            "$.batch.cursor_before.cursor_digest",
        ),
        (
            "cursor_after_remote_sequence",
            "$.batch.cursor_after.remote_sequence",
        ),
        (
            "cursor_after_previous_event_root",
            "$.batch.cursor_after.previous_event_root",
        ),
        ("cursor_after_digest", "$.batch.cursor_after.cursor_digest"),
        ("previous_batch_root", "$.batch.previous_batch_root"),
        ("batch_root", "$.batch.batch_root"),
        ("replay_classification", "$.batch.replay_classification"),
        ("event_count", "$.batch.event_count"),
        ("event_roots_json", "$.batch.event_roots"),
        ("event_inventory_digest", "$.batch.event_inventory_digest"),
        ("authenticated_at", "$.batch.authenticated_at"),
        ("received_at", "$.batch.received_at"),
        ("recorded_at", "$.batch.recorded_at"),
        ("authority_status", "$.batch.boundary.authority_status"),
        ("effects_json", "$.batch.boundary.effects"),
        ("readiness_json", "$.batch.boundary.readiness"),
    ]
}

fn event_fields() -> Vec<(&'static str, &'static str)> {
    vec![
        ("event_schema", "$.schema"),
        ("event_id", "$.event_id"),
        ("event_digest", "$.event_digest"),
        ("canonicalization", "$.canonicalization"),
        ("digest_algorithm", "$.digest_algorithm"),
        ("event_batch_id", "$.event.event_batch_id"),
        ("event_batch_digest", "$.event.event_batch_digest"),
        ("remote_identity_digest", "$.event.remote_identity_digest"),
        ("event_ordinal", "$.event.event_ordinal"),
        ("remote_event_id", "$.event.remote_event_id"),
        ("event_type", "$.event.event_type"),
        ("remote_sequence", "$.event.remote_sequence"),
        ("previous_event_root", "$.event.previous_event_root"),
        ("event_root", "$.event.event_root"),
        ("canonical_event_digest", "$.event.canonical_event_digest"),
        ("observed_at", "$.event.observed_at"),
        ("recorded_at", "$.event.recorded_at"),
        ("authority_status", "$.event.boundary.authority_status"),
        ("effects_json", "$.event.boundary.effects"),
        ("readiness_json", "$.event.boundary.readiness"),
    ]
}
