use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod tables;
mod view;

pub(crate) fn migration_v252(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    view::install(&transaction)?;
    guards::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "compute_external_pool_adapter_sandbox_reattestation/tests.rs"]
mod tests;
