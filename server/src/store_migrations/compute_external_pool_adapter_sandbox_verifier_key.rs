use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod tables;
mod view;

pub(crate) fn migration_v237(conn: &Connection) -> Result<()> {
    crate::store_migrations::compute_external_pool_adapter_artifact_signing_key::migration_v230(
        conn,
    )?;
    crate::store_migrations::compute_external_pool_adapter_scanner_key::migration_v235(conn)?;
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    guards::install(&transaction)?;
    view::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_verifier_key_migration_is_repeatable() {
        let connection = Connection::open_in_memory().unwrap();
        migration_v237(&connection).unwrap();
        migration_v237(&connection).unwrap();
        for object in [
            "compute_external_pool_adapter_sandbox_verifier_keys",
            "compute_external_pool_adapter_sandbox_verifier_key_transitions",
            "compute_external_pool_adapter_sandbox_verifier_key_current",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name=?1",
                    [object],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "missing migration object {object}");
        }
    }
}
