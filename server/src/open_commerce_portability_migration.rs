use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v130(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_consumer_portability_exports (
           id                   TEXT PRIMARY KEY,
           consumer_project_id  TEXT NOT NULL,
           consumer_user_id     TEXT NOT NULL,
           idempotency_key      TEXT NOT NULL,
           package_schema       TEXT NOT NULL,
           payload_json         TEXT NOT NULL,
           payload_sha256       TEXT NOT NULL,
           created_at           TEXT NOT NULL,
           FOREIGN KEY(consumer_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE,
           UNIQUE(consumer_project_id, consumer_user_id, idempotency_key)
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_portability_exports_owner
           ON open_commerce_consumer_portability_exports(
             consumer_project_id, consumer_user_id, created_at DESC
           );",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent_and_keys_exports_per_owner() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v130(&conn).unwrap();
        migration_v130(&conn).unwrap();
        let indexes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='index' AND name='idx_open_commerce_portability_exports_owner'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(indexes, 1);
    }
}
