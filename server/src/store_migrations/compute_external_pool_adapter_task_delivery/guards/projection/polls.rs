use anyhow::Result;
use rusqlite::Connection;

use super::install_projection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    install_projection(
        conn,
        "v273_task_reconcile_poll_projection",
        "compute_external_pool_adapter_task_reconcile_polls",
        "reconcile_poll_json",
        &reconcile_fields(),
    )?;
    install_projection(
        conn,
        "v273_task_event_poll_projection",
        "compute_external_pool_adapter_task_event_polls",
        "event_poll_json",
        &event_fields(),
    )
}

fn reconcile_fields() -> Vec<(&'static str, &'static str)> {
    let mut fields = metadata("reconcile_poll");
    fields.extend([
        (
            "predecessor_reconcile_poll_id",
            "$.poll.lineage.predecessor_id",
        ),
        (
            "predecessor_reconcile_poll_digest",
            "$.poll.lineage.predecessor_digest",
        ),
        ("poll_ordinal", "$.poll.lineage.poll_ordinal"),
        (
            "uncertain_exchange_attempt_id",
            "$.poll.uncertain_exchange_attempt_id",
        ),
        (
            "uncertain_exchange_attempt_digest",
            "$.poll.uncertain_exchange_attempt_digest",
        ),
    ]);
    fields.extend(command_fields());
    fields.extend([
        ("remote_execution_id", "$.poll.remote.remote_execution_id"),
        (
            "remote_identity_digest",
            "$.poll.remote.remote_identity_digest",
        ),
        (
            "remote_execution_state",
            "$.poll.remote.remote_execution_state",
        ),
        (
            "executor_binding_digest",
            "$.poll.remote.executor_binding_digest",
        ),
        (
            "authenticated_subject_sha256",
            "$.poll.authenticated_subject_sha256",
        ),
        ("request_digest", "$.poll.request_digest"),
        ("not_before", "$.poll.not_before"),
        ("not_after", "$.poll.not_after"),
        ("created_at", "$.poll.created_at"),
        ("authority_status", "$.poll.boundary.authority_status"),
        ("effects_json", "$.poll.boundary.effects"),
        ("readiness_json", "$.poll.boundary.readiness"),
    ]);
    fields
}

fn event_fields() -> Vec<(&'static str, &'static str)> {
    let mut fields = metadata("event_poll");
    fields.extend([
        ("predecessor_event_poll_id", "$.poll.lineage.predecessor_id"),
        (
            "predecessor_event_poll_digest",
            "$.poll.lineage.predecessor_digest",
        ),
        ("poll_ordinal", "$.poll.lineage.poll_ordinal"),
        (
            "source_exchange_receipt_id",
            "$.poll.source_exchange_receipt_id",
        ),
        (
            "source_exchange_receipt_digest",
            "$.poll.source_exchange_receipt_digest",
        ),
    ]);
    fields.extend(command_fields());
    fields.extend([
        ("remote_execution_id", "$.poll.remote.remote_execution_id"),
        (
            "remote_identity_digest",
            "$.poll.remote.remote_identity_digest",
        ),
        (
            "remote_execution_state",
            "$.poll.remote.remote_execution_state",
        ),
        (
            "executor_binding_digest",
            "$.poll.remote.executor_binding_digest",
        ),
        (
            "authenticated_subject_sha256",
            "$.poll.authenticated_subject_sha256",
        ),
        (
            "requested_remote_sequence",
            "$.poll.requested_cursor.remote_sequence",
        ),
        (
            "requested_previous_event_root",
            "$.poll.requested_cursor.previous_event_root",
        ),
        (
            "requested_cursor_digest",
            "$.poll.requested_cursor.cursor_digest",
        ),
        ("request_digest", "$.poll.request_digest"),
        ("not_before", "$.poll.not_before"),
        ("not_after", "$.poll.not_after"),
        ("created_at", "$.poll.created_at"),
        ("authority_status", "$.poll.boundary.authority_status"),
        ("effects_json", "$.poll.boundary.effects"),
        ("readiness_json", "$.poll.boundary.readiness"),
    ]);
    fields
}

fn metadata(kind: &'static str) -> Vec<(&'static str, &'static str)> {
    if kind == "reconcile_poll" {
        vec![
            ("reconcile_poll_schema", "$.schema"),
            ("reconcile_poll_id", "$.reconcile_poll_id"),
            ("reconcile_poll_digest", "$.reconcile_poll_digest"),
            ("canonicalization", "$.canonicalization"),
            ("digest_algorithm", "$.digest_algorithm"),
        ]
    } else {
        vec![
            ("event_poll_schema", "$.schema"),
            ("event_poll_id", "$.event_poll_id"),
            ("event_poll_digest", "$.event_poll_digest"),
            ("canonicalization", "$.canonicalization"),
            ("digest_algorithm", "$.digest_algorithm"),
        ]
    }
}

fn command_fields() -> [(&'static str, &'static str); 11] {
    [
        ("command_id", "$.poll.command.command_id"),
        ("command_digest", "$.poll.command.command_digest"),
        ("outbox_id", "$.poll.command.outbox_id"),
        ("outbox_digest", "$.poll.command.outbox_digest"),
        ("send_attempt_id", "$.poll.command.send_attempt_id"),
        ("send_attempt_digest", "$.poll.command.send_attempt_digest"),
        (
            "route_authorization_id",
            "$.poll.command.route_authorization_id",
        ),
        (
            "route_authorization_digest",
            "$.poll.command.route_authorization_digest",
        ),
        (
            "executor_binding_digest",
            "$.poll.command.executor_binding_digest",
        ),
        ("fencing_generation", "$.poll.command.fencing_generation"),
        ("fence_digest", "$.poll.command.fence_digest"),
    ]
}
