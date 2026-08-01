//! Merchant-controlled developer App blocks and emergency revocation state.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v118(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_merchant_app_blocks (
           id                    TEXT PRIMARY KEY,
           project_id            TEXT NOT NULL,
           merchant_id           TEXT NOT NULL,
           requester_app_id      TEXT NOT NULL,
           reason_code           TEXT NOT NULL
                                 CHECK(reason_code IN (
                                   'abusive_traffic', 'policy_violation',
                                   'security_incident', 'merchant_request', 'other'
                                 )),
           reason_note           TEXT NOT NULL DEFAULT '',
           status                TEXT NOT NULL DEFAULT 'active'
                                 CHECK(status IN ('active', 'unblocked')),
           blocked_by_user_id    TEXT NOT NULL,
           unblocked_by_user_id  TEXT,
           blocked_at            TEXT NOT NULL,
           unblocked_at          TEXT,
           created_at            TEXT NOT NULL,
           updated_at            TEXT NOT NULL,
           UNIQUE(merchant_id, requester_app_id),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(requester_app_id) REFERENCES open_commerce_developer_apps(app_id) ON DELETE CASCADE,
           FOREIGN KEY(blocked_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
           FOREIGN KEY(unblocked_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_app_blocks_project
           ON open_commerce_merchant_app_blocks(project_id, merchant_id, status, updated_at DESC);
         CREATE INDEX IF NOT EXISTS idx_open_commerce_app_blocks_lookup
           ON open_commerce_merchant_app_blocks(merchant_id, requester_app_id, status);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_block_table_and_lookup_index() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_merchants(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_developer_apps(app_id TEXT UNIQUE);",
        )
        .unwrap();
        migration_v118(&conn).unwrap();
        let objects: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE name IN (
                   'open_commerce_merchant_app_blocks',
                   'idx_open_commerce_app_blocks_lookup'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(objects, 2);
    }
}
