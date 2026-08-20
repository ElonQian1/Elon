//! Exact forward writers for the six immutable V273 ledgers.

use anyhow::{ensure, Result};
use rusqlite::{params_from_iter, types::Value, Connection};

use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    validate_task_production_event, validate_task_production_event_batch,
    validate_task_production_event_poll, validate_task_production_exchange_attempt,
    validate_task_production_exchange_receipt, validate_task_production_reconcile_poll,
    ExternalPoolAdapterTaskEventBatchEnvelope, ExternalPoolAdapterTaskEventEnvelope,
    ExternalPoolAdapterTaskEventPollEnvelope, ExternalPoolAdapterTaskExchangeAttemptEnvelope,
    ExternalPoolAdapterTaskExchangeReceiptEnvelope, ExternalPoolAdapterTaskReconcilePollEnvelope,
};

use super::{
    columns::*,
    mapping::*,
    reachability_pending_plan::ExternalPoolAdapterTaskReachabilityPendingPlanGuard,
    read::{
        read_event_batch_on, read_event_on, read_event_poll_on, read_exchange_attempt_on,
        read_exchange_receipt_on, read_reconcile_poll_on,
    },
    types::{ExternalPoolAdapterTaskLedgerWriteDisposition, PollClaimProjection},
};

pub(in crate::store) fn insert_external_pool_adapter_task_exchange_attempt_on(
    connection: &Connection,
    plan: Option<&ExternalPoolAdapterTaskReachabilityPendingPlanGuard>,
    envelope: &ExternalPoolAdapterTaskExchangeAttemptEnvelope,
) -> Result<ExternalPoolAdapterTaskLedgerWriteDisposition> {
    validate_task_production_exchange_attempt(envelope)?;
    let values = exchange_attempt_values(envelope)?;
    if !exchange_attempt_needs_insert_on(connection, envelope)? {
        return Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay);
    }
    ensure!(
        plan.is_some(),
        "V278 fresh exchange attempt lacks a pending plan"
    );
    insert_values_on(
        connection,
        EXCHANGE_ATTEMPT_TABLE,
        &EXCHANGE_ATTEMPT_COLUMNS,
        &values,
    )?;
    ensure!(
        read_exchange_attempt_on(connection, &envelope.exchange_attempt_id)?.as_ref()
            == Some(envelope),
        "V273 exchange attempt readback is not exact"
    );
    Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted)
}

pub(in crate::store) fn insert_external_pool_adapter_task_exchange_receipt_on(
    connection: &Connection,
    plan: Option<&ExternalPoolAdapterTaskReachabilityPendingPlanGuard>,
    envelope: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<ExternalPoolAdapterTaskLedgerWriteDisposition> {
    validate_task_production_exchange_receipt(envelope)?;
    let values = exchange_receipt_values(envelope)?;
    if !exchange_receipt_needs_insert_on(connection, envelope)? {
        return Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay);
    }
    ensure!(
        plan.is_some(),
        "V278 fresh exchange receipt lacks a pending plan"
    );
    insert_values_on(
        connection,
        EXCHANGE_RECEIPT_TABLE,
        &EXCHANGE_RECEIPT_COLUMNS,
        &values,
    )?;
    ensure!(
        read_exchange_receipt_on(connection, &envelope.exchange_receipt_id)?.as_ref()
            == Some(envelope),
        "V273 exchange receipt readback is not exact"
    );
    Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted)
}

pub(in crate::store) fn insert_external_pool_adapter_task_reconcile_poll_on(
    connection: &Connection,
    plan: Option<&ExternalPoolAdapterTaskReachabilityPendingPlanGuard>,
    envelope: &ExternalPoolAdapterTaskReconcilePollEnvelope,
) -> Result<ExternalPoolAdapterTaskLedgerWriteDisposition> {
    validate_task_production_reconcile_poll(envelope)?;
    let values = reconcile_poll_values(envelope, &initial_poll_claim())?;
    if !reconcile_poll_needs_insert_on(connection, envelope)? {
        return Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay);
    }
    ensure!(
        plan.is_some(),
        "V278 fresh reconcile poll lacks a pending plan"
    );
    insert_values_on(
        connection,
        RECONCILE_POLL_TABLE,
        &RECONCILE_POLL_COLUMNS,
        &values,
    )?;
    ensure!(
        read_reconcile_poll_on(connection, &envelope.reconcile_poll_id)?.is_some_and(|stored| {
            stored.envelope == *envelope && stored.claim == initial_poll_claim()
        }),
        "V273 reconcile poll readback is not exact"
    );
    Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted)
}

pub(in crate::store) fn insert_external_pool_adapter_task_event_poll_on(
    connection: &Connection,
    plan: Option<&ExternalPoolAdapterTaskReachabilityPendingPlanGuard>,
    envelope: &ExternalPoolAdapterTaskEventPollEnvelope,
) -> Result<ExternalPoolAdapterTaskLedgerWriteDisposition> {
    validate_task_production_event_poll(envelope)?;
    let values = event_poll_values(envelope, &initial_poll_claim())?;
    if !event_poll_needs_insert_on(connection, envelope)? {
        return Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay);
    }
    ensure!(plan.is_some(), "V278 fresh event poll lacks a pending plan");
    insert_values_on(connection, EVENT_POLL_TABLE, &EVENT_POLL_COLUMNS, &values)?;
    ensure!(
        read_event_poll_on(connection, &envelope.event_poll_id)?.is_some_and(|stored| {
            stored.envelope == *envelope && stored.claim == initial_poll_claim()
        }),
        "V273 event poll readback is not exact"
    );
    Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted)
}

pub(in crate::store) fn insert_external_pool_adapter_task_event_batch_on(
    connection: &Connection,
    plan: Option<&ExternalPoolAdapterTaskReachabilityPendingPlanGuard>,
    envelope: &ExternalPoolAdapterTaskEventBatchEnvelope,
) -> Result<ExternalPoolAdapterTaskLedgerWriteDisposition> {
    validate_task_production_event_batch(envelope)?;
    let values = event_batch_values(envelope)?;
    if !event_batch_needs_insert_on(connection, envelope)? {
        return Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay);
    }
    ensure!(
        plan.is_some(),
        "V278 fresh event batch lacks a pending plan"
    );
    insert_values_on(connection, EVENT_BATCH_TABLE, &EVENT_BATCH_COLUMNS, &values)?;
    ensure!(
        read_event_batch_on(connection, &envelope.event_batch_id)?.as_ref() == Some(envelope),
        "V273 event batch readback is not exact"
    );
    Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted)
}

pub(in crate::store) fn insert_external_pool_adapter_task_event_on(
    connection: &Connection,
    plan: Option<&ExternalPoolAdapterTaskReachabilityPendingPlanGuard>,
    envelope: &ExternalPoolAdapterTaskEventEnvelope,
) -> Result<ExternalPoolAdapterTaskLedgerWriteDisposition> {
    validate_task_production_event(envelope)?;
    let values = event_values(envelope)?;
    if !event_needs_insert_on(connection, envelope)? {
        return Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay);
    }
    ensure!(plan.is_some(), "V278 fresh event lacks a pending plan");
    insert_values_on(connection, EVENT_TABLE, &EVENT_COLUMNS, &values)?;
    ensure!(
        read_event_on(connection, &envelope.event_id)?.as_ref() == Some(envelope),
        "V273 event readback is not exact"
    );
    Ok(ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted)
}

pub(super) fn exchange_attempt_needs_insert_on(
    connection: &Connection,
    envelope: &ExternalPoolAdapterTaskExchangeAttemptEnvelope,
) -> Result<bool> {
    validate_task_production_exchange_attempt(envelope)?;
    match read_exchange_attempt_on(connection, &envelope.exchange_attempt_id)? {
        Some(existing) => {
            ensure!(existing == *envelope, "V273 exchange attempt id conflicts");
            Ok(false)
        }
        None => Ok(true),
    }
}

pub(super) fn exchange_receipt_needs_insert_on(
    connection: &Connection,
    envelope: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<bool> {
    validate_task_production_exchange_receipt(envelope)?;
    match read_exchange_receipt_on(connection, &envelope.exchange_receipt_id)? {
        Some(existing) => {
            ensure!(existing == *envelope, "V273 exchange receipt id conflicts");
            Ok(false)
        }
        None => Ok(true),
    }
}

pub(super) fn reconcile_poll_needs_insert_on(
    connection: &Connection,
    envelope: &ExternalPoolAdapterTaskReconcilePollEnvelope,
) -> Result<bool> {
    validate_task_production_reconcile_poll(envelope)?;
    match read_reconcile_poll_on(connection, &envelope.reconcile_poll_id)? {
        Some(existing) => {
            ensure!(
                existing.envelope == *envelope,
                "V273 reconcile poll id conflicts"
            );
            Ok(false)
        }
        None => Ok(true),
    }
}

pub(super) fn event_poll_needs_insert_on(
    connection: &Connection,
    envelope: &ExternalPoolAdapterTaskEventPollEnvelope,
) -> Result<bool> {
    validate_task_production_event_poll(envelope)?;
    match read_event_poll_on(connection, &envelope.event_poll_id)? {
        Some(existing) => {
            ensure!(
                existing.envelope == *envelope,
                "V273 event poll id conflicts"
            );
            Ok(false)
        }
        None => Ok(true),
    }
}

pub(super) fn event_batch_needs_insert_on(
    connection: &Connection,
    envelope: &ExternalPoolAdapterTaskEventBatchEnvelope,
) -> Result<bool> {
    validate_task_production_event_batch(envelope)?;
    match read_event_batch_on(connection, &envelope.event_batch_id)? {
        Some(existing) => {
            ensure!(existing == *envelope, "V273 event batch id conflicts");
            Ok(false)
        }
        None => Ok(true),
    }
}

pub(super) fn event_needs_insert_on(
    connection: &Connection,
    envelope: &ExternalPoolAdapterTaskEventEnvelope,
) -> Result<bool> {
    validate_task_production_event(envelope)?;
    match read_event_on(connection, &envelope.event_id)? {
        Some(existing) => {
            ensure!(existing == *envelope, "V273 event id conflicts");
            Ok(false)
        }
        None => Ok(true),
    }
}

fn insert_values_on(
    connection: &Connection,
    table: &str,
    columns: &[&str],
    values: &[Value],
) -> Result<()> {
    ensure!(columns.len() == values.len(), "V273 writer arity drifted");
    let placeholders = (1..=values.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({placeholders})",
        columns.join(",")
    );
    let changed = connection.execute(&sql, params_from_iter(values.iter()))?;
    ensure!(changed == 1, "V273 writer did not insert exactly one row");
    Ok(())
}

fn initial_poll_claim() -> PollClaimProjection {
    PollClaimProjection {
        status: super::types::CLAIM_STATUS_PENDING.to_string(),
        revision: 1,
        generation: 0,
        owner_id: None,
        token_digest: None,
        expires_at: None,
    }
}
