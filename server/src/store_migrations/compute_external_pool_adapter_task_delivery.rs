use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod receipt_integrity;
mod tables;

pub(super) fn register_receipt_integrity_functions(conn: &Connection) -> Result<()> {
    receipt_integrity::register(conn)
}

pub(crate) fn migration_v273(conn: &Connection) -> Result<()> {
    register_receipt_integrity_functions(conn)?;
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    guards::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}
