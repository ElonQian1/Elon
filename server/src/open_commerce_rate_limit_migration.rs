//! Persistent, merchant-controlled rate limits for open-commerce invocations.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v117(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_rate_limit_policies (
           id                 TEXT PRIMARY KEY,
           project_id         TEXT NOT NULL,
           merchant_id        TEXT NOT NULL,
           capability_id      TEXT NOT NULL,
           capability_key     TEXT NOT NULL,
           requester_app_id   TEXT NOT NULL DEFAULT '*',
           window_seconds     INTEGER NOT NULL CHECK(window_seconds BETWEEN 1 AND 86400),
           max_requests       INTEGER NOT NULL CHECK(max_requests BETWEEN 1 AND 1000000),
           status             TEXT NOT NULL DEFAULT 'active'
                              CHECK(status IN ('active', 'disabled')),
           created_by_user_id TEXT NOT NULL,
           created_at         TEXT NOT NULL,
           updated_at         TEXT NOT NULL,
           UNIQUE(capability_id, requester_app_id),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(capability_id) REFERENCES open_commerce_capabilities(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_rate_limit_project
           ON open_commerce_rate_limit_policies(project_id, merchant_id, status);

         CREATE TABLE IF NOT EXISTS open_commerce_rate_limit_counters (
           policy_id          TEXT NOT NULL,
           subject_key        TEXT NOT NULL,
           window_started_at  INTEGER NOT NULL,
           request_count      INTEGER NOT NULL DEFAULT 0 CHECK(request_count >= 0),
           updated_at         TEXT NOT NULL,
           PRIMARY KEY(policy_id, subject_key),
           FOREIGN KEY(policy_id) REFERENCES open_commerce_rate_limit_policies(id) ON DELETE CASCADE
         );",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_policy_and_bounded_counter_tables() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_merchants(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_capabilities(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v117(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name LIKE 'open_commerce_rate_limit_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }
}
