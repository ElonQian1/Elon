//! Schema migration for verified merchant runtimes.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v113(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = OFF;
         BEGIN;
         CREATE TABLE open_commerce_capabilities_v113 (
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
                                CHECK(handler_type IN ('merchant_profile', 'static_json', 'merchant_runtime')),
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
         INSERT INTO open_commerce_capabilities_v113
         SELECT * FROM open_commerce_capabilities;
         DROP TABLE open_commerce_capabilities;
         ALTER TABLE open_commerce_capabilities_v113 RENAME TO open_commerce_capabilities;
         CREATE INDEX idx_open_commerce_capabilities_discovery
           ON open_commerce_capabilities(status, capability_key, access_level);

         CREATE TABLE open_commerce_runtime_bindings (
           id                    TEXT PRIMARY KEY,
           project_id            TEXT NOT NULL,
           merchant_id           TEXT NOT NULL UNIQUE,
           endpoint_base_url     TEXT NOT NULL,
           credential_ref        TEXT NOT NULL,
           manifest_sha256       TEXT,
           timeout_ms            INTEGER NOT NULL DEFAULT 5000
                                 CHECK(timeout_ms BETWEEN 500 AND 15000),
           status                TEXT NOT NULL DEFAULT 'configured'
                                 CHECK(status IN ('configured', 'active', 'degraded', 'disabled')),
           last_verified_at      TEXT,
           last_error_code       TEXT,
           created_by_user_id    TEXT NOT NULL,
           created_at            TEXT NOT NULL,
           updated_at            TEXT NOT NULL,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE
         );
         CREATE INDEX idx_open_commerce_runtime_project
           ON open_commerce_runtime_bindings(project_id, status, updated_at DESC);
         COMMIT;
         PRAGMA foreign_keys = ON;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_commerce_migration::migration_v108;

    #[test]
    fn migration_preserves_capabilities_and_adds_runtime_handler() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             INSERT INTO projects(id) VALUES ('project-1');",
        )
        .unwrap();
        migration_v108(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO open_commerce_merchants
             (id, project_id, owner_user_id, slug, display_name, description, status,
              node_mode, public_profile_json, created_at, updated_at)
             VALUES ('merchant-1', 'project-1', 'user-1', 'coffee', 'Coffee', '', 'active',
                     'self_hosted', '{}', 'now', 'now');
             INSERT INTO open_commerce_capabilities
             (id, merchant_id, capability_key, display_name, kind, access_level,
              handler_type, created_at, updated_at)
             VALUES ('cap-1', 'merchant-1', 'merchant.profile.read', 'Profile', 'query',
                     'public', 'merchant_profile', 'now', 'now');",
        )
        .unwrap();

        migration_v113(&conn).unwrap();
        let preserved: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM open_commerce_capabilities",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preserved, 1);
        conn.execute_batch(
            "INSERT INTO open_commerce_capabilities
             (id, merchant_id, capability_key, display_name, kind, access_level,
              handler_type, created_at, updated_at)
             VALUES ('cap-2', 'merchant-1', 'catalog.search', 'Catalog', 'query',
                     'public', 'merchant_runtime', 'now', 'now');",
        )
        .unwrap();
        let violations: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(violations, 0);
    }
}
