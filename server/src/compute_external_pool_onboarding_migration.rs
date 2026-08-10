use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod projection_triggers;
mod source_trigger;
mod state_guards;
mod tables;

pub(crate) fn migration_v221(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    projection_triggers::install(&transaction)?;
    state_guards::install(&transaction)?;
    source_trigger::replace(&transaction)?;
    transaction.commit()?;
    Ok(())
}
