use anyhow::Result;
use rusqlite::Connection;

mod actor_guard;
mod backfill;
mod cleanup_guard;
mod deadline_guard;

pub(super) fn migration_v215(conn: &Connection) -> Result<()> {
    backfill::ensure_no_unsafe_backfill(conn)?;
    cleanup_guard::install(conn)?;
    actor_guard::install(conn)?;
    deadline_guard::install(conn)?;
    Ok(())
}
