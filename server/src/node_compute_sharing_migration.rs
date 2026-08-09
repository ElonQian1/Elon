//! Explicit, owner-controlled supply policy for shared node inference.

use anyhow::Result;
use rusqlite::Connection;

mod endpoint_authority;

pub(crate) fn migration_v121(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS node_compute_sharing_policies (
           node_id                  TEXT PRIMARY KEY,
           owner_user_id            TEXT NOT NULL,
           enabled                  INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0, 1)),
           allowed_model_ids_json   TEXT NOT NULL DEFAULT '[]',
           max_concurrent_runs      INTEGER NOT NULL DEFAULT 1
                                    CHECK(max_concurrent_runs BETWEEN 1 AND 16),
           daily_token_limit        INTEGER NOT NULL DEFAULT 0
                                    CHECK(daily_token_limit BETWEEN 0 AND 1000000000000),
           created_at               TEXT NOT NULL,
           updated_at               TEXT NOT NULL,
           FOREIGN KEY(node_id) REFERENCES node_credentials(agent_id) ON DELETE CASCADE,
           FOREIGN KEY(owner_user_id) REFERENCES users(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_node_compute_sharing_enabled
           ON node_compute_sharing_policies(enabled, owner_user_id, updated_at);",
    )?;
    Ok(())
}

pub(crate) fn migration_v216(conn: &Connection) -> Result<()> {
    endpoint_authority::migration_v216(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_fail_closed_compute_supply_policy() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE node_credentials(agent_id TEXT PRIMARY KEY);",
        )
        .unwrap();

        migration_v121(&conn).unwrap();

        let sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master
                  WHERE type='table' AND name='node_compute_sharing_policies'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(sql.contains("DEFAULT 0"));
        assert!(sql.contains("max_concurrent_runs BETWEEN 1 AND 16"));
    }
}
