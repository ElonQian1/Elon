use anyhow::Result;
use rusqlite::{params, Connection};

pub(super) fn list_pending_challenge_lease_ids_on(
    conn: &Connection,
    consumer_user_id: &str,
    cutoff: &str,
    as_of: &str,
    limit: usize,
) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT settlement.lease_id
           FROM compute_attempt_settlements settlement
           JOIN compute_settlement_postings posting
             ON posting.settlement_receipt_id=settlement.settlement_receipt_id
           JOIN compute_settlement_ledger_legs consumer_leg
             ON consumer_leg.posting_id=posting.posting_id
            AND consumer_leg.leg_kind='consumer_capture'
          WHERE consumer_leg.account_id=?1
            AND settlement.settled_at>=?2
            AND settlement.settled_at<=?3
            AND NOT EXISTS (
                  SELECT 1
                    FROM compute_settlement_challenges challenge
                   WHERE challenge.settlement_receipt_id=settlement.settlement_receipt_id
                )
            AND NOT EXISTS (
                  SELECT 1
                    FROM compute_settlement_releases release
                   WHERE release.settlement_receipt_id=settlement.settlement_receipt_id
                )
          ORDER BY settlement.settled_at ASC,
                   settlement.settlement_receipt_id ASC
          LIMIT ?4",
    )?;
    let rows = statement.query_map(
        params![consumer_user_id, cutoff, as_of, limit as i64],
        |row| row.get(0),
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
