use anyhow::Result;
use rusqlite::Connection;

mod fences;
mod immutability;
mod lineage;
mod projection;
mod roots;

pub(super) fn install(conn: &Connection) -> Result<()> {
    immutability::install(conn)?;
    projection::install(conn)?;
    roots::install(conn)?;
    lineage::install(conn)?;
    fences::install(conn)?;
    Ok(())
}
