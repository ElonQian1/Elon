use anyhow::Result;
use rusqlite::Connection;

/// Additive and atomic; does not convert Paper users, grants, wallets or balances.
pub(crate) fn migration_v292(conn: &Connection) -> Result<()> {
    conn.execute_batch("SAVEPOINT game_access_v292")?;
    if let Err(error) = conn.execute_batch(include_str!("schema.sql")) {
        conn.execute_batch("ROLLBACK TO game_access_v292; RELEASE game_access_v292")?;
        return Err(error.into());
    }
    conn.execute_batch("RELEASE game_access_v292")?;
    Ok(())
}
