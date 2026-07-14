use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v102(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_android_device_leases (
          lease_id TEXT NOT NULL PRIMARY KEY,
          project_id TEXT NOT NULL,
          hardware_serial TEXT NOT NULL,
          owner_user_id TEXT NOT NULL,
          owner_display_name TEXT NOT NULL,
          client_instance_id TEXT NOT NULL,
          created_at TEXT NOT NULL,
          heartbeat_at TEXT NOT NULL,
          expires_at TEXT NOT NULL,
          UNIQUE (project_id, hardware_serial),
          FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
          FOREIGN KEY (owner_user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_android_device_leases_expiry
          ON project_android_device_leases(expires_at);
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
        migration_v102(&conn).unwrap();
        migration_v102(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='project_android_device_leases'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
