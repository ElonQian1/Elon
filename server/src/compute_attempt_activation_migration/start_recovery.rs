use anyhow::Result;
use rusqlite::Connection;

mod ack_guard;
mod backfill;
mod proof_guard;
mod send_guard;
mod source_guard;

pub(super) fn migration_v214(conn: &Connection) -> Result<()> {
    backfill::ensure_no_unsafe_backfill(conn)?;
    source_guard::install(conn)?;
    ack_guard::install(conn)?;
    send_guard::install(conn)?;
    proof_guard::install(conn)?;
    Ok(())
}
