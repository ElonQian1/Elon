//! Preserve both V214 no-start guards while admitting one receipt-bound direct reconcile chain.

use anyhow::{ensure, Result};
use rusqlite::Connection;

const EXACT_TRIGGER: &str = "trg_compute_attempt_no_start_proof_exact";
const SOURCE_TRIGGER: &str = "trg_compute_attempt_remote_no_start_source_v214";
const MARKER: &str = "external_pool_adapter_task_receipt.v1";

pub(super) fn install(connection: &Connection) -> Result<()> {
    let branch = receipt_no_start_branch();
    append_source(
        connection,
        EXACT_TRIGGER,
        "prepare.operation_kind='prepare'",
        &branch,
    )?;
    append_source(
        connection,
        SOURCE_TRIGGER,
        "prepare_send.operation_kind='prepare'",
        &branch,
    )
}

fn append_source(
    connection: &Connection,
    trigger: &str,
    legacy_marker: &str,
    branch: &str,
) -> Result<()> {
    let sql: String = connection.query_row(
        "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
        [trigger],
        |row| row.get(0),
    )?;
    if sql.contains(MARKER) {
        ensure!(
            sql.matches(MARKER).count() == 1 && sql.contains(legacy_marker),
            "V278 receipt-backed no-start guard is not exact"
        );
        return Ok(());
    }
    ensure!(
        sql.contains("NOT EXISTS (") && sql.contains(legacy_marker),
        "V278 no-start predecessor guard drifted"
    );
    let sql = sql.replacen("NOT EXISTS (", "NOT (EXISTS (", 1);
    let begin = sql
        .rfind("BEGIN")
        .ok_or_else(|| anyhow::anyhow!("V278 no-start guard lost BEGIN"))?;
    let replacement = format!(
        "{} OR EXISTS (\n{branch}\n        ))\n{}",
        &sql[..begin],
        &sql[begin..]
    );
    connection.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS {trigger};\n{replacement};"
    ))?;
    let installed: String = connection.query_row(
        "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
        [trigger],
        |row| row.get(0),
    )?;
    ensure!(
        installed.matches(MARKER).count() == 1 && installed.contains(legacy_marker),
        "V278 receipt-backed no-start branch was not installed"
    );
    Ok(())
}

fn receipt_no_start_branch() -> &'static str {
    r#"            SELECT 1
              FROM compute_attempt_dispatch_commands command
              JOIN compute_attempt_start_outbox prepare
                ON prepare.command_id=command.command_id
               AND prepare.operation_kind='prepare'
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=prepare.route_authorization_id
               AND route.route_authorization_digest=prepare.route_authorization_digest
              JOIN compute_attempt_start_outbox cancel
                ON cancel.subject_outbox_id=prepare.outbox_id
               AND cancel.operation_kind='cancel'
              JOIN compute_attempt_start_send_attempts cancel_send
                ON cancel_send.outbox_id=cancel.outbox_id
               AND cancel_send.outbox_digest=cancel.outbox_digest
               AND cancel_send.operation_kind='cancel'
               AND cancel_send.command_id=cancel.command_id
               AND cancel_send.command_digest=cancel.command_digest
               AND cancel_send.route_authorization_id=cancel.route_authorization_id
               AND cancel_send.route_authorization_digest=cancel.route_authorization_digest
              JOIN compute_external_pool_adapter_task_exchange_receipts cancel_receipt
                ON cancel_receipt.operation_kind='cancel_no_start'
               AND cancel_receipt.source_kind='start_outbox_send_attempt'
               AND cancel_receipt.source_id=cancel_send.send_attempt_id
               AND cancel_receipt.source_digest=cancel_send.send_attempt_digest
              JOIN compute_external_pool_adapter_task_reconcile_polls poll
                ON poll.uncertain_exchange_attempt_id=cancel_receipt.exchange_attempt_id
               AND poll.uncertain_exchange_attempt_digest=cancel_receipt.exchange_attempt_digest
              JOIN compute_external_pool_adapter_task_exchange_receipts reconcile_receipt
                ON reconcile_receipt.operation_kind='reconcile'
               AND reconcile_receipt.source_kind='reconcile_poll'
               AND reconcile_receipt.source_id=poll.reconcile_poll_id
               AND reconcile_receipt.source_digest=poll.reconcile_poll_digest
              JOIN compute_attempt_start_remote_observations observation
                ON observation.verification_kind='external_pool_adapter_task_receipt.v1'
               AND observation.verifier_id=reconcile_receipt.exchange_receipt_id
               AND observation.verification_digest=
                    reconcile_receipt.semantic_observation_sha256
             WHERE command.command_id=NEW.command_id
               AND command.command_digest=NEW.command_digest
               AND command.execution_plan_id=NEW.plan_id
               AND command.execution_plan_digest=NEW.plan_digest
               AND command.reservation_id=NEW.reservation_id
               AND command.reservation_revision=NEW.reservation_revision
               AND command.reservation_digest=NEW.reservation_digest
               AND command.job_id=NEW.job_id AND command.job_revision=NEW.job_revision
               AND command.job_digest=NEW.job_digest
               AND command.capacity_claim_id=NEW.capacity_claim_id
               AND command.claim_revision=NEW.capacity_claim_revision
               AND command.claim_digest=NEW.capacity_claim_digest
               AND command.budget_reservation_id=NEW.budget_reservation_id
               AND command.budget_reserved_fen=NEW.budget_reserved_fen
               AND command.broker_request_digest=NEW.broker_request_digest
               AND command.lease_id=NEW.lease_id
               AND command.fencing_generation=NEW.fencing_generation
               AND command.provider_id=NEW.provider_id AND command.adapter_id=NEW.adapter_id
               AND command.adapter_binding_digest=NEW.adapter_binding_digest
               AND prepare.outbox_id=NEW.outbox_id AND prepare.outbox_digest=NEW.outbox_digest
               AND prepare.plan_id=NEW.plan_id AND prepare.plan_digest=NEW.plan_digest
               AND prepare.provider_id=NEW.provider_id AND prepare.adapter_id=NEW.adapter_id
               AND prepare.adapter_binding_digest=NEW.adapter_binding_digest
               AND route.adapter_revision=NEW.adapter_revision
               AND route.adapter_registry_digest=NEW.adapter_registry_digest
               AND route.route_authorization_id=NEW.route_authorization_id
               AND route.route_authorization_digest=NEW.route_authorization_digest
               AND cancel.operation_generation=1
               AND cancel.command_id=prepare.command_id
               AND cancel.command_digest=prepare.command_digest
               AND cancel.provider_id=prepare.provider_id
               AND cancel.adapter_id=prepare.adapter_id
               AND cancel.adapter_binding_digest=prepare.adapter_binding_digest
               AND cancel.route_authorization_id=prepare.route_authorization_id
               AND cancel.route_authorization_digest=prepare.route_authorization_digest
               AND cancel.plan_id=prepare.plan_id AND cancel.plan_digest=prepare.plan_digest
               AND cancel.lease_id=prepare.lease_id
               AND cancel.fencing_generation=prepare.fencing_generation
               AND cancel.state='delivery_observed'
               AND cancel_send.attempt_no=cancel.attempt_count
               AND cancel_send.claim_generation=cancel.claim_generation
               AND cancel_receipt.command_id=cancel.command_id
               AND cancel_receipt.command_digest=cancel.command_digest
               AND cancel_receipt.outbox_id=cancel.outbox_id
               AND cancel_receipt.outbox_digest=cancel.outbox_digest
               AND cancel_receipt.send_attempt_id=cancel_send.send_attempt_id
               AND poll.command_id=cancel.command_id AND poll.command_digest=cancel.command_digest
               AND poll.outbox_id=cancel.outbox_id AND poll.outbox_digest=cancel.outbox_digest
               AND poll.send_attempt_id=cancel_send.send_attempt_id
               AND poll.send_attempt_digest=cancel_send.send_attempt_digest
               AND poll.authenticated_subject_sha256=cancel_receipt.semantic_observation_sha256
               AND poll.claim_status='delivery_observed'
               AND reconcile_receipt.command_id=cancel.command_id
               AND reconcile_receipt.command_digest=cancel.command_digest
               AND reconcile_receipt.outbox_id=cancel.outbox_id
               AND reconcile_receipt.outbox_digest=cancel.outbox_digest
               AND reconcile_receipt.send_attempt_id=cancel_send.send_attempt_id
               AND observation.observation_id=NEW.observation_id
               AND observation.observation_digest=NEW.observation_digest
               AND observation.send_attempt_id=cancel_send.send_attempt_id
               AND observation.outbox_id=cancel.outbox_id
               AND observation.outbox_digest=cancel.outbox_digest
               AND observation.operation_kind='cancel'
               AND observation.observation_kind='reconcile_attestation'
               AND observation.command_id=prepare.command_id
               AND observation.command_digest=prepare.command_digest
               AND observation.provider_id=prepare.provider_id
               AND observation.adapter_id=prepare.adapter_id
               AND observation.adapter_binding_digest=prepare.adapter_binding_digest
               AND observation.response_outcome='observed'
               AND observation.remote_execution_state='terminal_no_start'
               AND observation.terminality='final'
               AND observation.no_commit_tombstone_id=NEW.no_commit_tombstone_id
               AND observation.no_commit_tombstone_digest=NEW.no_commit_tombstone_digest
               AND reconcile_receipt.authenticated_at=observation.authenticated_at
               AND reconcile_receipt.received_at=observation.received_at
               AND reconcile_receipt.recorded_at=observation.recorded_at
               AND observation.recorded_at<=NEW.proven_at
               AND NEW.proven_at<=NEW.recorded_at
               AND NOT EXISTS (SELECT 1 FROM compute_attempt_activations activation
                                WHERE activation.lease_id=NEW.lease_id
                                   OR activation.reservation_id=NEW.reservation_id)
               AND NOT EXISTS (SELECT 1 FROM compute_attempt_dispatch_applications application
                                WHERE application.command_id=NEW.command_id
                                   OR application.lease_id=NEW.lease_id)
               AND NOT EXISTS (SELECT 1 FROM compute_attempt_start_outbox commit_intent
                                JOIN compute_attempt_start_send_attempts commit_send
                                  ON commit_send.outbox_id=commit_intent.outbox_id
                                 AND commit_send.outbox_digest=commit_intent.outbox_digest
                               WHERE commit_intent.command_id=NEW.command_id
                                 AND commit_intent.operation_kind='commit')
               AND NOT EXISTS (SELECT 1 FROM compute_attempt_start_remote_observations conflict
                                WHERE conflict.command_id=NEW.command_id
                                  AND conflict.remote_execution_state IN
                                      ('committed','running','terminal_after_run'))"#
}
