use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v135(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_business_handoff_receipts (
           id                       TEXT PRIMARY KEY,
           project_id               TEXT NOT NULL,
           merchant_id              TEXT NOT NULL,
           invocation_id            TEXT NOT NULL,
           integration_id           TEXT NOT NULL,
           receipt_key              TEXT NOT NULL,
           receipt_fingerprint      TEXT NOT NULL,
           status                   TEXT NOT NULL
                                    CHECK(status IN ('applied', 'ignored', 'rejected')),
           target_domain            TEXT NOT NULL
                                    CHECK(target_domain IN ('erp', 'crm')),
           evidence_result_sha256   TEXT NOT NULL,
           target_reference_sha256  TEXT,
           error_code               TEXT,
           confirmed_by_user        INTEGER NOT NULL CHECK(confirmed_by_user IN (0, 1)),
           assertion_authority      TEXT NOT NULL
                                    CHECK(assertion_authority = 'project_editor_asserted'),
           recorded_by_user_id      TEXT NOT NULL,
           recorded_by_app_id       TEXT NOT NULL,
           completed_at             TEXT NOT NULL,
           created_at               TEXT NOT NULL,
           UNIQUE(integration_id, receipt_key),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(invocation_id) REFERENCES open_commerce_invocations(id) ON DELETE CASCADE,
           FOREIGN KEY(integration_id) REFERENCES open_commerce_integrations(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_handoff_merchant_time
           ON open_commerce_business_handoff_receipts(
             project_id, merchant_id, created_at DESC
           );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_handoff_invocation
           ON open_commerce_business_handoff_receipts(invocation_id, created_at DESC);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent_and_bounds_handoff_states() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE projects(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        crate::open_commerce_migration::migration_v108(&conn).unwrap();
        crate::open_commerce_integration_migration::migration_v109(&conn).unwrap();
        migration_v135(&conn).unwrap();
        migration_v135(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table'
                   AND name = 'open_commerce_business_handoff_receipts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
