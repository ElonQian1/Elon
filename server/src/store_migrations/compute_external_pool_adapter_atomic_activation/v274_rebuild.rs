use anyhow::{bail, Result};
use rusqlite::Connection;

pub(super) fn rebuild_if_required(conn: &Connection) -> Result<()> {
    let columns = table_columns(
        conn,
        "compute_external_pool_adapter_provider_active_successor_receipts",
    )?;
    let exact = columns.len() == 85
        && columns
            .iter()
            .any(|column| column == "activation_target_updated_at")
        && columns.iter().any(|column| column == "evidence_checked_at")
        && !columns.iter().any(|column| column == "checked_at")
        && has_v277_witness_fk(conn)?;
    if exact {
        return Ok(());
    }
    let receipt_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM compute_external_pool_adapter_provider_active_successor_receipts",
        [],
        |row| row.get(0),
    )?;
    let revocation_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM compute_external_pool_adapter_provider_active_successor_revocations",
        [],
        |row| row.get(0),
    )?;
    if receipt_count != 0 || revocation_count != 0 {
        bail!("V277 refuses to rebuild non-empty V274 authority tables")
    }
    conn.execute_batch(
        "DROP VIEW IF EXISTS compute_external_pool_adapter_provider_active_successor_current;
         DROP TABLE compute_external_pool_adapter_provider_active_successor_revocations;
         DROP TABLE compute_external_pool_adapter_provider_active_successor_receipts;",
    )?;
    super::super::compute_external_pool_adapter_provider_active_successor::recreate_empty_schema_for_v277(conn)
}

fn has_v277_witness_fk(conn: &Connection) -> Result<bool> {
    let mut statement = conn.prepare(
        "PRAGMA foreign_key_list(compute_external_pool_adapter_provider_active_successor_receipts)",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;
    let entries = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entries.iter().any(|(id, table, from, to)| {
        table == "compute_external_pool_adapter_atomic_activation_receipts"
            && from == "activation_witness_id"
            && to == "activation_receipt_id"
            && entries
                .iter()
                .any(|(other_id, other_table, other_from, other_to)| {
                    other_id == id
                        && other_table == table
                        && other_from == "activation_witness_digest"
                        && other_to == "activation_receipt_digest"
                })
            && entries
                .iter()
                .any(|(other_id, other_table, other_from, other_to)| {
                    other_id == id
                        && other_table == table
                        && other_from == "activation_root_digest"
                        && other_to == "activation_root_digest"
                })
    }))
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
