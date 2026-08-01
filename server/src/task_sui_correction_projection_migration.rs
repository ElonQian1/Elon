use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v125(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_sui_correction_projection_packages (
           id TEXT PRIMARY KEY,
           project_id TEXT NOT NULL REFERENCES projects(id),
           correction_id TEXT NOT NULL REFERENCES task_settlement_corrections(id),
           reversal_receipt_id TEXT NOT NULL REFERENCES task_settlement_receipts(id),
           replacement_receipt_id TEXT NOT NULL REFERENCES task_settlement_receipts(id),
           target_network TEXT NOT NULL CHECK(target_network IN ('devnet', 'testnet', 'mainnet')),
           package_schema TEXT NOT NULL,
           projection_digest TEXT NOT NULL,
           source_bundle_digest TEXT NOT NULL,
           envelope_json TEXT NOT NULL,
           integrity_status TEXT NOT NULL DEFAULT 'verified'
             CHECK(integrity_status IN ('verified', 'conflict')),
           network_submission TEXT NOT NULL DEFAULT 'not_submitted'
             CHECK(network_submission = 'not_submitted'),
           submission_attempts INTEGER NOT NULL DEFAULT 0 CHECK(submission_attempts = 0),
           last_error TEXT,
           created_by_user_id TEXT NOT NULL REFERENCES users(id),
           verified_at TEXT NOT NULL,
           created_at TEXT NOT NULL,
           updated_at TEXT NOT NULL,
           CHECK(reversal_receipt_id <> replacement_receipt_id),
           UNIQUE(project_id, correction_id, target_network, package_schema)
         );
         CREATE INDEX IF NOT EXISTS idx_task_sui_correction_projection_project_created
           ON task_sui_correction_projection_packages(project_id, created_at DESC);
         CREATE INDEX IF NOT EXISTS idx_task_sui_correction_projection_submission
           ON task_sui_correction_projection_packages(
             network_submission, integrity_status, updated_at
           );",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_atomic_correction_projection_package_table_idempotently() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE project_ai_matters(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        crate::task_settlement_migration::migration_v110(&conn).unwrap();
        crate::task_settlement_dispute_migration::migration_v123(&conn).unwrap();
        crate::task_settlement_correction_migration::migration_v124(&conn).unwrap();
        migration_v125(&conn).unwrap();
        migration_v125(&conn).unwrap();
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                  WHERE type='table'
                    AND name='task_sui_correction_projection_packages'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 1);
        let default_submission: String = conn
            .query_row(
                "SELECT dflt_value
                   FROM pragma_table_info('task_sui_correction_projection_packages')
                  WHERE name='network_submission'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(default_submission, "'not_submitted'");
    }
}
