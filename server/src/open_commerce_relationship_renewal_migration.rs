use anyhow::Result;
use rusqlite::Connection;

use crate::store_migrations::add_column_if_missing;

pub(crate) fn migration_v129(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "open_commerce_consumer_relationships",
        "renewed_from_relationship_id",
        "renewed_from_relationship_id TEXT REFERENCES open_commerce_consumer_relationships(id) ON DELETE SET NULL",
    )?;
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_open_commerce_relationships_renewed_from
           ON open_commerce_consumer_relationships(renewed_from_relationship_id)
           WHERE renewed_from_relationship_id IS NOT NULL;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent_and_limits_each_renewal_source() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_merchants(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        crate::open_commerce_relationship_migration::migration_v127(&conn).unwrap();
        migration_v129(&conn).unwrap();
        migration_v129(&conn).unwrap();

        let has_column = conn
            .prepare("PRAGMA table_info(open_commerce_consumer_relationships)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
            .into_iter()
            .any(|name| name == "renewed_from_relationship_id");
        assert!(has_column);
        let indexes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='index' AND name='idx_open_commerce_relationships_renewed_from'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(indexes, 1);
    }
}
