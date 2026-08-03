use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v137(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "ALTER TABLE open_commerce_adapter_credentials ADD COLUMN expires_at TEXT;
         UPDATE open_commerce_adapter_credentials
            SET expires_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+90 days')
          WHERE expires_at IS NULL;
         CREATE INDEX idx_open_commerce_adapter_credentials_expiry
           ON open_commerce_adapter_credentials(status, expires_at);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_backfills_expiration_for_existing_credentials() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE open_commerce_adapter_credentials (
               id TEXT PRIMARY KEY,
               status TEXT NOT NULL
             );
             INSERT INTO open_commerce_adapter_credentials(id, status)
             VALUES ('credential-1', 'active');",
        )
        .unwrap();
        migration_v137(&conn).unwrap();
        let expires_at: String = conn
            .query_row(
                "SELECT expires_at FROM open_commerce_adapter_credentials WHERE id='credential-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(chrono::DateTime::parse_from_rfc3339(&expires_at).is_ok());
    }
}
