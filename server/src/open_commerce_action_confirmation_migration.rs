use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v133(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_action_confirmations (
           id                  TEXT PRIMARY KEY,
           project_id          TEXT NOT NULL,
           merchant_id         TEXT NOT NULL,
           capability_id       TEXT NOT NULL,
           capability_key      TEXT NOT NULL,
           requester_user_id   TEXT NOT NULL,
           requester_app_id    TEXT NOT NULL,
           grant_id            TEXT,
           idempotency_key     TEXT NOT NULL,
           request_hash        TEXT NOT NULL,
           request_shape_json  TEXT NOT NULL,
           status              TEXT NOT NULL
                               CHECK(status IN ('pending', 'confirmed', 'consumed', 'expired')),
           expires_at          TEXT NOT NULL,
           created_at          TEXT NOT NULL,
           confirmed_at        TEXT,
           consumed_at         TEXT,
           canceled_at         TEXT,
           invocation_id       TEXT,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(capability_id) REFERENCES open_commerce_capabilities(id) ON DELETE CASCADE,
           FOREIGN KEY(grant_id) REFERENCES open_commerce_grants(id) ON DELETE SET NULL,
           FOREIGN KEY(invocation_id) REFERENCES open_commerce_invocations(id) ON DELETE SET NULL
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_action_confirmations_actor
           ON open_commerce_action_confirmations(
             requester_user_id, requester_app_id, status, expires_at
           );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_open_commerce_action_confirmations_invocation
           ON open_commerce_action_confirmations(invocation_id)
           WHERE invocation_id IS NOT NULL;",
    )?;
    Ok(())
}

pub(crate) fn migration_v166(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_action_confirmations",
        "canceled_at",
        "canceled_at TEXT",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent_and_keeps_one_confirmation_per_invocation() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_merchants(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_capabilities(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_grants(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_invocations(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v133(&conn).unwrap();
        migration_v133(&conn).unwrap();
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='table' AND name='open_commerce_action_confirmations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='index' AND name='idx_open_commerce_action_confirmations_invocation'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 1);
        assert_eq!(index_count, 1);
    }

    #[test]
    fn migration_v166_preserves_existing_rows_and_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE open_commerce_action_confirmations (
               id TEXT PRIMARY KEY,
               status TEXT NOT NULL,
               invocation_id TEXT
             );
             INSERT INTO open_commerce_action_confirmations(id, status, invocation_id) VALUES
               ('pending-1', 'pending', NULL),
               ('confirmed-1', 'confirmed', NULL),
               ('consumed-1', 'consumed', 'invocation-1'),
               ('expired-1', 'expired', NULL);",
        )
        .unwrap();

        migration_v166(&conn).unwrap();
        migration_v166(&conn).unwrap();

        let canceled_column_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('open_commerce_action_confirmations')
                 WHERE name='canceled_at'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let existing_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM open_commerce_action_confirmations
                 WHERE canceled_at IS NULL
                   AND ((id='pending-1' AND status='pending' AND invocation_id IS NULL)
                     OR (id='confirmed-1' AND status='confirmed' AND invocation_id IS NULL)
                     OR (id='consumed-1' AND status='consumed' AND invocation_id='invocation-1')
                     OR (id='expired-1' AND status='expired' AND invocation_id IS NULL))",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(canceled_column_count, 1);
        assert_eq!(existing_rows, 4);
    }
}
