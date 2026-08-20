//! Preserve the V213 prepare-observation guard while admitting one receipt-bound reconcile proof.

use anyhow::{ensure, Result};
use rusqlite::Connection;

const OBSERVATION_TRIGGER: &str = "trg_compute_attempt_start_observation_exact_attempt";
const ACK_TRIGGER: &str = "trg_compute_attempt_adapter_ack_requires_observation_v213";
const MARKER: &str = "external_pool_adapter_task_receipt.v1";

pub(super) fn install(connection: &Connection) -> Result<()> {
    install_observation(connection)?;
    install_ack(connection)
}

fn install_observation(connection: &Connection) -> Result<()> {
    let receipt_branch = format!(
        r#" OR EXISTS (
            SELECT 1
              FROM compute_attempt_start_send_attempts attempt
              JOIN compute_attempt_start_outbox outbox ON outbox.outbox_id=attempt.outbox_id
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=attempt.route_authorization_id
               AND route.route_authorization_digest=attempt.route_authorization_digest
              JOIN compute_route_authorization_capabilities capability
                ON capability.route_authorization_id=route.route_authorization_id
               AND capability.capability_id='authenticated_ack'
              JOIN compute_external_pool_adapter_task_exchange_receipts receipt
                ON receipt.exchange_receipt_id=NEW.verifier_id
               AND receipt.semantic_observation_sha256=NEW.verification_digest
             WHERE NEW.verification_kind='{MARKER}'
               AND NEW.operation_kind=attempt.operation_kind
               AND attempt.send_attempt_id=NEW.send_attempt_id
               AND attempt.outbox_id=NEW.outbox_id
               AND attempt.outbox_digest=NEW.outbox_digest
               AND attempt.command_id=NEW.command_id
               AND attempt.command_digest=NEW.command_digest
               AND outbox.outbox_digest=NEW.outbox_digest
               AND outbox.provider_id=NEW.provider_id
               AND outbox.adapter_id=NEW.adapter_id
               AND outbox.adapter_binding_digest=NEW.adapter_binding_digest
               AND outbox.state='in_flight_unknown'
               AND route.provider_id=NEW.provider_id
               AND route.adapter_id=NEW.adapter_id
               AND route.adapter_binding_digest=NEW.adapter_binding_digest
               AND route.authenticated_at<=NEW.authenticated_at
               AND NEW.authenticated_at<route.cleanup_expires_at
               AND attempt.started_at<=NEW.received_at
               AND receipt.command_id=NEW.command_id
               AND receipt.command_digest=NEW.command_digest
               AND receipt.outbox_id=NEW.outbox_id
               AND receipt.outbox_digest=NEW.outbox_digest
               AND receipt.send_attempt_id=NEW.send_attempt_id
               AND receipt.provider_id=NEW.provider_id
               AND receipt.adapter_id=NEW.adapter_id
               AND receipt.route_authorization_id=route.route_authorization_id
               AND receipt.route_authorization_digest=route.route_authorization_digest
               AND receipt.authenticated_at=NEW.authenticated_at
               AND receipt.received_at=NEW.received_at
               AND receipt.recorded_at=NEW.recorded_at
               AND ((receipt.operation_kind='prepare'
                AND receipt.source_kind='start_outbox_send_attempt'
                     AND receipt.source_id=NEW.send_attempt_id
                     AND NEW.operation_kind='prepare'
                     AND NEW.observation_kind='prepare_response')
                 OR (receipt.operation_kind='reconcile'
                     AND receipt.source_kind='reconcile_poll'
                     AND NEW.operation_kind IN ('prepare','commit','cancel')
                     AND NEW.observation_kind='reconcile_attestation'
                     AND EXISTS (
                       SELECT 1 FROM compute_external_pool_adapter_task_reconcile_polls poll
                        WHERE poll.reconcile_poll_id=receipt.source_id
                          AND poll.reconcile_poll_digest=receipt.source_digest
                          AND poll.claim_status='delivery_observed')))
        )"#
    );
    append_exists_branch(
        connection,
        OBSERVATION_TRIGGER,
        "route.verification_kind=NEW.verification_kind",
        &receipt_branch,
    )
}

fn install_ack(connection: &Connection) -> Result<()> {
    let sql: String = connection.query_row(
        "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
        [ACK_TRIGGER],
        |row| row.get(0),
    )?;
    if sql.contains(MARKER) {
        ensure!(
            sql.matches(MARKER).count() == 1,
            "V278 reconcile ACK branch is installed more than once"
        );
        return Ok(());
    }
    ensure!(
        sql.contains("WHEN NOT EXISTS (")
            && sql.contains("observation.observation_kind='prepare_response'")
            && sql.contains("NEW.outcome='accepted'")
            && sql.contains("BEGIN"),
        "V278 reconcile ACK predecessor guard drifted"
    );
    let sql = sql.replacen("WHEN NOT EXISTS (", "WHEN NOT (EXISTS (", 1);
    let begin = sql
        .rfind("BEGIN")
        .ok_or_else(|| anyhow::anyhow!("V278 reconcile ACK guard lost BEGIN"))?;
    let reconcile = format!(
        r#" OR EXISTS (
            SELECT 1
              FROM compute_attempt_start_remote_observations observation
              JOIN compute_attempt_start_outbox outbox ON outbox.outbox_id=observation.outbox_id
              JOIN compute_external_pool_adapter_task_exchange_receipts receipt
                ON receipt.exchange_receipt_id=observation.verifier_id
               AND receipt.semantic_observation_sha256=observation.verification_digest
             WHERE outbox.operation_kind='prepare'
               AND outbox.command_id=NEW.command_id
               AND outbox.command_digest=NEW.command_digest
               AND outbox.adapter_binding_digest=NEW.adapter_binding_digest
               AND receipt.operation_kind='reconcile'
               AND receipt.source_kind='reconcile_poll'
               AND receipt.command_id=NEW.command_id
               AND receipt.command_digest=NEW.command_digest
               AND receipt.outbox_id=outbox.outbox_id
               AND receipt.outbox_digest=outbox.outbox_digest
               AND observation.operation_kind='prepare'
               AND observation.observation_kind='reconcile_attestation'
               AND observation.command_id=NEW.command_id
               AND observation.command_digest=NEW.command_digest
               AND observation.provider_id=NEW.provider_id
               AND observation.adapter_id=NEW.adapter_id
               AND observation.adapter_binding_digest=NEW.adapter_binding_digest
               AND observation.adapter_observation_id=NEW.adapter_ack_id
               AND observation.response_outcome='accepted'
               AND observation.remote_execution_state IN ('prepared','committed','running')
               AND observation.terminality='non_terminal'
               AND observation.remote_execution_ref IS NEW.remote_execution_ref
               AND observation.reason_code IS NULL
               AND observation.observed_at=NEW.observed_at
               AND observation.received_at=NEW.received_at
               AND observation.recorded_at<=NEW.created_at
               AND observation.verification_kind='{MARKER}')
        )
"#
    );
    let replacement = format!("{}{reconcile}{}", &sql[..begin], &sql[begin..]);
    connection.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS {ACK_TRIGGER};\n{replacement};"
    ))?;
    let installed: String = connection.query_row(
        "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
        [ACK_TRIGGER],
        |row| row.get(0),
    )?;
    ensure!(
        installed.matches(MARKER).count() == 1
            && installed.contains("observation.observation_kind='prepare_response'")
            && installed.contains("observation.observation_kind='reconcile_attestation'"),
        "V278 reconcile ACK branch did not preserve both exact sources"
    );
    Ok(())
}

fn append_exists_branch(
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
            "V278 receipt observation branch is not exact"
        );
        return Ok(());
    }
    ensure!(
        sql.contains("WHEN NOT EXISTS (") && sql.contains(legacy_marker),
        "V278 receipt observation predecessor guard drifted"
    );
    let sql = sql.replacen("WHEN NOT EXISTS (", "WHEN NOT (EXISTS (", 1);
    let begin = sql
        .rfind("BEGIN")
        .ok_or_else(|| anyhow::anyhow!("V278 receipt observation guard lost BEGIN"))?;
    let replacement = format!("{}{branch})\n{}", &sql[..begin], &sql[begin..]);
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
        "V278 receipt observation branch was not installed"
    );
    Ok(())
}
