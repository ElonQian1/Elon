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
