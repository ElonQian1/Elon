use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v128(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_consumer_data_requests (
           id                   TEXT PRIMARY KEY,
           consumer_project_id  TEXT NOT NULL,
           consumer_user_id     TEXT NOT NULL,
           merchant_project_id  TEXT NOT NULL,
           merchant_id          TEXT NOT NULL,
           relationship_id      TEXT NOT NULL,
           subject_alias        TEXT NOT NULL,
           request_type         TEXT NOT NULL DEFAULT 'erase_linked_data'
                                CHECK(request_type = 'erase_linked_data'),
           status               TEXT NOT NULL DEFAULT 'requested'
                                CHECK(status IN (
                                  'requested', 'in_progress', 'completed',
                                  'rejected', 'withdrawn'
                                )),
           resolution_kind      TEXT,
           resolution_note      TEXT,
           requested_at         TEXT NOT NULL,
           accepted_at          TEXT,
           resolved_at          TEXT,
           withdrawn_at         TEXT,
           updated_at           TEXT NOT NULL,
           FOREIGN KEY(consumer_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(relationship_id)
             REFERENCES open_commerce_consumer_relationships(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_data_requests_consumer
           ON open_commerce_consumer_data_requests(
             consumer_project_id, consumer_user_id, updated_at DESC
           );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_data_requests_merchant
           ON open_commerce_consumer_data_requests(
             merchant_project_id, merchant_id, updated_at DESC
           );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_open_commerce_data_requests_open
           ON open_commerce_consumer_data_requests(relationship_id)
           WHERE status IN ('requested', 'in_progress');",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent_and_limits_open_requests() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_merchants(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_consumer_relationships(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v128(&conn).unwrap();
        migration_v128(&conn).unwrap();
        let indexes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='index' AND name LIKE 'idx_open_commerce_data_requests_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(indexes, 3);
    }
}
