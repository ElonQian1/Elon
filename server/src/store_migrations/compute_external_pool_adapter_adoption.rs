use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod tables;
mod view;

pub(crate) fn migration_v244(conn: &Connection) -> Result<()> {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tables::create(&tx)?;
    guards::install(&tx)?;
    view::install(&tx)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn Adapter_adoption_migration_is_repeatable() {
        let connection = Connection::open_in_memory().unwrap();
        crate::store_schema::apply_migrations(&connection).unwrap();
        migration_v244(&connection).unwrap();
        migration_v244(&connection).unwrap();
        for object in [
            "compute_external_pool_adapter_adoption_receipts",
            "compute_external_pool_adapter_adoption_terminal_receipts",
            "compute_external_pool_adapter_adoption_current",
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
