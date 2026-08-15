use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    for sql in [
        include_str!("tables/exchange_attempts.sql"),
        include_str!("tables/exchange_receipts.sql"),
        include_str!("tables/polls.sql"),
        include_str!("tables/events.sql"),
        include_str!("tables/indexes.sql"),
    ] {
        conn.execute_batch(sql)?;
    }
    Ok(())
}
