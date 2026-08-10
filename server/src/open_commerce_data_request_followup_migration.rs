use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v163(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_data_request_followups (
           id                   TEXT PRIMARY KEY,
           data_request_id      TEXT NOT NULL,
           consumer_project_id  TEXT NOT NULL,
           consumer_user_id     TEXT NOT NULL,
           merchant_project_id  TEXT NOT NULL,
           merchant_id          TEXT NOT NULL,
           action_kind          TEXT NOT NULL
                                CHECK(action_kind IN ('reminder', 'escalate_attention')),
           idempotency_key      TEXT NOT NULL,
           note                 TEXT,
           created_at           TEXT NOT NULL,
           FOREIGN KEY(data_request_id)
             REFERENCES open_commerce_consumer_data_requests(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           UNIQUE(data_request_id, consumer_user_id, idempotency_key)
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_data_request_followups_request
           ON open_commerce_data_request_followups(data_request_id, created_at DESC);
         CREATE INDEX IF NOT EXISTS idx_open_commerce_data_request_followups_merchant
           ON open_commerce_data_request_followups(
             merchant_project_id, merchant_id, created_at DESC
           );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_open_commerce_data_request_one_escalation
           ON open_commerce_data_request_followups(data_request_id)
           WHERE action_kind='escalate_attention';",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_upgrades_predecessor_schema_idempotently_and_cascades() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_merchants(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_consumer_data_requests(id TEXT PRIMARY KEY);
             INSERT INTO users(id) VALUES ('consumer');
             INSERT INTO projects(id) VALUES ('consumer-project'), ('merchant-project');
             INSERT INTO open_commerce_merchants(id) VALUES ('merchant');
             INSERT INTO open_commerce_consumer_data_requests(id) VALUES ('request');",
        )
        .unwrap();

        migration_v163(&conn).unwrap();
        migration_v163(&conn).unwrap();
        conn.execute(
            "INSERT INTO open_commerce_data_request_followups (
               id, data_request_id, consumer_project_id, consumer_user_id,
               merchant_project_id, merchant_id, action_kind, idempotency_key,
               note, created_at
             ) VALUES (
               'followup', 'request', 'consumer-project', 'consumer',
               'merchant-project', 'merchant', 'reminder', 'key', NULL,
               '2026-08-01T00:00:00Z'
             )",
            [],
        )
        .unwrap();

        let index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                  WHERE type='index'
                    AND name LIKE 'idx_open_commerce_data_request_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(index_count, 3);
        conn.execute(
            "DELETE FROM open_commerce_consumer_data_requests WHERE id='request'",
            [],
        )
        .unwrap();
        let followup_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM open_commerce_data_request_followups",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(followup_count, 0);
    }
}
