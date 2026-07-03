use anyhow::Result;
use rusqlite::Connection;

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

#[cfg(test)]
mod tests {
    use super::migration_v83;
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
}
