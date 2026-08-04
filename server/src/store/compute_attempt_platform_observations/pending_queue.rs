use anyhow::Result;
use rusqlite::{params, Connection};

pub(super) fn list_pending_platform_observation_lease_ids_on(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT candidate.lease_id
           FROM compute_attempt_terminal_candidates candidate
          WHERE NOT EXISTS (
                    SELECT 1
                      FROM compute_attempt_platform_observations observation
                     WHERE observation.terminal_candidate_id = candidate.terminal_candidate_id
                )
          ORDER BY candidate.declared_at ASC, candidate.terminal_candidate_id ASC
          LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
