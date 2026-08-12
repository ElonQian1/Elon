use anyhow::Result;
use rusqlite::Connection;

mod exact_roots;
mod immutability;
mod projection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    immutability::install(conn)?;
    exact_roots::install(conn)?;
    projection::install(conn)?;
    Ok(())
}

pub(super) fn repair_release_exact_roots(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DROP TRIGGER IF EXISTS external_pool_adapter_registry_release_exact_roots;",
    )?;
    exact_roots::install(conn)
}
