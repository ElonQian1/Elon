use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v240(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS sandbox_conformance_receipt_identity_required
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_conformance_reports
        WHEN NEW.sandbox_conformance_receipt_id IS NULL
        BEGIN
          SELECT RAISE(ABORT,'sandbox conformance receipt identity is required');
        END;

        CREATE TRIGGER IF NOT EXISTS sandbox_conformance_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_conformance_reports
        WHEN EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_sandbox_conformance_reports old
           WHERE old.sandbox_conformance_receipt_id=NEW.sandbox_conformance_receipt_id
              OR old.sandbox_conformance_receipt_digest=NEW.sandbox_conformance_receipt_digest
              OR old.conformance_material_digest=NEW.conformance_material_digest
              OR old.admission_id=NEW.admission_id
              OR old.vulnerability_report_receipt_id=NEW.vulnerability_report_receipt_id
              OR old.vulnerability_report_receipt_digest=NEW.vulnerability_report_receipt_digest
              OR old.verifier_report_id=NEW.verifier_report_id
              OR (old.idempotency_scope=NEW.idempotency_scope
                  AND old.idempotency_key=NEW.idempotency_key)
        )
        BEGIN
          SELECT RAISE(ABORT,'sandbox conformance report cannot replace immutable history');
        END;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_repeatable_and_guards_every_immutable_identity() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE compute_external_pool_adapter_sandbox_conformance_reports (
                  sandbox_conformance_receipt_id TEXT,
                  sandbox_conformance_receipt_digest TEXT,
                  conformance_material_digest TEXT,
                  admission_id TEXT,
                  vulnerability_report_receipt_id TEXT,
                  vulnerability_report_receipt_digest TEXT,
                  verifier_report_id TEXT,
                  idempotency_scope TEXT,
                  idempotency_key TEXT
                );
                INSERT INTO compute_external_pool_adapter_sandbox_conformance_reports VALUES
                  ('receipt-1','digest-1','material-1','admission-1',
                   'vulnerability-1','vulnerability-digest-1','verifier-report-1',
                   'scope-1','key-1');
                "#,
            )
            .unwrap();
        migration_v240(&connection).unwrap();
        migration_v240(&connection).unwrap();

        assert!(insert_with_null_receipt_id(&connection).is_err());

        for (column, value) in [
            ("sandbox_conformance_receipt_id", "receipt-1"),
            ("sandbox_conformance_receipt_digest", "digest-1"),
            ("conformance_material_digest", "material-1"),
            ("admission_id", "admission-1"),
            ("vulnerability_report_receipt_id", "vulnerability-1"),
            (
                "vulnerability_report_receipt_digest",
                "vulnerability-digest-1",
            ),
            ("verifier_report_id", "verifier-report-1"),
        ] {
            assert!(
                insert_with_override(&connection, column, value).is_err(),
                "{column}"
            );
        }
        assert!(insert_with_idempotency(&connection, "scope-1", "key-1").is_err());
    }

    fn insert_with_override(
        connection: &Connection,
        column: &str,
        value: &str,
    ) -> rusqlite::Result<usize> {
        let columns = [
            "sandbox_conformance_receipt_id",
            "sandbox_conformance_receipt_digest",
            "conformance_material_digest",
            "admission_id",
            "vulnerability_report_receipt_id",
            "vulnerability_report_receipt_digest",
            "verifier_report_id",
            "idempotency_scope",
            "idempotency_key",
        ];
        let values = columns
            .iter()
            .map(|candidate| {
                if *candidate == column {
                    format!("'{value}'")
                } else {
                    format!("'{candidate}-fresh'")
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        connection.execute(
            &format!(
                "INSERT OR REPLACE INTO compute_external_pool_adapter_sandbox_conformance_reports
                 VALUES ({values})"
            ),
            [],
        )
    }

    fn insert_with_null_receipt_id(connection: &Connection) -> rusqlite::Result<usize> {
        connection.execute(
            "INSERT INTO compute_external_pool_adapter_sandbox_conformance_reports
             VALUES (NULL,'digest-null','material-null','admission-null',
                     'vulnerability-null','vulnerability-digest-null','verifier-null',
                     'scope-null','key-null')",
            [],
        )
    }

    fn insert_with_idempotency(
        connection: &Connection,
        scope: &str,
        key: &str,
    ) -> rusqlite::Result<usize> {
        connection.execute(
            "INSERT OR REPLACE INTO compute_external_pool_adapter_sandbox_conformance_reports
             VALUES ('receipt-fresh','digest-fresh','material-fresh','admission-fresh',
                     'vulnerability-fresh','vulnerability-digest-fresh','verifier-fresh',?1,?2)",
            [scope, key],
        )
    }
}
