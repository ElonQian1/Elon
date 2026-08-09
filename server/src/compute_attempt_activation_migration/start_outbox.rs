use anyhow::{bail, Result};
use rusqlite::Connection;

mod authority_tables;
mod command_triggers;
mod no_start_triggers;
mod observation_triggers;
mod outbox_tables;
mod replacement_guards;
mod route_tables;

pub(super) fn migration_v213(conn: &Connection) -> Result<()> {
    let requires_backfill = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM compute_attempt_dispatch_commands LIMIT 1)
             OR EXISTS(SELECT 1 FROM compute_attempt_dispatch_applications LIMIT 1)",
        [],
        |row| row.get::<_, bool>(0),
    )?;
    if requires_backfill {
        bail!("COMPUTE_START_OUTBOX_BACKFILL_REQUIRED");
    }

    route_tables::create(conn)?;
    authority_tables::create(conn)?;
    outbox_tables::create(conn)?;
    command_triggers::install(conn)?;
    observation_triggers::install(conn)?;
    no_start_triggers::install(conn)?;
    replacement_guards::install(conn)?;
    Ok(())
}
