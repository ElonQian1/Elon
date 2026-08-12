use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod tables;
mod views;

pub(crate) fn migration_v249(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    views::install(&transaction)?;
    guards::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}

pub(crate) fn migration_v251(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    guards::repair_release_exact_roots(&transaction)?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "compute_external_pool_adapter_registry/tests.rs"]
mod tests;
