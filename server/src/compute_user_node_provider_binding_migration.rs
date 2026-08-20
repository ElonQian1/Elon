//! V279 immutable binding authority for one opted-in node and one `user_node` Provider.

use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod precheck;
mod tables;

pub(crate) fn register_receipt_integrity_function(connection: &Connection) -> Result<()> {
    precheck::register(connection)
}

pub(crate) fn migration_v279(connection: &Connection) -> Result<()> {
    register_receipt_integrity_function(connection)?;
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    precheck::install(&transaction)?;
    guards::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}
