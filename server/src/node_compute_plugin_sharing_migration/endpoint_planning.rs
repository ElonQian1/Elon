//! Append-only endpoint-session provenance for the inert six-message Planning bootstrap.

use anyhow::Result;
use rusqlite::Connection;

mod append_only;
mod guards;
mod table;

pub(super) fn migration_v219(conn: &Connection) -> Result<()> {
    table::create(conn)?;
    guards::install(conn)?;
    append_only::install(conn)?;
    Ok(())
}
