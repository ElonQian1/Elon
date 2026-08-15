use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_task_protocol_conformance::*;

use super::{audit::*, types::*};

pub(super) fn run_by_id_on(
    conn: &Connection,
    run_receipt_id: &str,
) -> Result<Option<StoredTaskProtocolConformanceRun>> {
    run_on(conn, "run_receipt_id=?1", params![run_receipt_id])
}

pub(super) fn run_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredTaskProtocolConformanceRun>> {
    run_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn run_head_by_release_on(
    conn: &Connection,
    registry_release_id: &str,
) -> Result<Option<StoredTaskProtocolConformanceRun>> {
    run_on(
        conn,
        "registry_release_id=?1 ORDER BY sequence DESC LIMIT 1",
        params![registry_release_id],
    )
}

fn run_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredTaskProtocolConformanceRun>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT run_receipt_json,recorded_by_admin_user_id,idempotency_scope,
                        idempotency_key,confirmation,runtime_custody_epoch_digest,
                        process_hmac_seal,receipt_integrity_digest
                   FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts
                  WHERE {filter}"
            ),
            values,
            |row| {
                let (receipt, receipt_json) = decode(row, 0)?;
                Ok(StoredTaskProtocolConformanceRun {
                    receipt,
                    receipt_json,
                    recorded_by_admin_user_id: row.get(1)?,
                    idempotency_scope: row.get(2)?,
                    idempotency_key: row.get(3)?,
                    confirmation: row.get(4)?,
                    runtime_custody_epoch_digest: row.get(5)?,
                    process_hmac_seal: row.get(6)?,
                    receipt_integrity_digest: row.get(7)?,
                })
            },
        )
        .optional()?;
    stored.map(|value| audit_run(conn, value)).transpose()
}

pub(super) fn revocation_by_run_on(
    conn: &Connection,
    run_receipt_id: &str,
) -> Result<Option<StoredTaskProtocolConformanceRevocation>> {
    revocation_on(conn, "run_receipt_id=?1", params![run_receipt_id])
}

pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredTaskProtocolConformanceRevocation>> {
    revocation_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn revocation_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredTaskProtocolConformanceRevocation>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT revocation_receipt_json,revoked_by_admin_user_id,idempotency_scope,
                        idempotency_key,confirmation
                   FROM compute_external_pool_adapter_task_protocol_conformance_revocations
                  WHERE {filter}"
            ),
            values,
            |row| {
                let (receipt, receipt_json) = decode(row, 0)?;
                Ok(StoredTaskProtocolConformanceRevocation {
                    receipt,
                    receipt_json,
                    revoked_by_admin_user_id: row.get(1)?,
                    idempotency_scope: row.get(2)?,
                    idempotency_key: row.get(3)?,
                    confirmation: row.get(4)?,
                })
            },
        )
        .optional()?;
    stored
        .map(|value| audit_revocation(conn, value))
        .transpose()
}

fn decode<T: serde::de::DeserializeOwned>(
    row: &rusqlite::Row<'_>,
    index: usize,
) -> rusqlite::Result<(T, String)> {
    let json: String = row.get(index)?;
    if json.len() > TASK_PROTOCOL_CONFORMANCE_MAX_RECEIPT_JSON_BYTES {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            index,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "task-protocol conformance receipt exceeds its durable bound",
            )),
        ));
    }
    let receipt = serde_json::from_str(&json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })?;
    Ok((receipt, json))
}

pub(super) fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("task-protocol conformance identifier is invalid")
    }
    Ok(())
}
