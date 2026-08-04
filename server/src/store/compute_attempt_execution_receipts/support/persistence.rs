use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::compute_federation::receipts::ComputeExecutionReceipt;

use super::StoredExecutionReceipt;

pub(super) fn execution_receipt_by_idempotency_on(
    conn: &Connection,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<Option<StoredExecutionReceipt>> {
    query_one(
        conn,
        "SELECT execution_receipt_id, verification_decision_id,
                verification_event_digest, lease_id, receipt_digest,
                receipt_json, request_digest, idempotency_scope,
                idempotency_key, issued_by_user_id, issued_at, created_at
           FROM compute_attempt_execution_receipts
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![idempotency_scope, idempotency_key],
    )
}

pub(super) fn execution_receipt_by_verification_on(
    conn: &Connection,
    verification_decision_id: &str,
) -> Result<Option<StoredExecutionReceipt>> {
    query_one(
        conn,
        "SELECT execution_receipt_id, verification_decision_id,
                verification_event_digest, lease_id, receipt_digest,
                receipt_json, request_digest, idempotency_scope,
                idempotency_key, issued_by_user_id, issued_at, created_at
           FROM compute_attempt_execution_receipts
          WHERE verification_decision_id=?1",
        params![verification_decision_id],
    )
}

pub(super) fn execution_receipt_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredExecutionReceipt>> {
    query_one(
        conn,
        "SELECT execution_receipt_id, verification_decision_id,
                verification_event_digest, lease_id, receipt_digest,
                receipt_json, request_digest, idempotency_scope,
                idempotency_key, issued_by_user_id, issued_at, created_at
           FROM compute_attempt_execution_receipts
          WHERE lease_id=?1",
        params![lease_id],
    )
}

fn query_one<P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> Result<Option<StoredExecutionReceipt>> {
    conn.query_row(sql, params, stored_from_row)
        .optional()
        .map_err(Into::into)
}

fn stored_from_row(row: &Row<'_>) -> rusqlite::Result<StoredExecutionReceipt> {
    let receipt_json: String = row.get(5)?;
    Ok(StoredExecutionReceipt {
        execution_receipt_id: row.get(0)?,
        verification_decision_id: row.get(1)?,
        verification_event_digest: row.get(2)?,
        lease_id: row.get(3)?,
        receipt_digest: row.get(4)?,
        receipt: serde_json::from_str::<ComputeExecutionReceipt>(&receipt_json)
            .map_err(json_error)?,
        request_digest: row.get(6)?,
        idempotency_scope: row.get(7)?,
        idempotency_key: row.get(8)?,
        issued_by_user_id: row.get(9)?,
        issued_at: row.get(10)?,
        created_at: row.get(11)?,
    })
}

fn json_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
