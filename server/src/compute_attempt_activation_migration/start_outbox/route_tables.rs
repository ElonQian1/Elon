use anyhow::Result;
use rusqlite::Connection;

mod adapter;
mod authorization;
mod credential;
mod triggers;

pub(super) fn create(conn: &Connection) -> Result<()> {
    adapter::create(conn)?;
    credential::create(conn)?;
    authorization::create(conn)?;
    triggers::install(conn)?;
    Ok(())
}
