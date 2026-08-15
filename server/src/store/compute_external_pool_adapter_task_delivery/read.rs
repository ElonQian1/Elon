use anyhow::{bail, ensure, Result};
use chrono::{DateTime, FixedOffset, SecondsFormat};
use rusqlite::{params, types::Value, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    validate_task_production_event, validate_task_production_event_batch,
    validate_task_production_event_poll, validate_task_production_exchange_attempt,
    validate_task_production_exchange_receipt, validate_task_production_reconcile_poll,
    ExternalPoolAdapterTaskEventBatchEnvelope, ExternalPoolAdapterTaskEventEnvelope,
    ExternalPoolAdapterTaskEventPollEnvelope, ExternalPoolAdapterTaskExchangeAttemptEnvelope,
    ExternalPoolAdapterTaskExchangeReceiptEnvelope, ExternalPoolAdapterTaskReconcilePollEnvelope,
    TASK_PRODUCTION_MAX_LEDGER_JSON_BYTES, TASK_PRODUCTION_MAX_SAFE_INTEGER,
};

use super::{columns::*, mapping::*, types::*};

pub(in crate::store) fn read_exchange_attempt_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<ExternalPoolAdapterTaskExchangeAttemptEnvelope>> {
    let Some(row) = row_by_id_on(
        conn,
        EXCHANGE_ATTEMPT_TABLE,
        "exchange_attempt_id",
        id,
        &EXCHANGE_ATTEMPT_COLUMNS,
    )?
    else {
        return Ok(None);
    };
    let envelope: ExternalPoolAdapterTaskExchangeAttemptEnvelope = envelope_at(&row, 3)?;
    validate_task_production_exchange_attempt(&envelope)?;
    ensure_exact(
        &row,
        exchange_attempt_values(&envelope)?,
        "exchange attempt",
    )?;
    Ok(Some(envelope))
}

pub(in crate::store) fn read_exchange_receipt_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<ExternalPoolAdapterTaskExchangeReceiptEnvelope>> {
    let Some(row) = row_by_id_on(
        conn,
        EXCHANGE_RECEIPT_TABLE,
        "exchange_receipt_id",
        id,
        &EXCHANGE_RECEIPT_COLUMNS,
    )?
    else {
        return Ok(None);
    };
    let envelope: ExternalPoolAdapterTaskExchangeReceiptEnvelope = envelope_at(&row, 3)?;
    validate_task_production_exchange_receipt(&envelope)?;
    ensure_exact(
        &row,
        exchange_receipt_values(&envelope)?,
        "exchange receipt",
    )?;
    Ok(Some(envelope))
}

pub(super) fn read_reconcile_poll_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<AuditedReconcilePoll>> {
    let Some(row) = row_by_id_on(
        conn,
        RECONCILE_POLL_TABLE,
        "reconcile_poll_id",
        id,
        &RECONCILE_POLL_COLUMNS,
    )?
    else {
        return Ok(None);
    };
    let envelope: ExternalPoolAdapterTaskReconcilePollEnvelope = envelope_at(&row, 3)?;
    validate_task_production_reconcile_poll(&envelope)?;
    let claim = claim_projection_at(&row, 33)?;
    validate_claim_window(&claim, &envelope.poll.not_after)?;
    ensure_exact(
        &row,
        reconcile_poll_values(&envelope, &claim)?,
        "reconcile poll",
    )?;
    Ok(Some(AuditedReconcilePoll { envelope, claim }))
}

pub(super) fn read_event_poll_on(conn: &Connection, id: &str) -> Result<Option<AuditedEventPoll>> {
    let Some(row) = row_by_id_on(
        conn,
        EVENT_POLL_TABLE,
        "event_poll_id",
        id,
        &EVENT_POLL_COLUMNS,
    )?
    else {
        return Ok(None);
    };
    let envelope: ExternalPoolAdapterTaskEventPollEnvelope = envelope_at(&row, 3)?;
    validate_task_production_event_poll(&envelope)?;
    let claim = claim_projection_at(&row, 36)?;
    validate_claim_window(&claim, &envelope.poll.not_after)?;
    ensure_exact(&row, event_poll_values(&envelope, &claim)?, "event poll")?;
    Ok(Some(AuditedEventPoll { envelope, claim }))
}

pub(in crate::store) fn read_event_batch_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<ExternalPoolAdapterTaskEventBatchEnvelope>> {
    let Some(row) = row_by_id_on(
        conn,
        EVENT_BATCH_TABLE,
        "event_batch_id",
        id,
        &EVENT_BATCH_COLUMNS,
    )?
    else {
        return Ok(None);
    };
    let envelope: ExternalPoolAdapterTaskEventBatchEnvelope = envelope_at(&row, 3)?;
    validate_task_production_event_batch(&envelope)?;
    ensure_exact(&row, event_batch_values(&envelope)?, "event batch")?;
    Ok(Some(envelope))
}

pub(in crate::store) fn read_event_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<ExternalPoolAdapterTaskEventEnvelope>> {
    let Some(row) = row_by_id_on(conn, EVENT_TABLE, "event_id", id, &EVENT_COLUMNS)? else {
        return Ok(None);
    };
    let envelope: ExternalPoolAdapterTaskEventEnvelope = envelope_at(&row, 3)?;
    validate_task_production_event(&envelope)?;
    ensure_exact(&row, event_values(&envelope)?, "event")?;
    Ok(Some(envelope))
}

fn row_by_id_on(
    conn: &Connection,
    table: &str,
    id_column: &str,
    id: &str,
    columns: &[&str],
) -> Result<Option<Vec<Value>>> {
    let sql = format!(
        "SELECT {} FROM {table} WHERE {id_column}=?1",
        columns.join(",")
    );
    Ok(conn
        .query_row(&sql, params![id], |row| {
            (0..columns.len()).map(|index| row.get(index)).collect()
        })
        .optional()?)
}

fn envelope_at<T: serde::de::DeserializeOwned>(row: &[Value], index: usize) -> Result<T> {
    let json = text_at(row, index)?;
    ensure!(
        json.len() <= TASK_PRODUCTION_MAX_LEDGER_JSON_BYTES,
        "V273 durable envelope exceeds its bound"
    );
    Ok(serde_json::from_str(json)?)
}

fn claim_projection_at(row: &[Value], index: usize) -> Result<PollClaimProjection> {
    let claim = PollClaimProjection {
        status: text_at(row, index)?.to_string(),
        revision: integer_at(row, index + 1)?,
        generation: integer_at(row, index + 2)?,
        owner_id: optional_text_at(row, index + 3)?,
        token_digest: optional_text_at(row, index + 4)?,
        expires_at: optional_text_at(row, index + 5)?,
    };
    ensure!(
        claim.revision > 0
            && claim.revision <= TASK_PRODUCTION_MAX_SAFE_INTEGER
            && claim.generation <= TASK_PRODUCTION_MAX_SAFE_INTEGER,
        "V273 poll claim revision is invalid"
    );
    match claim.status.as_str() {
        CLAIM_STATUS_CLAIMED => {
            let owner = claim
                .owner_id
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("V273 claimed poll lacks owner"))?;
            ensure!(
                !owner.is_empty()
                    && owner.trim() == owner
                    && owner.chars().count() <= 240
                    && !owner.chars().any(char::is_control)
                    && claim.generation > 0,
                "V273 poll claim owner is invalid"
            );
            digest(
                claim
                    .token_digest
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("V273 claimed poll lacks token digest"))?,
            )?;
            canonical_nanos(
                claim
                    .expires_at
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("V273 claimed poll lacks expiry"))?,
            )?;
        }
        CLAIM_STATUS_PENDING
        | CLAIM_STATUS_IN_FLIGHT_UNKNOWN
        | CLAIM_STATUS_DELIVERY_OBSERVED
        | CLAIM_STATUS_QUARANTINED => ensure!(
            claim.owner_id.is_none() && claim.token_digest.is_none() && claim.expires_at.is_none(),
            "V273 unclaimed poll retains claim custody"
        ),
        _ => bail!("V273 poll claim status is invalid"),
    }
    if claim.generation == 0 {
        ensure!(
            claim.status == CLAIM_STATUS_PENDING && claim.revision == 1,
            "V273 unclaimed poll has impossible initial generation"
        );
    } else if claim.status != CLAIM_STATUS_PENDING {
        ensure!(
            claim.generation > 0,
            "V273 transitioned poll lacks claim generation"
        );
    }
    Ok(claim)
}

fn validate_claim_window(claim: &PollClaimProjection, not_after: &str) -> Result<()> {
    if let Some(expires_at) = claim.expires_at.as_deref() {
        ensure!(
            expires_at <= not_after,
            "V273 poll claim expiry exceeds intent window"
        );
    }
    Ok(())
}

fn ensure_exact(row: &[Value], expected: Vec<Value>, kind: &str) -> Result<()> {
    ensure!(
        row.len() == expected.len() && row == expected,
        "V273 {kind} full-column projection is not exact"
    );
    Ok(())
}

fn text_at(row: &[Value], index: usize) -> Result<&str> {
    match row.get(index) {
        Some(Value::Text(value)) => Ok(value),
        _ => bail!("V273 durable column {index} is not text"),
    }
}

fn optional_text_at(row: &[Value], index: usize) -> Result<Option<String>> {
    match row.get(index) {
        Some(Value::Null) => Ok(None),
        Some(Value::Text(value)) => Ok(Some(value.clone())),
        _ => bail!("V273 durable column {index} is not nullable text"),
    }
}

fn integer_at(row: &[Value], index: usize) -> Result<u64> {
    match row.get(index) {
        Some(Value::Integer(value)) => Ok(u64::try_from(*value)?),
        _ => bail!("V273 durable column {index} is not an integer"),
    }
}

fn digest(value: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
            && value.bytes().any(|byte| byte != b'0'),
        "V273 claim token digest is invalid"
    );
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<()> {
    ensure!(value.len() == 30, "V273 claim expiry is not UTC nanos");
    let parsed: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(value)?;
    ensure!(
        parsed.offset().local_minus_utc() == 0
            && parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) == value,
        "V273 claim expiry is not canonical UTC nanos"
    );
    Ok(())
}
