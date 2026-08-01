use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v123(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_settlement_disputes (
           id TEXT PRIMARY KEY,
           project_id TEXT NOT NULL REFERENCES projects(id),
           settlement_receipt_id TEXT NOT NULL REFERENCES task_settlement_receipts(id),
           status TEXT NOT NULL DEFAULT 'open'
             CHECK(status IN ('open', 'accepted', 'rejected', 'withdrawn')),
           reason_code TEXT NOT NULL
             CHECK(reason_code IN ('amount', 'provider_allocation', 'policy', 'source_evidence', 'other')),
           summary TEXT NOT NULL,
           evidence_ref TEXT,
           opened_by_user_id TEXT NOT NULL REFERENCES users(id),
           resolved_by_user_id TEXT REFERENCES users(id),
           resolution_note TEXT,
           opened_at TEXT NOT NULL,
           resolved_at TEXT,
           updated_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_task_settlement_disputes_receipt
           ON task_settlement_disputes(project_id, settlement_receipt_id, opened_at DESC);
         CREATE UNIQUE INDEX IF NOT EXISTS idx_task_settlement_disputes_one_open
           ON task_settlement_disputes(project_id, settlement_receipt_id)
           WHERE status='open';
         CREATE UNIQUE INDEX IF NOT EXISTS idx_task_settlement_disputes_one_accepted
           ON task_settlement_disputes(project_id, settlement_receipt_id)
           WHERE status='accepted';

         CREATE TABLE IF NOT EXISTS task_settlement_dispute_events (
           id TEXT PRIMARY KEY,
           dispute_id TEXT NOT NULL REFERENCES task_settlement_disputes(id),
           action TEXT NOT NULL CHECK(action IN ('opened', 'accepted', 'rejected', 'withdrawn')),
           previous_status TEXT,
           next_status TEXT NOT NULL
             CHECK(next_status IN ('open', 'accepted', 'rejected', 'withdrawn')),
           actor_user_id TEXT NOT NULL REFERENCES users(id),
           note TEXT,
           created_at TEXT NOT NULL,
           UNIQUE(dispute_id, action)
         );
         CREATE INDEX IF NOT EXISTS idx_task_settlement_dispute_events_case
           ON task_settlement_dispute_events(dispute_id, created_at, id);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_append_only_dispute_case_and_event_tables() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        crate::task_settlement_migration::migration_v110(&conn).unwrap();
        migration_v123(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                  WHERE type='table'
                    AND name IN ('task_settlement_disputes', 'task_settlement_dispute_events')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
        let partial_indexes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                  WHERE type='index'
                    AND name IN (
                      'idx_task_settlement_disputes_one_open',
                      'idx_task_settlement_disputes_one_accepted'
                    )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(partial_indexes, 2);
    }
}
