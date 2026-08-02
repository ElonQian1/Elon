use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v131(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_consumer_preference_profiles (
           consumer_project_id TEXT NOT NULL,
           consumer_user_id    TEXT NOT NULL,
           preferences_json    TEXT NOT NULL,
           revision            INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0),
           created_at          TEXT NOT NULL,
           updated_at          TEXT NOT NULL,
           PRIMARY KEY(consumer_project_id, consumer_user_id),
           FOREIGN KEY(consumer_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE
         );
         CREATE TABLE IF NOT EXISTS open_commerce_consumer_preference_disclosures (
           relationship_id   TEXT PRIMARY KEY,
           shared_fields_json TEXT NOT NULL,
           disclosure_json   TEXT NOT NULL,
           profile_revision  INTEGER NOT NULL CHECK(profile_revision > 0),
           created_at        TEXT NOT NULL,
           updated_at        TEXT NOT NULL,
           FOREIGN KEY(relationship_id)
             REFERENCES open_commerce_consumer_relationships(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_preference_disclosures_updated
           ON open_commerce_consumer_preference_disclosures(updated_at DESC);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent_and_scopes_disclosures_to_relationships() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_consumer_relationships(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v131(&conn).unwrap();
        migration_v131(&conn).unwrap();
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='table' AND name LIKE 'open_commerce_consumer_preference_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 2);
    }
}
