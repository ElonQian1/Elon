use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("fences/replace_pending_plan_fences.sql"))?;
    Ok(())
}
