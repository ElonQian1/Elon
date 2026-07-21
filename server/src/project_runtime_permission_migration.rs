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

/// Re-run the schema repair for installations that already recorded v104
/// before its corrected implementation reached them.
pub(crate) fn migration_v105(conn: &Connection) -> Result<()> {
    migration_v103(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_v105_schema(conn: &Connection) {
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        let columns: Vec<(String, Option<String>)> = conn
            .prepare("PRAGMA table_info(project_runtime_permissions)")
            .unwrap()
            .query_map([], |row| Ok((row.get(1)?, row.get(4)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            columns,
            vec![
                ("project_id".into(), None),
                ("mode".into(), Some("'full_access'".into())),
                ("updated_by".into(), None),
                ("updated_at".into(), None),
                ("expires_at".into(), None),
            ]
        );

        for mode in ["project_write", "full_access", "danger_full_access"] {
            conn.execute(
                "INSERT INTO project_runtime_permissions (project_id, mode) VALUES (?1, ?2)",
                rusqlite::params![format!("explicit-{mode}"), mode],
            )
            .unwrap();
        }
        assert!(conn
            .execute(
                "INSERT INTO project_runtime_permissions (project_id, mode) VALUES ('invalid-new', 'invalid')",
                [],
            )
            .is_err());
        conn.execute(
            "INSERT INTO project_runtime_permissions (project_id) VALUES ('new-default')",
            [],
        )
        .unwrap();
        let defaulted: String = conn
            .query_row(
                "SELECT mode FROM project_runtime_permissions WHERE project_id='new-default'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(defaulted, "full_access");
    }

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

    #[test]
    fn fresh_database_applies_unique_v105_with_full_access_schema() {
        let conn = Connection::open_in_memory().unwrap();
        crate::store_schema::apply_migrations(&conn).unwrap();
        crate::store_schema::apply_migrations(&conn).unwrap();

        let applied: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 105",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(applied, 1);
        assert_v105_schema(&conn);
    }

    #[test]
    fn v104_existing_database_rolls_forward_preserving_valid_modes_and_audit() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE users (id TEXT PRIMARY KEY);
            CREATE TABLE projects (id TEXT PRIMARY KEY);
            INSERT INTO users (id) VALUES ('user-1'), ('user-2'), ('user-3'), ('user-4');
            INSERT INTO projects (id) VALUES
              ('kept-project-write'), ('kept-full'), ('kept-danger'), ('legacy-invalid');
            CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
            INSERT INTO schema_migrations (version, applied_at) VALUES (104, 'now');
            CREATE TABLE project_runtime_permissions (
              project_id TEXT PRIMARY KEY,
              mode TEXT NOT NULL DEFAULT 'danger_full_access',
              updated_by TEXT,
              updated_at TEXT,
              expires_at TEXT
            );
            INSERT INTO project_runtime_permissions
              (project_id, mode, updated_by, updated_at, expires_at) VALUES
              ('kept-project-write', 'project_write', 'user-1', 'time-1', 'expiry-1'),
              ('kept-full', 'full_access', 'user-2', 'time-2', 'expiry-2'),
              ('kept-danger', 'danger_full_access', 'user-3', 'time-3', 'expiry-3'),
              ('legacy-invalid', 'invalid', 'user-4', 'time-4', 'expiry-4');
            "#,
        )
        .unwrap();

        crate::store_schema::apply_migrations(&conn).unwrap();
        migration_v105(&conn).unwrap();

        let rows: Vec<(
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = conn
            .prepare(
                "SELECT project_id, mode, updated_by, updated_at, expires_at
                   FROM project_runtime_permissions
                  WHERE project_id LIKE 'kept-%' OR project_id = 'legacy-invalid'
                  ORDER BY project_id",
            )
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                (
                    "kept-danger".into(),
                    "danger_full_access".into(),
                    Some("user-3".into()),
                    Some("time-3".into()),
                    Some("expiry-3".into())
                ),
                (
                    "kept-full".into(),
                    "full_access".into(),
                    Some("user-2".into()),
                    Some("time-2".into()),
                    Some("expiry-2".into())
                ),
                (
                    "kept-project-write".into(),
                    "project_write".into(),
                    Some("user-1".into()),
                    Some("time-1".into()),
                    Some("expiry-1".into())
                ),
                (
                    "legacy-invalid".into(),
                    "full_access".into(),
                    Some("user-4".into()),
                    Some("time-4".into()),
                    Some("expiry-4".into())
                ),
            ]
        );
        assert_v105_schema(&conn);
    }
}
