use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("tables/receipts.sql"))?;
    conn.execute_batch(include_str!("tables/revocations.sql"))?;
    conn.execute_batch(include_str!("tables/indexes.sql"))?;
    Ok(())
}
