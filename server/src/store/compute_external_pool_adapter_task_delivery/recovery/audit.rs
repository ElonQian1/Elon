use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    validate_task_production_event_remote_state_transition,
    ExternalPoolAdapterTaskExchangeIdentity, ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    ExternalPoolAdapterTaskPollCommandBinding, TASK_PRODUCTION_MAX_EVENTS_PER_BATCH,
};

use super::super::{
    read::{
        read_event_batch_on, read_event_on, read_exchange_attempt_on, read_exchange_receipt_on,
    },
    types::{
        AuditedEventPoll, AuditedReconcilePoll, CLAIM_STATUS_DELIVERY_OBSERVED,
        CLAIM_STATUS_IN_FLIGHT_UNKNOWN, CLAIM_STATUS_PENDING,
    },
};

pub(super) fn audit_reconcile_target_on(
    conn: &Connection,
    poll: &AuditedReconcilePoll,
    target: &str,
) -> Result<usize> {
    match target {
        CLAIM_STATUS_DELIVERY_OBSERVED => {
            let receipt_id = source_receipt_id_on(
                conn,
                "reconcile_poll",
                &poll.envelope.reconcile_poll_id,
                &poll.envelope.reconcile_poll_digest,
            )?
            .ok_or_else(|| anyhow::anyhow!("V273 observed reconcile poll lacks receipt"))?;
            let receipt = read_exchange_receipt_on(conn, &receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("V273 reconcile receipt disappeared"))?;
            ensure!(
                receipt.receipt.identity.operation_kind == "reconcile"
                    && receipt.receipt.identity.request_digest == poll.envelope.poll.request_digest
                    && receipt.receipt.identity.source.source_kind == "reconcile_poll"
                    && receipt.receipt.identity.source.source_id == poll.envelope.reconcile_poll_id
                    && receipt.receipt.identity.source.source_digest
                        == poll.envelope.reconcile_poll_digest
                    && identity_matches_command(
                        &receipt.receipt.identity,
                        &poll.envelope.poll.command
                    ),
                "V273 reconcile receipt does not close the exact poll"
            );
            audit_receipt_attempt_on(conn, &receipt)?;
            Ok(2)
        }
        CLAIM_STATUS_IN_FLIGHT_UNKNOWN => {
            audit_unknown_attempt_on(
                conn,
                "reconcile_poll",
                &poll.envelope.reconcile_poll_id,
                &poll.envelope.reconcile_poll_digest,
                &poll.envelope.poll.command,
                &poll.envelope.poll.request_digest,
            )?;
            Ok(1)
        }
        CLAIM_STATUS_PENDING => {
            ensure_no_source_attempt_on(
                conn,
                "reconcile_poll",
                &poll.envelope.reconcile_poll_id,
                &poll.envelope.reconcile_poll_digest,
            )?;
            Ok(0)
        }
        _ => anyhow::bail!("V273 reconcile recovery target is unsupported"),
    }
}

pub(super) fn audit_event_target_on(
    conn: &Connection,
    poll: &AuditedEventPoll,
    target: &str,
) -> Result<usize> {
    match target {
        CLAIM_STATUS_DELIVERY_OBSERVED => audit_complete_event_batch_on(conn, poll),
        CLAIM_STATUS_IN_FLIGHT_UNKNOWN => {
            audit_unknown_attempt_on(
                conn,
                "event_poll",
                &poll.envelope.event_poll_id,
                &poll.envelope.event_poll_digest,
                &poll.envelope.poll.command,
                &poll.envelope.poll.request_digest,
            )?;
            Ok(1)
        }
        CLAIM_STATUS_PENDING => {
            ensure_no_source_attempt_on(
                conn,
                "event_poll",
                &poll.envelope.event_poll_id,
                &poll.envelope.event_poll_digest,
            )?;
            Ok(0)
        }
        _ => anyhow::bail!("V273 event recovery target is unsupported"),
    }
}

fn audit_unknown_attempt_on(
    conn: &Connection,
    source_kind: &str,
    source_id: &str,
    source_digest: &str,
    command: &ExternalPoolAdapterTaskPollCommandBinding,
    request_digest: &str,
) -> Result<()> {
    let expected_operation = if source_kind == "reconcile_poll" {
        "reconcile"
    } else {
        "authenticated_events"
    };
    let attempt_id = conn
        .query_row(
            "SELECT attempt.exchange_attempt_id
               FROM compute_external_pool_adapter_task_exchange_attempts attempt
              WHERE attempt.source_kind=?1 AND attempt.source_id=?2 AND attempt.source_digest=?3
                AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                                 WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id
                                   AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest)",
            params![source_kind, source_id, source_digest],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("V273 remote-unknown poll lacks exact attempt"))?;
    let attempt = read_exchange_attempt_on(conn, &attempt_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 remote-unknown attempt disappeared"))?;
    ensure!(
        attempt.attempt.identity.operation_kind == expected_operation
            && attempt.attempt.identity.request_digest == request_digest
            && attempt.attempt.identity.source.source_kind == source_kind
            && attempt.attempt.identity.source.source_id == source_id
            && attempt.attempt.identity.source.source_digest == source_digest
            && identity_matches_command(&attempt.attempt.identity, command),
        "V273 remote-unknown attempt does not match poll custody"
    );
    ensure_no_attempt_receipt_on(
        conn,
        &attempt.exchange_attempt_id,
        &attempt.exchange_attempt_digest,
    )
}

fn audit_complete_event_batch_on(conn: &Connection, poll: &AuditedEventPoll) -> Result<usize> {
    let batch_id = conn
        .query_row(
            "SELECT batch.event_batch_id
               FROM compute_external_pool_adapter_task_event_batches batch
              WHERE batch.event_poll_id=?1 AND batch.event_poll_digest=?2",
            params![poll.envelope.event_poll_id, poll.envelope.event_poll_digest],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("V273 observed event poll lacks batch"))?;
    let batch = read_event_batch_on(conn, &batch_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 event batch disappeared"))?;
    validate_task_production_event_remote_state_transition(
        &poll.envelope.poll.remote.remote_execution_state,
        &batch.batch.remote.remote_execution_state,
    )?;
    ensure!(
        batch.batch.event_poll_id == poll.envelope.event_poll_id
            && batch.batch.event_poll_digest == poll.envelope.event_poll_digest
            && batch.batch.remote.executor_binding_digest
                == poll.envelope.poll.remote.executor_binding_digest
            && batch.batch.remote.remote_execution_id
                == poll.envelope.poll.remote.remote_execution_id
            && batch.batch.remote.remote_identity_digest
                == poll.envelope.poll.remote.remote_identity_digest
            && batch.batch.cursor_before == poll.envelope.poll.requested_cursor
            && batch.batch.event_count <= TASK_PRODUCTION_MAX_EVENTS_PER_BATCH,
        "V273 event batch does not match bounded poll custody"
    );
    let receipt = read_exchange_receipt_on(conn, &batch.batch.exchange_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 event batch receipt disappeared"))?;
    ensure!(
        receipt.exchange_receipt_digest == batch.batch.exchange_receipt_digest
            && receipt.receipt.identity.operation_kind == "authenticated_events"
            && receipt.receipt.identity.source.source_kind == "event_poll"
            && receipt.receipt.identity.source.source_id == poll.envelope.event_poll_id
            && receipt.receipt.identity.source.source_digest == poll.envelope.event_poll_digest
            && receipt.receipt.identity.request_digest == poll.envelope.poll.request_digest
            && receipt.receipt.semantic_observation_sha256
                == batch.batch.authenticated_observation_sha256
            && identity_matches_command(&receipt.receipt.identity, &poll.envelope.poll.command),
        "V273 event batch receipt does not close the exact poll"
    );
    audit_receipt_attempt_on(conn, &receipt)?;
    let mut statement = conn.prepare(
        "SELECT event_id FROM compute_external_pool_adapter_task_events
          WHERE event_batch_id=?1 ORDER BY event_ordinal LIMIT ?2",
    )?;
    let event_ids = statement
        .query_map(
            params![
                batch.event_batch_id,
                i64::try_from(TASK_PRODUCTION_MAX_EVENTS_PER_BATCH + 1)?
            ],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(
        u64::try_from(event_ids.len())? == batch.batch.event_count,
        "V273 event batch durable event count is not exact"
    );
    for (index, event_id) in event_ids.iter().enumerate() {
        let event = read_event_on(conn, event_id)?
            .ok_or_else(|| anyhow::anyhow!("V273 ordered event disappeared"))?;
        let expected_ordinal = u64::try_from(index)? + 1;
        let expected_root = batch
            .batch
            .event_roots
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("V273 event root inventory is incomplete"))?;
        let expected_previous_root = if index == 0 {
            batch.batch.cursor_before.previous_event_root.as_deref()
        } else {
            batch.batch.event_roots.get(index - 1).map(String::as_str)
        };
        let expected_remote_sequence = batch
            .batch
            .cursor_before
            .remote_sequence
            .checked_add(expected_ordinal)
            .ok_or_else(|| anyhow::anyhow!("V273 event remote sequence overflow"))?;
        ensure!(
            event.event.event_batch_id == batch.event_batch_id
                && event.event.event_batch_digest == batch.event_batch_digest
                && event.event.remote_identity_digest == batch.batch.remote.remote_identity_digest
                && event.event.event_ordinal == expected_ordinal
                && event.event.remote_sequence == expected_remote_sequence
                && event.event.previous_event_root.as_deref() == expected_previous_root
                && event.event.event_root == expected_root.as_str()
                && event.event.recorded_at == batch.batch.recorded_at,
            "V273 ordered event does not match exact batch projection"
        );
    }
    Ok(event_ids.len() + 3)
}

fn audit_receipt_attempt_on(
    conn: &Connection,
    receipt: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<()> {
    let attempt = read_exchange_attempt_on(conn, &receipt.receipt.exchange_attempt_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 receipt source attempt disappeared"))?;
    ensure!(
        attempt.exchange_attempt_digest == receipt.receipt.exchange_attempt_digest
            && attempt.attempt.identity == receipt.receipt.identity,
        "V273 receipt does not reprove its exact audited attempt"
    );
    Ok(())
}

fn source_receipt_id_on(
    conn: &Connection,
    source_kind: &str,
    source_id: &str,
    source_digest: &str,
) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT exchange_receipt_id
               FROM compute_external_pool_adapter_task_exchange_receipts
              WHERE source_kind=?1 AND source_id=?2 AND source_digest=?3",
            params![source_kind, source_id, source_digest],
            |row| row.get(0),
        )
        .optional()?)
}

fn ensure_no_source_attempt_on(
    conn: &Connection,
    source_kind: &str,
    source_id: &str,
    source_digest: &str,
) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts
          WHERE source_kind=?1 AND source_id=?2 AND source_digest=?3)",
        params![source_kind, source_id, source_digest],
        |row| row.get(0),
    )?;
    ensure!(!exists, "V273 expired poll has a source attempt");
    Ok(())
}

fn ensure_no_attempt_receipt_on(
    conn: &Connection,
    attempt_id: &str,
    attempt_digest: &str,
) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts
          WHERE exchange_attempt_id=?1 AND exchange_attempt_digest=?2)",
        params![attempt_id, attempt_digest],
        |row| row.get(0),
    )?;
    ensure!(!exists, "V273 remote-unknown attempt already has a receipt");
    Ok(())
}

fn identity_matches_command(
    identity: &ExternalPoolAdapterTaskExchangeIdentity,
    command: &ExternalPoolAdapterTaskPollCommandBinding,
) -> bool {
    identity.command.command_id == command.command_id
        && identity.command.command_digest == command.command_digest
        && identity.command.outbox_id == command.outbox_id
        && identity.command.outbox_digest == command.outbox_digest
        && identity.command.send_attempt_id == command.send_attempt_id
        && identity.command.send_attempt_digest == command.send_attempt_digest
        && identity.route.route_authorization_id == command.route_authorization_id
        && identity.route.route_authorization_digest == command.route_authorization_digest
        && identity.executor_binding_digest == command.executor_binding_digest
        && identity.fencing_generation == command.fencing_generation
        && identity.fence_digest == command.fence_digest
}
