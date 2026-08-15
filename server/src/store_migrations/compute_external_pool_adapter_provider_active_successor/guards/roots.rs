use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("roots/structural.sql"))?;
    conn.execute_batch(include_str!("roots/provider_credential.sql"))?;
    conn.execute_batch(include_str!("roots/task_protocol.sql"))?;
    Ok(())
}
