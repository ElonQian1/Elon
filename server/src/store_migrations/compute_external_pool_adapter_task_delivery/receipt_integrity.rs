use anyhow::Result;
use rusqlite::{functions::FunctionFlags, Connection};

use crate::compute_federation::external_pool_adapter_task_protocol_production::*;

const ATTEMPT_EXACT: &str = "elon_v273_task_exchange_attempt_is_exact";
const RECEIPT_EXACT: &str = "elon_v273_task_exchange_receipt_is_exact";
const RECONCILE_POLL_EXACT: &str = "elon_v273_task_reconcile_poll_is_exact";
const EVENT_POLL_EXACT: &str = "elon_v273_task_event_poll_is_exact";
const EVENT_BATCH_EXACT: &str = "elon_v273_task_event_batch_is_exact";
const EVENT_EXACT: &str = "elon_v273_task_event_is_exact";

pub(super) fn register(conn: &Connection) -> Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    conn.create_scalar_function(ATTEMPT_EXACT, 1, flags, |context| {
        Ok(i64::from(text(context, 0).is_some_and(attempt_is_exact)))
    })?;
    conn.create_scalar_function(RECEIPT_EXACT, 1, flags, |context| {
        Ok(i64::from(text(context, 0).is_some_and(receipt_is_exact)))
    })?;
    conn.create_scalar_function(RECONCILE_POLL_EXACT, 1, flags, |context| {
        Ok(i64::from(
            text(context, 0).is_some_and(reconcile_poll_is_exact),
        ))
    })?;
    conn.create_scalar_function(EVENT_POLL_EXACT, 1, flags, |context| {
        Ok(i64::from(text(context, 0).is_some_and(event_poll_is_exact)))
    })?;
    conn.create_scalar_function(EVENT_BATCH_EXACT, 1, flags, |context| {
        Ok(i64::from(
            text(context, 0).is_some_and(event_batch_is_exact),
        ))
    })?;
    conn.create_scalar_function(EVENT_EXACT, 1, flags, |context| {
        Ok(i64::from(text(context, 0).is_some_and(event_is_exact)))
    })?;
    Ok(())
}

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS v273_task_exchange_attempt_integrity
         BEFORE INSERT ON compute_external_pool_adapter_task_exchange_attempts
         WHEN {ATTEMPT_EXACT}(NEW.exchange_attempt_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V273 exchange attempt canonical integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v273_task_exchange_receipt_integrity
         BEFORE INSERT ON compute_external_pool_adapter_task_exchange_receipts
         WHEN {RECEIPT_EXACT}(NEW.exchange_receipt_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V273 exchange receipt canonical integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v273_task_reconcile_poll_integrity
         BEFORE INSERT ON compute_external_pool_adapter_task_reconcile_polls
         WHEN {RECONCILE_POLL_EXACT}(NEW.reconcile_poll_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V273 reconcile poll canonical integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v273_task_event_poll_integrity
         BEFORE INSERT ON compute_external_pool_adapter_task_event_polls
         WHEN {EVENT_POLL_EXACT}(NEW.event_poll_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V273 event poll canonical integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v273_task_event_batch_integrity
         BEFORE INSERT ON compute_external_pool_adapter_task_event_batches
         WHEN {EVENT_BATCH_EXACT}(NEW.event_batch_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V273 event batch canonical integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v273_task_event_integrity
         BEFORE INSERT ON compute_external_pool_adapter_task_events
         WHEN {EVENT_EXACT}(NEW.event_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V273 event canonical integrity mismatch'); END;"
    ))?;
    Ok(())
}

fn text<'a>(context: &'a rusqlite::functions::Context<'a>, index: usize) -> Option<&'a str> {
    context.get_raw(index).as_str().ok()
}

fn attempt_is_exact(json: &str) -> bool {
    let Ok(value) = bounded_parse::<ExternalPoolAdapterTaskExchangeAttemptEnvelope>(json) else {
        return false;
    };
    validate_task_production_exchange_attempt(&value).is_ok()
        && canonical_task_production_exchange_attempt_json_and_digest(&value)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn receipt_is_exact(json: &str) -> bool {
    let Ok(value) = bounded_parse::<ExternalPoolAdapterTaskExchangeReceiptEnvelope>(json) else {
        return false;
    };
    validate_task_production_exchange_receipt(&value).is_ok()
        && canonical_task_production_exchange_receipt_json_and_digest(&value)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn reconcile_poll_is_exact(json: &str) -> bool {
    let Ok(value) = bounded_parse::<ExternalPoolAdapterTaskReconcilePollEnvelope>(json) else {
        return false;
    };
    validate_task_production_reconcile_poll(&value).is_ok()
        && canonical_task_production_reconcile_poll_json_and_digest(&value)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn event_poll_is_exact(json: &str) -> bool {
    let Ok(value) = bounded_parse::<ExternalPoolAdapterTaskEventPollEnvelope>(json) else {
        return false;
    };
    validate_task_production_event_poll(&value).is_ok()
        && canonical_task_production_event_poll_json_and_digest(&value)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn event_batch_is_exact(json: &str) -> bool {
    let Ok(value) = bounded_parse::<ExternalPoolAdapterTaskEventBatchEnvelope>(json) else {
        return false;
    };
    validate_task_production_event_batch(&value).is_ok()
        && canonical_task_production_event_batch_json_and_digest(&value)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn event_is_exact(json: &str) -> bool {
    let Ok(value) = bounded_parse::<ExternalPoolAdapterTaskEventEnvelope>(json) else {
        return false;
    };
    validate_task_production_event(&value).is_ok()
        && canonical_task_production_event_json_and_digest(&value)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn bounded_parse<T: serde::de::DeserializeOwned>(json: &str) -> Result<T> {
    if json.len() > TASK_PRODUCTION_MAX_LEDGER_JSON_BYTES {
        anyhow::bail!("V273 durable envelope exceeds the bound")
    }
    Ok(serde_json::from_str(json)?)
}
