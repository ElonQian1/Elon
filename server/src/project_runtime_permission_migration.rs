use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v103(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_runtime_permissions_v103 (
          project_id TEXT PRIMARY KEY,
          mode       TEXT NOT NULL DEFAULT 'full_access'
                     CHECK (mode IN ('project_write', 'full_access', 'danger_full_access')),
          updated_by TEXT,
          updated_at TEXT,
          expires_at TEXT,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (updated_by) REFERENCES users(id)
        );

        INSERT OR REPLACE INTO project_runtime_permissions_v103
          (project_id, mode, updated_by, updated_at, expires_at)
        SELECT project_id,
               CASE
                 WHEN mode IN ('project_write', 'full_access', 'danger_full_access') THEN mode
                 ELSE 'full_access'
               END,
               updated_by,
               updated_at,
               expires_at
          FROM project_runtime_permissions;

        DROP TABLE project_runtime_permissions;
        ALTER TABLE project_runtime_permissions_v103 RENAME TO project_runtime_permissions;
        "#,
    )?;
    Ok(())
}

/// Restore the safe schema default after v103 briefly defaulted new rows to
/// `danger_full_access`. All valid existing rows are explicit user/project
/// choices and are preserved unchanged.
pub(crate) fn migration_v104(conn: &Connection) -> Result<()> {
    migration_v103(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_preserves_explicit_modes_and_defaults_new_rows_to_full_access() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE users (id TEXT PRIMARY KEY);
            CREATE TABLE projects (id TEXT PRIMARY KEY);
            INSERT INTO projects (id) VALUES
              ('explicit-project-write'), ('explicit-full'), ('legacy-invalid'), ('new-default');
            CREATE TABLE project_runtime_permissions (
              project_id TEXT PRIMARY KEY,
              mode TEXT NOT NULL DEFAULT 'project_write',
              updated_by TEXT,
              updated_at TEXT,
              expires_at TEXT
            );
            INSERT INTO project_runtime_permissions (project_id, mode) VALUES
              ('explicit-project-write', 'project_write'),
              ('explicit-full', 'full_access'),
              ('legacy-invalid', 'unknown');
            "#,
        )
        .unwrap();

        migration_v103(&conn).unwrap();
        migration_v103(&conn).unwrap();
        conn.execute(
            "INSERT INTO project_runtime_permissions (project_id) VALUES ('new-default')",
            [],
        )
        .unwrap();

        let explicit: String = conn
            .query_row(
                "SELECT mode FROM project_runtime_permissions WHERE project_id = 'explicit-project-write'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let defaulted: String = conn
            .query_row(
                "SELECT mode FROM project_runtime_permissions WHERE project_id = 'new-default'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let explicit_full: String = conn
            .query_row(
                "SELECT mode FROM project_runtime_permissions WHERE project_id = 'explicit-full'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let legacy_invalid: String = conn
            .query_row(
                "SELECT mode FROM project_runtime_permissions WHERE project_id = 'legacy-invalid'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(explicit, "project_write");
        assert_eq!(explicit_full, "full_access");
        assert_eq!(legacy_invalid, "full_access");
        assert_eq!(defaulted, "full_access");
    }

    #[test]
    fn v104_defaults_new_rows_to_full_access_and_preserves_explicit_danger() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE users (id TEXT PRIMARY KEY);
            CREATE TABLE projects (id TEXT PRIMARY KEY);
            INSERT INTO projects (id) VALUES ('explicit-danger'), ('new-default');
            CREATE TABLE project_runtime_permissions (
              project_id TEXT PRIMARY KEY,
              mode TEXT NOT NULL DEFAULT 'full_access',
              updated_by TEXT, updated_at TEXT, expires_at TEXT
            );
            INSERT INTO project_runtime_permissions (project_id, mode)
            VALUES ('explicit-danger', 'danger_full_access');
            "#,
        )
        .unwrap();
        migration_v104(&conn).unwrap();
        conn.execute(
            "INSERT INTO project_runtime_permissions (project_id) VALUES ('new-default')",
            [],
        )
        .unwrap();
        let explicit: String = conn
            .query_row(
                "SELECT mode FROM project_runtime_permissions WHERE project_id='explicit-danger'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let defaulted: String = conn
            .query_row(
                "SELECT mode FROM project_runtime_permissions WHERE project_id='new-default'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(explicit, "danger_full_access");
        assert_eq!(defaulted, "full_access");
    }
}
