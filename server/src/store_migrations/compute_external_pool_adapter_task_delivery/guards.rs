use anyhow::Result;
use rusqlite::Connection;

mod event_lineage;
mod projection;
mod reachability;
mod route_authority;
mod source_lineage;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("guards/immutability.sql"))?;
    conn.execute_batch(include_str!("guards/no_replace.sql"))?;
    super::receipt_integrity::install(conn)?;
    projection::install(conn)?;
    source_lineage::install(conn)?;
    route_authority::install(conn)?;
    event_lineage::install(conn)?;
    conn.execute_batch(include_str!("guards/poll_claims.sql"))?;
    Ok(())
}

pub(super) fn install_v278_reachability(conn: &Connection) -> Result<()> {
    reachability::install(conn)
}
