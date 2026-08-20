use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod receipt_integrity;
mod tables;
mod v253_registering_bridge;
mod view;

#[cfg(test)]
mod dynamic_tests;

pub(super) fn register_receipt_integrity_functions(conn: &Connection) -> Result<()> {
    receipt_integrity::register(conn)
}

pub(super) fn recreate_empty_schema_for_v277(conn: &Connection) -> Result<()> {
    tables::create(conn)?;
    guards::install(conn)?;
    view::install(conn)?;
    v253_registering_bridge::install(conn)?;
    Ok(())
}

pub(crate) fn migration_v274(conn: &Connection) -> Result<()> {
    register_receipt_integrity_functions(conn)?;
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    guards::install(&transaction)?;
    view::install(&transaction)?;
    v253_registering_bridge::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}
