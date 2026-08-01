//! Explicit opt-in publication for the cross-project merchant directory.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v116(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_directory_publications (
           merchant_id          TEXT PRIMARY KEY,
           project_id           TEXT NOT NULL,
           status               TEXT NOT NULL
                                CHECK(status IN ('published', 'unpublished')),
           revision             INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0),
           published_by_user_id TEXT,
           published_at         TEXT,
           unpublished_at       TEXT,
           updated_at           TEXT NOT NULL,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(published_by_user_id) REFERENCES users(id) ON DELETE SET NULL
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_directory_published
           ON open_commerce_directory_publications(status, updated_at DESC);
         CREATE INDEX IF NOT EXISTS idx_open_commerce_directory_project
           ON open_commerce_directory_publications(project_id, updated_at DESC);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_commerce_migration::migration_v108;

    #[test]
    fn migration_creates_explicit_directory_publication_table() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v108(&conn).unwrap();
        migration_v116(&conn).unwrap();
        let table: String = conn
            .query_row(
                "SELECT name FROM sqlite_master
                 WHERE type = 'table' AND name = 'open_commerce_directory_publications'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table, "open_commerce_directory_publications");
    }
}
