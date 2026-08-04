use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v184(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_compute_reservations_offer_status
           ON compute_reservations(offer_id, status, updated_at, reservation_id);",
    )?;
    Ok(())
}
