use anyhow::Result;
use rusqlite::{params, Connection};

pub(super) fn list_pending_verification_lease_ids_on(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT candidate.lease_id
           FROM compute_attempt_terminal_candidates candidate
           JOIN compute_attempt_consumer_reviews consumer
             ON consumer.terminal_candidate_id = candidate.terminal_candidate_id
           JOIN compute_attempt_platform_observations observation
             ON observation.terminal_candidate_id = candidate.terminal_candidate_id
          WHERE NOT EXISTS (
                    SELECT 1
                      FROM compute_attempt_verification_decisions verification
                     WHERE verification.terminal_candidate_id = candidate.terminal_candidate_id
                )
          ORDER BY CASE
                       WHEN consumer.reviewed_at >= observation.observed_at
                       THEN consumer.reviewed_at
                       ELSE observation.observed_at
                   END ASC,
                   candidate.terminal_candidate_id ASC
          LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
