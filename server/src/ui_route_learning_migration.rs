use anyhow::Result;
use rusqlite::Connection;

use crate::store_migrations::add_column_if_missing;

pub(crate) fn migration_v97(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS ui_route_learning_entries (
          id                TEXT PRIMARY KEY,
          scope_type        TEXT NOT NULL DEFAULT 'project',
          scope_id          TEXT NOT NULL,
          phrase_key        TEXT NOT NULL,
          sample_text       TEXT NOT NULL,
          learned_route     TEXT NOT NULL,
          status            TEXT NOT NULL DEFAULT 'candidate',
          source            TEXT NOT NULL,
          confidence        REAL NOT NULL DEFAULT 0,
          evidence_count    INTEGER NOT NULL DEFAULT 1,
          conflict_count    INTEGER NOT NULL DEFAULT 0,
          hit_count         INTEGER NOT NULL DEFAULT 0,
          created_by_user_id TEXT,
          last_hit_at       TEXT,
          created_at        TEXT NOT NULL,
          updated_at        TEXT NOT NULL,
          CHECK (scope_type IN ('project', 'global')),
          CHECK (learned_route IN ('ui', 'non_ui')),
          CHECK (status IN ('candidate', 'active', 'revoked')),
          CHECK (source IN ('codex_proposal', 'user_override', 'runtime_verified', 'execution_verified', 'admin')),
          UNIQUE(scope_type, scope_id, phrase_key)
        );

        CREATE INDEX IF NOT EXISTS idx_ui_route_learning_lookup
          ON ui_route_learning_entries(scope_type, scope_id, phrase_key, status);
        CREATE INDEX IF NOT EXISTS idx_ui_route_learning_recent
          ON ui_route_learning_entries(scope_type, scope_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS ui_route_learning_events (
          id            TEXT PRIMARY KEY,
          entry_id      TEXT NOT NULL,
          action        TEXT NOT NULL,
          learned_route TEXT NOT NULL,
          source        TEXT NOT NULL,
          actor_user_id TEXT,
          evidence      TEXT,
          created_at    TEXT NOT NULL,
          FOREIGN KEY (entry_id) REFERENCES ui_route_learning_entries(id)
        );

        CREATE INDEX IF NOT EXISTS idx_ui_route_learning_events_entry
          ON ui_route_learning_events(entry_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v101(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "ui_route_learning_entries",
        "concept_key",
        "concept_key TEXT",
    )?;
    add_column_if_missing(
        conn,
        "ui_route_learning_entries",
        "concept_version",
        "concept_version INTEGER",
    )?;
    add_column_if_missing(
        conn,
        "ui_route_learning_entries",
        "cluster_hit_count",
        "cluster_hit_count INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "ui_route_learning_entries",
        "last_cluster_hit_at",
        "last_cluster_hit_at TEXT",
    )?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_ui_route_learning_concept
          ON ui_route_learning_entries(scope_type, scope_id, concept_key, concept_version, status);

        CREATE TABLE IF NOT EXISTS ui_route_learning_aliases (
          id             TEXT PRIMARY KEY,
          entry_id       TEXT NOT NULL,
          phrase_key     TEXT NOT NULL,
          sample_text    TEXT NOT NULL,
          source         TEXT NOT NULL DEFAULT 'controlled_vocabulary',
          status         TEXT NOT NULL DEFAULT 'active',
          evidence_count INTEGER NOT NULL DEFAULT 1,
          conflict_count INTEGER NOT NULL DEFAULT 0,
          hit_count      INTEGER NOT NULL DEFAULT 0,
          last_hit_at    TEXT,
          created_at     TEXT NOT NULL,
          updated_at     TEXT NOT NULL,
          CHECK (source IN ('controlled_vocabulary', 'user_override', 'execution_verified', 'codex_candidate')),
          CHECK (status IN ('candidate', 'active', 'revoked')),
          UNIQUE(entry_id, phrase_key),
          FOREIGN KEY (entry_id) REFERENCES ui_route_learning_entries(id)
        );

        CREATE INDEX IF NOT EXISTS idx_ui_route_learning_alias_lookup
          ON ui_route_learning_aliases(phrase_key, status);
        CREATE INDEX IF NOT EXISTS idx_ui_route_learning_alias_entry
          ON ui_route_learning_aliases(entry_id, updated_at DESC);
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migration_v97(&conn).unwrap();
        migration_v101(&conn).unwrap();
        migration_v101(&conn).unwrap();
        migration_v97(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name LIKE 'ui_route_learning_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }
}
