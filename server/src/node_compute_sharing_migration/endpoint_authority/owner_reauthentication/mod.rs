use anyhow::Result;
use rusqlite::Connection;

mod guards;
mod tables;

pub(super) fn install(conn: &Connection) -> Result<()> {
    tables::create(conn)?;
    guards::install(conn)?;
    Ok(())
}
