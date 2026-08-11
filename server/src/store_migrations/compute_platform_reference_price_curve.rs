use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod application_projection;
mod application_tables;
mod proposal_projection;
mod proposal_tables;
mod snapshot_source_guard;
mod state_source_guards;

pub(crate) fn migration_v223(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    snapshot_source_guard::reject_legacy_reserved_sources(&transaction)?;
    proposal_tables::create(&transaction)?;
    application_tables::create(&transaction)?;
    proposal_projection::install(&transaction)?;
    application_projection::install(&transaction)?;
    state_source_guards::install(&transaction)?;
    snapshot_source_guard::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}
