use anyhow::Result;
use rusqlite::Connection;

mod basis_guards;
mod immutability_guards;
mod projection_guards;
mod source_guards;
mod tables;

pub(super) fn install(conn: &Connection) -> Result<()> {
    tables::create(conn)?;
    projection_guards::install(conn)?;
    source_guards::install(conn)?;
    basis_guards::install(conn)?;
    immutability_guards::install(conn)?;
    Ok(())
}
