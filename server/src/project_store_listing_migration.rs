use anyhow::Result;
use rusqlite::Connection;

use crate::store_migrations::add_column_if_missing;

pub(crate) fn migration_v90(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "projects",
        "source_type",
        "source_type TEXT NOT NULL DEFAULT 'template'",
    )?;
    add_column_if_missing(
        conn,
        "projects",
        "status",
        "status TEXT NOT NULL DEFAULT 'active'",
    )?;
    add_column_if_missing(
        conn,
        "projects",
        "is_public",
        "is_public INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "projects",
        "join_mode",
        "join_mode TEXT NOT NULL DEFAULT 'open'",
    )?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_projects_store_public_updated
          ON projects(is_public, status, join_mode, updated_at DESC)
          WHERE is_public = 1
            AND status != 'deleted'
            AND join_mode != 'invite'
            AND source_type NOT IN ('agent_balloon', 'chat_memory');

        CREATE INDEX IF NOT EXISTS idx_projects_store_public_created
          ON projects(is_public, status, join_mode, created_at DESC)
          WHERE is_public = 1
            AND status != 'deleted'
            AND join_mode != 'invite'
            AND source_type NOT IN ('agent_balloon', 'chat_memory');
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_v90_adds_project_store_listing_indexes() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.execute_batch(
            r#"
            CREATE TABLE projects (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              workspace_key TEXT NOT NULL,
              template TEXT NOT NULL DEFAULT 'android',
              created_by TEXT NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            "#,
        )
        .expect("legacy projects table should apply");

        migration_v90(&conn).expect("store listing indexes should apply");

        for index_name in [
            "idx_projects_store_public_updated",
            "idx_projects_store_public_created",
        ] {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*)
                       FROM sqlite_master
                      WHERE type = 'index'
                        AND name = ?1",
                    [index_name],
                    |row| row.get(0),
                )
                .expect("index lookup should succeed");
            assert_eq!(exists, 1, "{index_name} should exist");
        }
    }
}
