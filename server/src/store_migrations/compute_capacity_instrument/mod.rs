use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod downstream_guards;
mod guards;
mod historical_exercise;
mod tables;

pub(crate) fn migration_v238(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    guards::install(&transaction)?;
    historical_exercise::create(&transaction)?;
    downstream_guards::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}
