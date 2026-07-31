//! SQLite governance metadata for ERP blueprints and merchant instances.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v114(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS erp_blueprints (
           id                   TEXT PRIMARY KEY,
           blueprint_key        TEXT NOT NULL UNIQUE,
           source_project_id    TEXT NOT NULL UNIQUE,
           name                 TEXT NOT NULL,
           description          TEXT NOT NULL DEFAULT '',
           proposal_threshold   INTEGER NOT NULL DEFAULT 3 CHECK(proposal_threshold BETWEEN 2 AND 100),
           definition_json      TEXT NOT NULL,
           status               TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'archived')),
           created_by           TEXT NOT NULL,
           created_at           TEXT NOT NULL,
           updated_at           TEXT NOT NULL,
           FOREIGN KEY(source_project_id) REFERENCES projects(id) ON DELETE CASCADE
         );

         CREATE TABLE IF NOT EXISTS erp_blueprint_versions (
           id                   TEXT PRIMARY KEY,
           blueprint_id         TEXT NOT NULL,
           version              TEXT NOT NULL,
           manifest_json        TEXT NOT NULL,
           manifest_sha256      TEXT NOT NULL,
           status               TEXT NOT NULL DEFAULT 'published' CHECK(status IN ('published', 'withdrawn')),
           created_by           TEXT NOT NULL,
           created_at           TEXT NOT NULL,
           UNIQUE(blueprint_id, version),
           FOREIGN KEY(blueprint_id) REFERENCES erp_blueprints(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_erp_blueprint_versions_latest
           ON erp_blueprint_versions(blueprint_id, created_at DESC);

         CREATE TABLE IF NOT EXISTS erp_instances (
           id                       TEXT PRIMARY KEY,
           instance_key             TEXT NOT NULL UNIQUE,
           project_id               TEXT NOT NULL UNIQUE,
           blueprint_id             TEXT NOT NULL,
           pinned_version_id         TEXT NOT NULL,
           industry                 TEXT NOT NULL,
           theme_key                TEXT NOT NULL,
           enabled_modules_json     TEXT NOT NULL,
           plugins_json             TEXT NOT NULL DEFAULT '[]',
           private_extensions_json  TEXT NOT NULL DEFAULT '[]',
           status                   TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'archived')),
           created_by               TEXT NOT NULL,
           created_at               TEXT NOT NULL,
           updated_at               TEXT NOT NULL,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(blueprint_id) REFERENCES erp_blueprints(id) ON DELETE RESTRICT,
           FOREIGN KEY(pinned_version_id) REFERENCES erp_blueprint_versions(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_erp_instances_blueprint
           ON erp_instances(blueprint_id, status, updated_at DESC);

         CREATE TABLE IF NOT EXISTS erp_feature_signals (
           id                    TEXT PRIMARY KEY,
           blueprint_id          TEXT NOT NULL,
           instance_id           TEXT NOT NULL,
           need_key              TEXT NOT NULL,
           requirement_summary   TEXT NOT NULL,
           industry              TEXT NOT NULL,
           requested_outcome     TEXT NOT NULL DEFAULT '',
           evidence_json         TEXT NOT NULL DEFAULT '{}',
           classification        TEXT NOT NULL,
           created_by            TEXT NOT NULL,
           created_at            TEXT NOT NULL,
           updated_at            TEXT NOT NULL,
           UNIQUE(instance_id, need_key),
           FOREIGN KEY(blueprint_id) REFERENCES erp_blueprints(id) ON DELETE CASCADE,
           FOREIGN KEY(instance_id) REFERENCES erp_instances(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_erp_feature_signals_aggregate
           ON erp_feature_signals(blueprint_id, need_key, created_at DESC);

         CREATE TABLE IF NOT EXISTS erp_feature_proposals (
           id                    TEXT PRIMARY KEY,
           blueprint_id          TEXT NOT NULL,
           need_key              TEXT NOT NULL,
           title                 TEXT NOT NULL,
           summary               TEXT NOT NULL,
           status                TEXT NOT NULL DEFAULT 'candidate'
                                 CHECK(status IN ('candidate', 'accepted', 'rejected', 'matter_created')),
           support_count         INTEGER NOT NULL DEFAULT 1,
           industries_json       TEXT NOT NULL DEFAULT '[]',
           matter_id             TEXT,
           decision_by           TEXT,
           decision_note         TEXT,
           created_at            TEXT NOT NULL,
           updated_at            TEXT NOT NULL,
           UNIQUE(blueprint_id, need_key),
           FOREIGN KEY(blueprint_id) REFERENCES erp_blueprints(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_erp_feature_proposals_status
           ON erp_feature_proposals(blueprint_id, status, support_count DESC, updated_at DESC);

         CREATE TABLE IF NOT EXISTS erp_upgrade_campaigns (
           id                                TEXT PRIMARY KEY,
           instance_id                       TEXT NOT NULL,
           from_version_id                   TEXT NOT NULL,
           target_version_id                 TEXT NOT NULL,
           status                            TEXT NOT NULL
                                             CHECK(status IN ('checking', 'ready', 'blocked', 'adopted', 'rolled_back')),
           compatibility_json                TEXT NOT NULL,
           private_extensions_snapshot_json  TEXT NOT NULL,
           created_by                        TEXT NOT NULL,
           decided_by                        TEXT,
           rollback_reason                   TEXT,
           created_at                        TEXT NOT NULL,
           updated_at                        TEXT NOT NULL,
           FOREIGN KEY(instance_id) REFERENCES erp_instances(id) ON DELETE CASCADE,
           FOREIGN KEY(from_version_id) REFERENCES erp_blueprint_versions(id) ON DELETE RESTRICT,
           FOREIGN KEY(target_version_id) REFERENCES erp_blueprint_versions(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_erp_upgrade_campaigns_instance
           ON erp_upgrade_campaigns(instance_id, created_at DESC);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_governance_tables() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE projects(id TEXT PRIMARY KEY);")
            .unwrap();
        migration_v114(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name LIKE 'erp_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 6);
    }
}
