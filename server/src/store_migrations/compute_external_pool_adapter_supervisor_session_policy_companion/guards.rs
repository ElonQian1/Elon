use anyhow::Result;
use rusqlite::Connection;

mod immutability;
mod lineage;
pub(super) mod policy_projection;
mod receipt_projection;
mod roots;
mod timestamp;

pub(super) fn install(conn: &Connection) -> Result<()> {
    immutability::install(conn)?;
    super::receipt_integrity::install(conn)?;
    receipt_projection::install(conn)?;
    policy_projection::install(conn)?;
    timestamp::install(conn)?;
    roots::install(conn)?;
    lineage::install(conn)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn receipt_projection_counts() -> (usize, usize) {
    receipt_projection::counts()
}
