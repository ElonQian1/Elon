use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::compute_federation::receipts::ComputeSettlementReceipt;

use super::{ComputeAttemptSettlementReceipt, SettleComputeAttemptRequest};

mod audit;

#[derive(Debug, Clone)]
pub(super) struct StoredSettlement {
    pub settlement_receipt_id: String,
    pub lease_id: String,
    pub finalization_id: String,
    pub finalization_event_digest: String,
    pub execution_receipt_id: String,
    pub execution_receipt_digest: String,
    pub budget_reservation_id: String,
    pub price_snapshot_id: String,
    pub price_snapshot_digest: String,
    pub job_id: String,
    pub source_job_revision: i64,
    pub source_job_digest: String,
    pub terminal_job_revision: i64,
    pub terminal_job_digest: String,
    pub request_json: String,
    pub request_digest: String,
    pub receipt_json: String,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub settled_by_user_id: String,
    pub settled_at: String,
}

impl StoredSettlement {
    pub(super) fn into_receipt(
        &self,
        conn: &Connection,
        replayed: bool,
    ) -> Result<ComputeAttemptSettlementReceipt> {
        audit::audited_settlement_on(conn, self, replayed)
    }
}

pub(super) fn normalize_settlement_request(
    input: &SettleComputeAttemptRequest,
) -> Result<SettleComputeAttemptRequest> {
    let mut normalized = input.clone();
    for (label, value) in [
        ("Attempt Lease ID", &mut normalized.lease_id),
        ("可信终态 ID", &mut normalized.expected_finalization_id),
        (
            "Execution Receipt ID",
            &mut normalized.expected_execution_receipt_id,
        ),
        (
            "预算预授权 ID",
            &mut normalized.expected_budget_reservation_id,
        ),
        ("价格快照 ID", &mut normalized.expected_price_snapshot_id),
        ("幂等键", &mut normalized.idempotency_key),
        ("结算管理员 ID", &mut normalized.settled_by_user_id),
    ] {
        *value = value.trim().to_string();
        validate_exact(label, value, 240)?;
    }
    for (label, value) in [
        (
            "可信终态摘要",
            &mut normalized.expected_finalization_event_digest,
        ),
        (
            "Execution Receipt 摘要",
            &mut normalized.expected_execution_receipt_digest,
        ),
        ("Job 摘要", &mut normalized.expected_job_digest),
        (
            "价格快照摘要",
            &mut normalized.expected_price_snapshot_digest,
        ),
    ] {
        *value = value.trim().to_ascii_lowercase();
        validate_digest(label, value)?;
    }
    if normalized.expected_job_revision <= 0 {
        bail!("Attempt 结算 expected_job_revision 必须为正数");
    }
    Ok(normalized)
}

pub(super) fn settlement_request_digest(input: &SettleComputeAttemptRequest) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(input)?)))
}

pub(super) fn compute_settlement_receipt_digest(
    receipt: &ComputeSettlementReceipt,
) -> Result<String> {
    let mut canonical = receipt.clone();
    canonical.settlement_receipt_digest.clear();
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}

pub(super) fn attempt_settlement_event_digest(
    receipt: &ComputeAttemptSettlementReceipt,
) -> Result<String> {
    let mut canonical = receipt.clone();
    canonical.event_digest.clear();
    canonical.replayed = false;
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}

pub(super) fn persist_settlement_on(
    conn: &Connection,
    request: &SettleComputeAttemptRequest,
    receipt: &ComputeAttemptSettlementReceipt,
    idempotency_scope: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_attempt_settlements (
           settlement_receipt_id, lease_id, finalization_id,
           finalization_event_digest, execution_receipt_id,
           execution_receipt_digest, budget_reservation_id,
           price_snapshot_id, price_snapshot_digest, job_id,
           source_job_revision, source_job_digest,
           terminal_job_revision, terminal_job_digest,
           request_json, request_digest, receipt_json, event_digest,
           idempotency_scope, idempotency_key, settled_by_user_id,
           settled_at, created_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?22)",
        params![
            receipt.settlement.settlement_receipt_id,
            receipt.lease_id,
            receipt.finalization_id,
            receipt.finalization_event_digest,
            receipt.settlement.execution_receipt_id,
            receipt.settlement.execution_receipt_digest,
            receipt.budget_reservation_id,
            receipt.settlement.price_snapshot_id,
            receipt.settlement.price_snapshot_digest,
            receipt.source_job.job_id,
            receipt.source_job.job_revision,
            receipt.source_job.job_digest,
            receipt.terminal_job.job_revision,
            receipt.terminal_job.job_digest,
            serde_json::to_string(request)?,
            receipt.request_digest,
            serde_json::to_string(receipt)?,
            receipt.event_digest,
            idempotency_scope,
            request.idempotency_key,
            receipt.settled_by_user_id,
            receipt.settled_at,
        ],
    )?;
    Ok(())
}

pub(super) fn settlement_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredSettlement>> {
    settlement_query(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn settlement_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredSettlement>> {
    settlement_query(conn, "WHERE lease_id=?1", params![lease_id])
}

fn settlement_query<P>(
    conn: &Connection,
    where_clause: &str,
    values: P,
) -> Result<Option<StoredSettlement>>
where
    P: rusqlite::Params,
{
    conn.query_row(
        &format!(
            "SELECT settlement_receipt_id, lease_id, finalization_id,
                    finalization_event_digest, execution_receipt_id,
                    execution_receipt_digest, budget_reservation_id,
                    price_snapshot_id, price_snapshot_digest, job_id,
                    source_job_revision, source_job_digest,
                    terminal_job_revision, terminal_job_digest,
                    request_json, request_digest, receipt_json, event_digest,
                    idempotency_scope, idempotency_key, settled_by_user_id, settled_at
               FROM compute_attempt_settlements {where_clause}"
        ),
        values,
        stored_settlement_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_settlement_from_row(row: &Row<'_>) -> rusqlite::Result<StoredSettlement> {
    Ok(StoredSettlement {
        settlement_receipt_id: row.get(0)?,
        lease_id: row.get(1)?,
        finalization_id: row.get(2)?,
        finalization_event_digest: row.get(3)?,
        execution_receipt_id: row.get(4)?,
        execution_receipt_digest: row.get(5)?,
        budget_reservation_id: row.get(6)?,
        price_snapshot_id: row.get(7)?,
        price_snapshot_digest: row.get(8)?,
        job_id: row.get(9)?,
        source_job_revision: row.get(10)?,
        source_job_digest: row.get(11)?,
        terminal_job_revision: row.get(12)?,
        terminal_job_digest: row.get(13)?,
        request_json: row.get(14)?,
        request_digest: row.get(15)?,
        receipt_json: row.get(16)?,
        event_digest: row.get(17)?,
        idempotency_scope: row.get(18)?,
        idempotency_key: row.get(19)?,
        settled_by_user_id: row.get(20)?,
        settled_at: row.get(21)?,
    })
}

pub(super) fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > max_len
        || value != value.trim()
        || value.chars().any(char::is_control)
    {
        bail!("{label}无效");
    }
    Ok(())
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    validate_exact(label, value, 64)?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label}必须是 64 位十六进制摘要");
    }
    Ok(())
}
