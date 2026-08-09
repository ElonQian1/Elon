use anyhow::Result;
use rusqlite::Connection;

mod credentials;
mod sessions;

pub(super) fn create(conn: &Connection) -> Result<()> {
    credentials::create(conn)?;
    sessions::create(conn)?;
    Ok(())
}
