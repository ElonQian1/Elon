use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod tables;
mod view;

pub(crate) fn migration_v247(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    guards::install(&transaction)?;
    view::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "compute_external_pool_adapter_installation_terminal/tests.rs"]
mod tests;
