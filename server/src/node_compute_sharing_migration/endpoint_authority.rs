//! Dormant server authority for versioned node endpoint credentials and authenticated sessions.

use anyhow::Result;
use rusqlite::Connection;

mod credential_guards;
mod owner_reauthentication;
mod replacement_guards;
mod session_guards;
mod tables;

#[cfg(test)]
mod tests;

pub(super) fn migration_v216(conn: &Connection) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tables::create(&tx)?;
    credential_guards::install(&tx)?;
    session_guards::install(&tx)?;
    replacement_guards::install(&tx)?;
    tx.commit()?;
    Ok(())
}

pub(super) fn migration_v217(conn: &Connection) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    owner_reauthentication::install(&tx)?;
    tx.commit()?;
    Ok(())
}
