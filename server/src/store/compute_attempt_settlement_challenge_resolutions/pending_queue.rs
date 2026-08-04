use anyhow::Result;
use rusqlite::{params, Connection};

pub(super) fn list_open_challenge_lease_ids_on(
    conn: &Connection,
    consumer_user_id: Option<&str>,
    limit: usize,
) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT challenge.lease_id
           FROM compute_settlement_challenges challenge
           LEFT JOIN compute_settlement_challenge_resolutions resolution
             ON resolution.challenge_id=challenge.challenge_id
          WHERE resolution.resolution_id IS NULL
            AND (?1 IS NULL OR challenge.consumer_account_id=?1)
          ORDER BY challenge.opened_at ASC,
                   challenge.challenge_id ASC
          LIMIT ?2",
    )?;
    let rows = statement.query_map(params![consumer_user_id, limit as i64], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
