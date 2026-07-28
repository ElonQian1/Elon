//! SQLite schema for the open commerce V1 vertical slice.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v108(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_merchants (
           id                  TEXT PRIMARY KEY,
           project_id          TEXT NOT NULL,
           owner_user_id       TEXT NOT NULL,
           slug                TEXT NOT NULL,
           display_name        TEXT NOT NULL,
           description         TEXT NOT NULL DEFAULT '',
           status              TEXT NOT NULL DEFAULT 'active'
                               CHECK(status IN ('active', 'disabled')),
           node_mode           TEXT NOT NULL DEFAULT 'platform_hosted',
           public_profile_json TEXT NOT NULL DEFAULT '{}',
           created_at          TEXT NOT NULL,
           updated_at          TEXT NOT NULL,
           UNIQUE(project_id, slug),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_merchants_discovery
           ON open_commerce_merchants(status, display_name, slug);

         CREATE TABLE IF NOT EXISTS open_commerce_capabilities (
           id                   TEXT PRIMARY KEY,
           merchant_id          TEXT NOT NULL,
           capability_key       TEXT NOT NULL,
           display_name         TEXT NOT NULL,
           description          TEXT NOT NULL DEFAULT '',
           kind                 TEXT NOT NULL CHECK(kind IN ('query', 'action')),
           access_level         TEXT NOT NULL
                                CHECK(access_level IN ('public', 'authorized', 'owner_only')),
           input_schema_json    TEXT NOT NULL DEFAULT '{}',
           output_schema_json   TEXT NOT NULL DEFAULT '{}',
           handler_type         TEXT NOT NULL
                                CHECK(handler_type IN ('merchant_profile', 'static_json')),
           handler_config_json  TEXT,
           unit_price_micros    INTEGER NOT NULL DEFAULT 0 CHECK(unit_price_micros >= 0),
           currency             TEXT NOT NULL DEFAULT 'CNY',
           freshness_seconds    INTEGER NOT NULL DEFAULT 0 CHECK(freshness_seconds >= 0),
           status               TEXT NOT NULL DEFAULT 'active'
                                CHECK(status IN ('active', 'disabled')),
           version              INTEGER NOT NULL DEFAULT 1 CHECK(version > 0),
           created_at           TEXT NOT NULL,
           updated_at           TEXT NOT NULL,
           UNIQUE(merchant_id, capability_key),
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_capabilities_discovery
           ON open_commerce_capabilities(status, capability_key, access_level);

         CREATE TABLE IF NOT EXISTS open_commerce_grants (
           id                TEXT PRIMARY KEY,
           project_id        TEXT NOT NULL,
           merchant_id       TEXT NOT NULL,
           grantor_user_id   TEXT NOT NULL,
           grantee_app_id    TEXT NOT NULL,
           scopes_json       TEXT NOT NULL,
           purpose           TEXT NOT NULL,
           expires_at        TEXT,
           revoked_at        TEXT,
           created_at        TEXT NOT NULL,
           updated_at        TEXT NOT NULL,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_grants_active
           ON open_commerce_grants(merchant_id, grantee_app_id, revoked_at, expires_at);

         CREATE TABLE IF NOT EXISTS open_commerce_invocations (
           id                  TEXT PRIMARY KEY,
           project_id          TEXT NOT NULL,
           merchant_id         TEXT NOT NULL,
           capability_id       TEXT NOT NULL,
           capability_key      TEXT NOT NULL,
           requester_user_id   TEXT NOT NULL,
           requester_app_id    TEXT NOT NULL,
           grant_id            TEXT,
           idempotency_key     TEXT NOT NULL,
           request_hash        TEXT NOT NULL,
           request_shape_json  TEXT NOT NULL,
           status              TEXT NOT NULL
                               CHECK(status IN ('started', 'succeeded', 'failed')),
           result_json         TEXT,
           error_code          TEXT,
           units               INTEGER NOT NULL DEFAULT 0 CHECK(units >= 0),
           unit_price_micros   INTEGER NOT NULL DEFAULT 0 CHECK(unit_price_micros >= 0),
           amount_micros       INTEGER NOT NULL DEFAULT 0 CHECK(amount_micros >= 0),
           currency            TEXT NOT NULL DEFAULT 'CNY',
           settlement_status   TEXT NOT NULL DEFAULT 'recorded_not_charged',
           created_at          TEXT NOT NULL,
           completed_at        TEXT,
           UNIQUE(
             requester_user_id,
             requester_app_id,
             merchant_id,
             capability_id,
             idempotency_key
           ),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(capability_id) REFERENCES open_commerce_capabilities(id) ON DELETE CASCADE,
           FOREIGN KEY(grant_id) REFERENCES open_commerce_grants(id) ON DELETE SET NULL
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_invocations_project_time
           ON open_commerce_invocations(project_id, created_at DESC);

         CREATE TABLE IF NOT EXISTS open_commerce_audit_events (
           id             TEXT PRIMARY KEY,
           project_id     TEXT NOT NULL,
           actor_user_id  TEXT NOT NULL,
           actor_app_id   TEXT,
           action         TEXT NOT NULL,
           subject_type   TEXT NOT NULL,
           subject_id     TEXT NOT NULL,
           metadata_json  TEXT NOT NULL DEFAULT '{}',
           created_at     TEXT NOT NULL,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_audit_project_time
           ON open_commerce_audit_events(project_id, created_at DESC);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_v1_tables_and_guards_handler_types() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE projects(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v108(&conn).unwrap();
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name LIKE 'open_commerce_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 5);
    }
}
