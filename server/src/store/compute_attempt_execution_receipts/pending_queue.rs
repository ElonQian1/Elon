use anyhow::Result;
use rusqlite::{params, Connection};

pub(super) fn list_pending_execution_receipt_lease_ids_on(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT verification.lease_id
           FROM compute_attempt_verification_decisions verification
          WHERE verification.decision = 'accepted'
            AND NOT EXISTS (
                    SELECT 1
                      FROM compute_attempt_execution_receipts receipt
                     WHERE receipt.verification_decision_id = verification.verification_decision_id
                )
          ORDER BY verification.decided_at ASC,
                   verification.verification_decision_id ASC
          LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
