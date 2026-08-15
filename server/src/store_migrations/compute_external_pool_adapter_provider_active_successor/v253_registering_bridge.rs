use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("v253/view.sql"))?;
    conn.execute_batch(include_str!("v253/challenge_roots.sql"))?;
    conn.execute_batch(include_str!("v253/receipt_current_roots.sql"))?;
    Ok(())
}
