use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{ComputeAttemptFinalizationReceipt, FinalizeComputeAttemptRequest};

mod audit;

pub(super) struct StoredFinalization {
    pub finalization_id: String,
    pub lease_id: String,
    pub execution_receipt_id: String,
    pub execution_receipt_digest: String,
    pub request: FinalizeComputeAttemptRequest,
    pub request_digest: String,
    pub receipt: ComputeAttemptFinalizationReceipt,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub finalized_by_user_id: String,
    pub effective_at: String,
    pub finalized_at: String,
    pub created_at: String,
}

pub(super) fn normalize_finalization_request(
    input: &FinalizeComputeAttemptRequest,
) -> Result<FinalizeComputeAttemptRequest> {
    for (label, value, max_len) in [
        ("Attempt Lease ID", input.lease_id.as_str(), 200),
        (
            "Execution Receipt ID",
            input.expected_execution_receipt_id.as_str(),
            200,
        ),
        (
            "Execution Receipt 摘要",
            input.expected_execution_receipt_digest.as_str(),
            64,
        ),
        ("Lease 摘要", input.expected_lease_digest.as_str(), 64),
        ("Job 摘要", input.expected_job_digest.as_str(), 64),
        (
            "Reservation 摘要",
            input.expected_reservation_digest.as_str(),
            64,
        ),
        (
            "Capacity Claim 摘要",
            input.expected_claim_digest.as_str(),
            64,
        ),
        ("幂等键", input.idempotency_key.as_str(), 200),
        ("可信终态执行用户", input.finalized_by_user_id.as_str(), 200),
    ] {
        validate_exact(label, value, max_len)?;
    }
    for (label, digest) in [
        (
            "Execution Receipt 摘要",
            input.expected_execution_receipt_digest.as_str(),
        ),
        ("Lease 摘要", input.expected_lease_digest.as_str()),
        ("Job 摘要", input.expected_job_digest.as_str()),
        (
            "Reservation 摘要",
            input.expected_reservation_digest.as_str(),
        ),
        ("Capacity Claim 摘要", input.expected_claim_digest.as_str()),
    ] {
        validate_digest(label, digest)?;
    }
    for (label, value) in [
        ("expected_lease_revision", input.expected_lease_revision),
        (
            "expected_fencing_generation",
            input.expected_fencing_generation,
        ),
        ("expected_job_revision", input.expected_job_revision),
        (
            "expected_reservation_revision",
            input.expected_reservation_revision,
        ),
        ("expected_claim_revision", input.expected_claim_revision),
    ] {
        if value <= 0 {
            bail!("{label} 必须为正整数");
        }
    }

    let mut normalized = input.clone();
    normalized.lease_id = normalized.lease_id.trim().to_string();
    normalized.expected_execution_receipt_id =
        normalized.expected_execution_receipt_id.trim().to_string();
    normalized.expected_execution_receipt_digest = normalized
        .expected_execution_receipt_digest
        .trim()
        .to_ascii_lowercase();
    normalized.expected_lease_digest = normalized.expected_lease_digest.trim().to_ascii_lowercase();
    normalized.expected_job_digest = normalized.expected_job_digest.trim().to_ascii_lowercase();
    normalized.expected_reservation_digest = normalized
        .expected_reservation_digest
        .trim()
        .to_ascii_lowercase();
    normalized.expected_claim_digest = normalized.expected_claim_digest.trim().to_ascii_lowercase();
    normalized.idempotency_key = normalized.idempotency_key.trim().to_string();
    normalized.finalized_by_user_id = normalized.finalized_by_user_id.trim().to_string();
    Ok(normalized)
}

pub(super) fn finalization_request_digest(input: &FinalizeComputeAttemptRequest) -> Result<String> {
    sha256_json(&serde_json::json!({
        "purpose": "compute_attempt_finalization_request",
        "request": input,
    }))
}

pub(super) fn finalization_event_digest(
    receipt: &ComputeAttemptFinalizationReceipt,
) -> Result<String> {
    let mut payload = receipt.clone();
    payload.event_digest.clear();
    payload.replayed = false;
    sha256_json(&serde_json::json!({
        "purpose": "compute_attempt_finalization_event",
        "receipt": payload,
    }))
}

pub(super) fn persist_finalization_on(
    conn: &Connection,
    request: &FinalizeComputeAttemptRequest,
    receipt: &ComputeAttemptFinalizationReceipt,
    idempotency_scope: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_attempt_finalizations (
            finalization_id, lease_id, execution_receipt_id,
            execution_receipt_digest, request_json, request_digest,
            receipt_json, event_digest, idempotency_scope, idempotency_key,
            finalized_by_user_id, effective_at, finalized_at, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
        params![
            receipt.finalization_id,
            receipt.lease_id,
            receipt.execution_receipt_id,
            receipt.execution_receipt_digest,
            serde_json::to_string(request)?,
            receipt.request_digest,
            serde_json::to_string(receipt)?,
            receipt.event_digest,
            idempotency_scope,
            request.idempotency_key,
            request.finalized_by_user_id,
            receipt.effective_at,
            receipt.finalized_at,
        ],
    )?;
    Ok(())
}

pub(super) fn finalization_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredFinalization>> {
    finalization_query(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        scope,
        key,
    )
}

pub(super) fn finalization_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredFinalization>> {
    finalization_query(conn, "lease_id=?1", lease_id, "")
}

fn finalization_query(
    conn: &Connection,
    predicate: &str,
    first: &str,
    second: &str,
) -> Result<Option<StoredFinalization>> {
    let sql = format!(
        "SELECT finalization_id, lease_id, execution_receipt_id,
                execution_receipt_digest, request_json, request_digest,
                receipt_json, event_digest, idempotency_scope, idempotency_key,
                finalized_by_user_id, effective_at, finalized_at, created_at
           FROM compute_attempt_finalizations WHERE {predicate}"
    );
    let mut statement = conn.prepare(&sql)?;
    let result = if second.is_empty() {
        statement
            .query_row(params![first], stored_finalization_from_row)
            .optional()?
    } else {
        statement
            .query_row(params![first, second], stored_finalization_from_row)
            .optional()?
    };
    Ok(result)
}

fn stored_finalization_from_row(row: &Row<'_>) -> rusqlite::Result<StoredFinalization> {
    let request_json: String = row.get(4)?;
    let receipt_json: String = row.get(6)?;
    Ok(StoredFinalization {
        finalization_id: row.get(0)?,
        lease_id: row.get(1)?,
        execution_receipt_id: row.get(2)?,
        execution_receipt_digest: row.get(3)?,
        request: serde_json::from_str(&request_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                request_json.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        request_digest: row.get(5)?,
        receipt: serde_json::from_str(&receipt_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                receipt_json.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        event_digest: row.get(7)?,
        idempotency_scope: row.get(8)?,
        idempotency_key: row.get(9)?,
        finalized_by_user_id: row.get(10)?,
        effective_at: row.get(11)?,
        finalized_at: row.get(12)?,
        created_at: row.get(13)?,
    })
}

pub(super) fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max_len || trimmed.chars().any(char::is_control) {
        bail!("{label} 不能为空且长度不能超过 {max_len}");
    }
    if trimmed != value {
        bail!("{label} 不能包含首尾空白");
    }
    Ok(())
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label} 必须是 64 位十六进制摘要");
    }
    Ok(())
}

fn sha256_json(value: &impl Serialize) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value)?)))
}
