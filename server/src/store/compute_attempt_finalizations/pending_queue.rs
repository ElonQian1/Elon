use anyhow::Result;
use rusqlite::{params, Connection};

pub(super) fn list_pending_finalization_lease_ids_on(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT receipt.lease_id
           FROM compute_attempt_execution_receipts receipt
          WHERE NOT EXISTS (
                    SELECT 1
                      FROM compute_attempt_finalizations finalization
                     WHERE finalization.lease_id = receipt.lease_id
                )
          ORDER BY receipt.issued_at ASC,
                   receipt.execution_receipt_id ASC
          LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
