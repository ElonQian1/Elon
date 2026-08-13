use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod precheck;
mod tables;

pub(crate) fn migration_v254(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    precheck::reject_existing_anomalies(&transaction)?;
    tables::create(&transaction)?;
    guards::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "compute_external_pool_provider_activation_candidate/tests.rs"]
mod tests;
