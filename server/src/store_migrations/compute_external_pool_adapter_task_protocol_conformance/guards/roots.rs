use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("roots/release_security.sql"))?;
    conn.execute_batch(include_str!("roots/runtime_compatibility.sql"))?;
    Ok(())
}
