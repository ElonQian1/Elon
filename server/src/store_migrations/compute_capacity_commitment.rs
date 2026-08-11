use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod claim_guards;
mod commitment_guards;
mod immutability_guards;
mod tables;
mod terminal_guards;

pub(crate) fn migration_v225(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    commitment_guards::install(&transaction)?;
    claim_guards::install(&transaction)?;
    terminal_guards::install(&transaction)?;
    immutability_guards::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}
