use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod tables;
mod view;

pub(crate) fn migration_v243(conn: &Connection) -> Result<()> {
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
    fn credential_verification_migration_is_repeatable() {
        let connection = Connection::open_in_memory().unwrap();
        crate::store_schema::apply_migrations(&connection).unwrap();
        migration_v243(&connection).unwrap();
        migration_v243(&connection).unwrap();
        for object in [
            "compute_external_pool_adapter_credential_verification_receipts",
            "compute_external_pool_adapter_credential_verification_current",
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
