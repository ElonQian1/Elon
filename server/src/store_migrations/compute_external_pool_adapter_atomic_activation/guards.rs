use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("guards/projection.sql"))?;
    conn.execute_batch(include_str!("guards/roots.sql"))?;
    conn.execute_batch(include_str!("guards/immutability.sql"))?;
    Ok(())
}
