use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod projection_guards;
mod source_guards;
mod tables;
mod view;

pub(crate) fn migration_v229(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    projection_guards::install(&transaction)?;
    source_guards::install(&transaction)?;
    view::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}
