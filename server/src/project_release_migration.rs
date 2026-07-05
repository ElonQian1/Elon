use anyhow::Result;
use rusqlite::Connection;

use crate::store_migrations::add_column_if_missing;

pub(crate) fn migration_v83(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_releases (
          id            TEXT PRIMARY KEY,
          project_id    TEXT NOT NULL,
          task_id       TEXT,
          uploaded_by   TEXT,
          version_name  TEXT,
          channel       TEXT NOT NULL DEFAULT 'internal',
          status        TEXT NOT NULL DEFAULT 'published',
          apk_url       TEXT NOT NULL,
          file_name     TEXT NOT NULL,
          file_path     TEXT,
          sha256        TEXT,
          size_bytes    INTEGER,
          changelog     TEXT,
          created_at    TEXT NOT NULL,
          updated_at    TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (task_id) REFERENCES tasks(id),
          FOREIGN KEY (uploaded_by) REFERENCES users(id)
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_project_releases_task
          ON project_releases(task_id)
          WHERE task_id IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_project_releases_project_status_time
          ON project_releases(project_id, status, updated_at);
        CREATE INDEX IF NOT EXISTS idx_project_releases_project_file
          ON project_releases(project_id, file_name);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v87(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "project_releases",
        "release_number",
        "release_number INTEGER",
    )?;
    add_column_if_missing(
        conn,
        "project_releases",
        "package_name",
        "package_name TEXT",
    )?;
    add_column_if_missing(
        conn,
        "project_releases",
        "version_code",
        "version_code INTEGER",
    )?;
    add_column_if_missing(
        conn,
        "project_releases",
        "build_started_at",
        "build_started_at TEXT",
    )?;
    add_column_if_missing(
        conn,
        "project_releases",
        "source_git_sha",
        "source_git_sha TEXT",
    )?;
    add_column_if_missing(
        conn,
        "project_releases",
        "source_worktree",
        "source_worktree TEXT",
    )?;
    add_column_if_missing(
        conn,
        "project_releases",
        "metadata_json",
        "metadata_json TEXT",
    )?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_project_releases_project_release_number
          ON project_releases(project_id, release_number);
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{migration_v83, migration_v87};
    use rusqlite::Connection;

    #[test]
    fn migration_creates_project_releases() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v83(&conn).expect("release migration should apply");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='project_releases'",
                [],
                |row| row.get(0),
            )
            .expect("table count should load");
        assert_eq!(count, 1);
    }

    #[test]
    fn metadata_migration_extends_project_releases() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v83(&conn).expect("release migration should apply");
        migration_v87(&conn).expect("metadata migration should apply");
        migration_v87(&conn).expect("metadata migration should be idempotent");

        let columns = conn
            .prepare("PRAGMA table_info(project_releases)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();

        assert!(columns.iter().any(|column| column == "release_number"));
        assert!(columns.iter().any(|column| column == "package_name"));
        assert!(columns.iter().any(|column| column == "version_code"));
        assert!(columns.iter().any(|column| column == "source_git_sha"));
    }
}
