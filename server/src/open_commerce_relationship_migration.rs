use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v127(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_consumer_relationships (
           id                   TEXT PRIMARY KEY,
           consumer_project_id  TEXT NOT NULL,
           consumer_user_id     TEXT NOT NULL,
           merchant_project_id  TEXT NOT NULL,
           merchant_id          TEXT NOT NULL,
           source_app_id        TEXT NOT NULL,
           subject_alias        TEXT NOT NULL UNIQUE,
           scopes_json          TEXT NOT NULL,
           purpose              TEXT NOT NULL,
           status               TEXT NOT NULL DEFAULT 'active'
                                CHECK(status IN ('active', 'revoked')),
           expires_at           TEXT NOT NULL,
           revoked_at           TEXT,
           created_at           TEXT NOT NULL,
           updated_at           TEXT NOT NULL,
           FOREIGN KEY(consumer_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_relationships_consumer
           ON open_commerce_consumer_relationships(
             consumer_project_id, consumer_user_id, updated_at DESC
           );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_relationships_merchant
           ON open_commerce_consumer_relationships(
             merchant_project_id, merchant_id, updated_at DESC
           );",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_consumer_relationship_indexes() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_merchants(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v127(&conn).unwrap();
        migration_v127(&conn).unwrap();
        let indexes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='index' AND name LIKE 'idx_open_commerce_relationships_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(indexes, 2);
    }
}
