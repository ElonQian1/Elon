use anyhow::Result;
use rusqlite::{params, Connection};

pub(super) fn list_pending_settlement_lease_ids_on(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT finalization.lease_id
           FROM compute_attempt_finalizations finalization
          WHERE NOT EXISTS (
                    SELECT 1
                      FROM compute_attempt_settlements settlement
                     WHERE settlement.lease_id = finalization.lease_id
                )
          ORDER BY finalization.finalized_at ASC,
                   finalization.finalization_id ASC
          LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
