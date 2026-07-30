//! SQLite schema for merchant data-source integrations and sync receipts.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v109(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_integrations (
           id                  TEXT PRIMARY KEY,
           project_id          TEXT NOT NULL,
           merchant_id         TEXT NOT NULL,
           integration_key     TEXT NOT NULL,
           provider_key        TEXT NOT NULL,
           display_name        TEXT NOT NULL,
           connection_mode     TEXT NOT NULL
                               CHECK(connection_mode IN (
                                 'official_api', 'merchant_export',
                                 'local_adapter', 'manual_import'
                               )),
           status              TEXT NOT NULL DEFAULT 'configured'
                               CHECK(status IN (
                                 'configured', 'connected', 'degraded', 'disabled'
                               )),
           scopes_json         TEXT NOT NULL DEFAULT '[]',
           data_domains_json   TEXT NOT NULL DEFAULT '[]',
           created_by_user_id  TEXT NOT NULL,
           last_verified_at    TEXT,
           last_sync_at        TEXT,
           created_at          TEXT NOT NULL,
           updated_at          TEXT NOT NULL,
           UNIQUE(merchant_id, integration_key),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_integrations_project_status
           ON open_commerce_integrations(project_id, status, updated_at DESC);

         CREATE TABLE IF NOT EXISTS open_commerce_sync_receipts (
           id                   TEXT PRIMARY KEY,
           project_id           TEXT NOT NULL,
           integration_id       TEXT NOT NULL,
           receipt_key          TEXT NOT NULL,
           receipt_fingerprint  TEXT NOT NULL,
           sync_kind            TEXT NOT NULL
                                CHECK(sync_kind IN ('full', 'incremental', 'health_check')),
           status               TEXT NOT NULL
                                CHECK(status IN ('succeeded', 'partial', 'failed')),
           records_seen         INTEGER NOT NULL DEFAULT 0 CHECK(records_seen >= 0),
           records_changed      INTEGER NOT NULL DEFAULT 0 CHECK(records_changed >= 0),
           cursor_digest        TEXT,
           error_code           TEXT,
           recorded_by_user_id  TEXT NOT NULL,
           recorded_by_app_id   TEXT NOT NULL,
           started_at           TEXT NOT NULL,
           completed_at         TEXT NOT NULL,
           created_at           TEXT NOT NULL,
           UNIQUE(integration_id, receipt_key),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(integration_id) REFERENCES open_commerce_integrations(id)
             ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_sync_receipts_project_time
           ON open_commerce_sync_receipts(project_id, created_at DESC);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_commerce_migration::migration_v108;

    #[test]
    fn migration_creates_integration_tables_with_bounded_states() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE projects(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v108(&conn).unwrap();
        migration_v109(&conn).unwrap();
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name IN (
                   'open_commerce_integrations', 'open_commerce_sync_receipts'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 2);
    }
}
