use anyhow::{bail, Result};
use rusqlite::Connection;

mod dispatch_guard;
mod source_trigger;
mod tables;
mod triggers;

pub(super) fn migration_v212(conn: &Connection) -> Result<()> {
    let dispatch_exists = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM compute_attempt_dispatch_commands LIMIT 1)",
        [],
        |row| row.get::<_, bool>(0),
    )?;
    if dispatch_exists {
        bail!("COMPUTE_EXECUTION_PLAN_BACKFILL_REQUIRED");
    }

    tables::create(conn)?;
    triggers::install(conn)?;
    source_trigger::install(conn)?;
    dispatch_guard::install(conn)?;
    Ok(())
}
