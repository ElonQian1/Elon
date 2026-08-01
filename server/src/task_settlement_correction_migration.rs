use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v124(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_settlement_corrections (
           id TEXT PRIMARY KEY,
           project_id TEXT NOT NULL REFERENCES projects(id),
           dispute_id TEXT NOT NULL REFERENCES task_settlement_disputes(id),
           original_settlement_receipt_id TEXT NOT NULL REFERENCES task_settlement_receipts(id),
           correction_matter_id TEXT NOT NULL UNIQUE REFERENCES project_ai_matters(id),
           status TEXT NOT NULL DEFAULT 'matter_pending'
             CHECK(status IN ('matter_pending', 'posted', 'canceled')),
           corrected_compute_amount_micros INTEGER NOT NULL
             CHECK(corrected_compute_amount_micros >= 0),
           corrected_provider_amount_micros INTEGER NOT NULL
             CHECK(corrected_provider_amount_micros >= 0
               AND corrected_provider_amount_micros <= corrected_compute_amount_micros),
           corrected_platform_amount_micros INTEGER NOT NULL
             CHECK(corrected_platform_amount_micros >= 0
               AND corrected_platform_amount_micros =
                 corrected_compute_amount_micros - corrected_provider_amount_micros),
           summary TEXT NOT NULL,
           evidence_ref TEXT,
           created_by_user_id TEXT NOT NULL REFERENCES users(id),
           posted_by_user_id TEXT REFERENCES users(id),
           reversal_receipt_id TEXT REFERENCES task_settlement_receipts(id),
           replacement_receipt_id TEXT REFERENCES task_settlement_receipts(id),
           created_at TEXT NOT NULL,
           posted_at TEXT,
           updated_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_task_settlement_corrections_original
           ON task_settlement_corrections(project_id, original_settlement_receipt_id, created_at DESC);
         CREATE UNIQUE INDEX IF NOT EXISTS idx_task_settlement_corrections_one_pending
           ON task_settlement_corrections(project_id, dispute_id)
           WHERE status='matter_pending';
         CREATE UNIQUE INDEX IF NOT EXISTS idx_task_settlement_corrections_one_posted
           ON task_settlement_corrections(project_id, dispute_id)
           WHERE status='posted';

         CREATE TABLE IF NOT EXISTS task_settlement_correction_events (
           id TEXT PRIMARY KEY,
           correction_id TEXT NOT NULL REFERENCES task_settlement_corrections(id),
           action TEXT NOT NULL CHECK(action IN ('matter_created', 'posted', 'canceled')),
           previous_status TEXT,
           next_status TEXT NOT NULL
             CHECK(next_status IN ('matter_pending', 'posted', 'canceled')),
           actor_user_id TEXT NOT NULL REFERENCES users(id),
           note TEXT,
           created_at TEXT NOT NULL,
           UNIQUE(correction_id, action)
         );
         CREATE INDEX IF NOT EXISTS idx_task_settlement_correction_events_case
           ON task_settlement_correction_events(correction_id, created_at, id);",
    )?;
    add_column_if_missing(
        conn,
        "task_settlement_receipts",
        "receipt_kind",
        "receipt_kind TEXT NOT NULL DEFAULT 'standard'
           CHECK(receipt_kind IN ('standard', 'correction_reversal', 'correction_replacement'))",
    )?;
    add_column_if_missing(
        conn,
        "task_settlement_receipts",
        "correction_id",
        "correction_id TEXT REFERENCES task_settlement_corrections(id)",
    )?;
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_task_settlement_receipts_correction_kind
           ON task_settlement_receipts(correction_id, receipt_kind)
           WHERE correction_id IS NOT NULL;",
    )?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|name| name == column);
    if !exists {
        conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {definition};"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_correction_workflow_and_receipt_classification() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE project_ai_matters(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        crate::task_settlement_migration::migration_v110(&conn).unwrap();
        crate::task_settlement_dispute_migration::migration_v123(&conn).unwrap();
        migration_v124(&conn).unwrap();
        migration_v124(&conn).unwrap();
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                  WHERE type='table'
                    AND name IN (
                      'task_settlement_corrections',
                      'task_settlement_correction_events'
                    )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 2);
        let columns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('task_settlement_receipts')
                  WHERE name IN ('receipt_kind', 'correction_id')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(columns, 2);
    }
}
