use anyhow::Result;
use rusqlite::Connection;

mod hostname;
mod immutability;
mod lineage;
mod policy_projection;
mod receipt_projection;
mod roots;
mod timestamp;

pub(super) fn install(conn: &Connection) -> Result<()> {
    immutability::install(conn)?;
    receipt_projection::install(conn)?;
    policy_projection::install(conn)?;
    hostname::install(conn)?;
    timestamp::install(conn)?;
    roots::install(conn)?;
    lineage::install(conn)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn receipt_projection_counts() -> (usize, usize) {
    receipt_projection::counts()
}

#[cfg(test)]
pub(super) fn policy_projection_count() -> usize {
    policy_projection::count()
}
