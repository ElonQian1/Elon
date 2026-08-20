use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(connection: &Connection) -> Result<()> {
    connection.execute_batch(include_str!("tables/receipts.sql"))?;
    Ok(())
}
