use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod receipt_integrity;
mod tables;
mod view;

pub(super) fn register_receipt_integrity_functions(conn: &Connection) -> Result<()> {
    receipt_integrity::register(conn)
}

pub(crate) fn migration_v259(conn: &Connection) -> Result<()> {
    register_receipt_integrity_functions(conn)?;
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    view::install(&transaction)?;
    guards::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}

pub(super) fn reinstall_current_policy(conn: &Connection) -> Result<()> {
    guards::reinstall_current_policy(conn)?;
    view::install(conn)?;
    Ok(())
}

#[cfg(test)]
#[path = "compute_external_pool_adapter_supervisor_session_policy_companion/tests.rs"]
mod tests;
