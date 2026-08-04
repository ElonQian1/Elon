use anyhow::Result;
use rusqlite::{params, Connection};

pub(super) fn list_pending_correction_lease_ids_on(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT resolution.lease_id
           FROM compute_settlement_challenge_resolutions resolution
          WHERE resolution.action='accepted'
            AND NOT EXISTS (
                  SELECT 1
                    FROM compute_settlement_corrections correction
                   WHERE correction.resolution_id=resolution.resolution_id
                )
            AND NOT EXISTS (
                  SELECT 1
                    FROM compute_settlement_releases release
                   WHERE release.settlement_receipt_id=resolution.settlement_receipt_id
                )
          ORDER BY resolution.resolved_at ASC,
                   resolution.resolution_id ASC
          LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
