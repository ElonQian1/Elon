use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod tables;
mod view;

pub(crate) fn migration_v230(conn: &Connection) -> Result<()> {
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
    fn external_pool_adapter_artifact_signing_key_migration_is_repeatable() {
        let connection = Connection::open_in_memory().unwrap();
        migration_v230(&connection).unwrap();
        migration_v230(&connection).unwrap();
        for object in [
            "compute_external_pool_adapter_artifact_signing_keys",
            "compute_external_pool_adapter_artifact_signing_key_activations",
            "compute_external_pool_adapter_artifact_signing_key_revocations",
            "compute_external_pool_adapter_artifact_signing_key_current",
        ] {
            let exists: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name=?1",
                    [object],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing migration object {object}");
        }
    }
}
