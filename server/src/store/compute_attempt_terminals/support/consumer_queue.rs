use anyhow::Result;
use rusqlite::{params, Connection};

use super::{select_sql, stored_from_row, StoredTerminalCandidate};

pub(super) fn list_pending_consumer_review_candidates_on(
    conn: &Connection,
    consumer_account_id: &str,
    limit: usize,
) -> Result<Vec<StoredTerminalCandidate>> {
    let mut statement = conn.prepare(&format!(
        "{} WHERE consumer_account_id=?1
              AND NOT EXISTS (
                  SELECT 1
                    FROM compute_attempt_consumer_reviews review
                   WHERE review.terminal_candidate_id =
                         compute_attempt_terminal_candidates.terminal_candidate_id
              )
            ORDER BY declared_at ASC, terminal_candidate_id ASC
            LIMIT ?2",
        select_sql()
    ))?;
    let rows = statement.query_map(params![consumer_account_id, limit as i64], stored_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
