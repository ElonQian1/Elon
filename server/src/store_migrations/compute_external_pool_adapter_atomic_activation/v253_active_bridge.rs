use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("v253_active_bridge/challenge_roots.sql"))?;
    conn.execute_batch(include_str!("v253_active_bridge/receipt_current_roots.sql"))?;
    conn.execute_batch(include_str!("v253_active_bridge/current_view.sql"))?;
    Ok(())
}
