//! V283 immutable interactive-desktop authority records and one current head per Session.

use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod tables;

pub(crate) fn migration_v283(connection: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    guards::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}
