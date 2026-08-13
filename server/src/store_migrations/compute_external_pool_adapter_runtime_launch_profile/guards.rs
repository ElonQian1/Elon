use anyhow::Result;
use rusqlite::Connection;

mod immutability;
mod lineage;
mod policy_projection;
mod profile_projection;
mod roots;

pub(super) fn install(conn: &Connection) -> Result<()> {
    immutability::install(conn)?;
    profile_projection::install(conn)?;
    policy_projection::install(conn)?;
    roots::install(conn)?;
    lineage::install(conn)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn profile_projection_counts() -> (usize, usize) {
    profile_projection::counts()
}

#[cfg(test)]
pub(super) fn policy_projection_count() -> usize {
    policy_projection::count()
}
