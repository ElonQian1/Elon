//! Project-scoped developer apps and consumer authorization requests.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v111(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_developer_apps (
           id                TEXT PRIMARY KEY,
           project_id        TEXT NOT NULL,
           owner_user_id     TEXT NOT NULL,
           app_id            TEXT NOT NULL UNIQUE,
           display_name      TEXT NOT NULL,
           environment       TEXT NOT NULL DEFAULT 'sandbox'
                             CHECK(environment IN ('sandbox')),
           status            TEXT NOT NULL DEFAULT 'active'
                             CHECK(status IN ('active', 'disabled')),
           test_token_hash   TEXT NOT NULL UNIQUE,
           token_hint        TEXT NOT NULL,
           created_at        TEXT NOT NULL,
           updated_at        TEXT NOT NULL,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(owner_user_id) REFERENCES users(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_developer_apps_project
           ON open_commerce_developer_apps(project_id, status, updated_at DESC);

         CREATE TABLE IF NOT EXISTS open_commerce_authorization_requests (
           id                  TEXT PRIMARY KEY,
           merchant_project_id TEXT NOT NULL,
           merchant_id         TEXT NOT NULL,
           requester_user_id   TEXT NOT NULL,
           requester_app_id    TEXT NOT NULL,
           scopes_json         TEXT NOT NULL,
           purpose             TEXT NOT NULL,
           status              TEXT NOT NULL DEFAULT 'pending'
                               CHECK(status IN ('pending', 'approved', 'rejected', 'canceled')),
           decided_by_user_id  TEXT,
           decision_reason     TEXT,
           grant_id            TEXT,
           created_at          TEXT NOT NULL,
           updated_at          TEXT NOT NULL,
           FOREIGN KEY(merchant_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(requester_user_id) REFERENCES users(id) ON DELETE CASCADE,
           FOREIGN KEY(grant_id) REFERENCES open_commerce_grants(id) ON DELETE SET NULL
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_auth_requests_project
           ON open_commerce_authorization_requests(
             merchant_project_id, status, updated_at DESC
           );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_auth_requests_requester
           ON open_commerce_authorization_requests(
             requester_user_id, requester_app_id, updated_at DESC
           );",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_commerce_migration::migration_v108;

    #[test]
    fn migration_creates_apps_and_authorization_requests() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v108(&conn).unwrap();
        migration_v111(&conn).unwrap();
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name IN (
                   'open_commerce_developer_apps',
                   'open_commerce_authorization_requests'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 2);
    }
}
