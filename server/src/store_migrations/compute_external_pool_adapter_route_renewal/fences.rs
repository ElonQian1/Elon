use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(connection: &Connection) -> Result<()> {
    connection.execute_batch(include_str!("fences/v254_route_union.sql"))?;
    Ok(())
}
