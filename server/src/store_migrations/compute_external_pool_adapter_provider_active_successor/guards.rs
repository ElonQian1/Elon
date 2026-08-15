use anyhow::Result;
use rusqlite::Connection;

mod roots;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("guards/immutability.sql"))?;
    conn.execute_batch(include_str!("guards/no_replace.sql"))?;
    super::receipt_integrity::install(conn)?;
    conn.execute_batch(include_str!("guards/projection.sql"))?;
    conn.execute_batch(include_str!("guards/lineage.sql"))?;
    roots::install(conn)?;
    Ok(())
}
