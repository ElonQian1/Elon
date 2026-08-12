use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod tables;
mod view;

pub(crate) fn migration_v231(conn: &Connection) -> Result<()> {
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
    fn signed_provenance_migration_is_repeatable() {
        let connection = Connection::open_in_memory().unwrap();
        crate::store_migrations::compute_external_pool_adapter_release::migration_v222(&connection)
            .unwrap();
        crate::store_migrations::compute_external_pool_adapter_artifact_source::migration_v227(
            &connection,
        )
        .unwrap();
        crate::store_migrations::compute_external_pool_adapter_release_lifecycle::migration_v229(
            &connection,
        )
        .unwrap();
        crate::store_migrations::compute_external_pool_adapter_artifact_signing_key::migration_v230(
            &connection,
        )
        .unwrap();
        migration_v231(&connection).unwrap();
        migration_v231(&connection).unwrap();
        for object in [
            "compute_external_pool_adapter_artifact_signed_provenance_receipts",
            "compute_external_pool_adapter_artifact_signed_provenance_current",
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
