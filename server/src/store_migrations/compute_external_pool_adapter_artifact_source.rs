use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod tables;
mod triggers;

pub(crate) fn migration_v227(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    triggers::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}
