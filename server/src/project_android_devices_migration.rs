use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v98(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_android_devices (
          project_id TEXT NOT NULL,
          hardware_serial TEXT NOT NULL,
          display_name TEXT NOT NULL,
          manufacturer TEXT,
          model TEXT,
          android_sdk INTEGER,
          android_release TEXT,
          last_endpoint TEXT NOT NULL,
          wireless_mode TEXT NOT NULL DEFAULT 'unknown',
          updated_by_user_id TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          PRIMARY KEY (project_id, hardware_serial),
          FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_project_android_devices_updated
          ON project_android_devices(project_id, updated_at DESC);
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
        migration_v98(&conn).unwrap();
        migration_v98(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='project_android_devices'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
