use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod guards;
mod tables;
mod view;

pub(crate) fn migration_v241(conn: &Connection) -> Result<()> {
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
    fn credential_verifier_migration_is_repeatable() {
        let connection = Connection::open_in_memory().unwrap();
        migration_v241(&connection).unwrap();
        migration_v241(&connection).unwrap();
        for object in [
            "compute_external_pool_adapter_credential_verifiers",
            "compute_external_pool_adapter_credential_verifier_transitions",
            "compute_external_pool_adapter_credential_verifier_current",
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
        for (table, column) in [
            (
                "compute_external_pool_adapter_credential_verifiers",
                "verifier_record_id",
            ),
            (
                "compute_external_pool_adapter_credential_verifier_transitions",
                "transition_receipt_id",
            ),
        ] {
            let not_null: i64 = connection
                .query_row(
                    &format!("SELECT [notnull] FROM pragma_table_info('{table}') WHERE name=?1"),
                    [column],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(not_null, 1, "{table}.{column} must reject NULL");
        }
        for trigger in [
            "credential_verifier_root_no_replace",
            "credential_verifier_transition_no_replace",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name=?1",
                    [trigger],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "missing no-replace trigger {trigger}");
        }
    }
}
