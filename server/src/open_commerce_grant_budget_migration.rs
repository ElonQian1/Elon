//! Optional lifetime budgets and per-invocation reservations for Grants.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v119(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "ALTER TABLE open_commerce_grants
           ADD COLUMN max_invocations INTEGER
           CHECK(max_invocations IS NULL OR max_invocations > 0);
         ALTER TABLE open_commerce_grants
           ADD COLUMN max_amount_micros INTEGER
           CHECK(max_amount_micros IS NULL OR max_amount_micros > 0);
         ALTER TABLE open_commerce_grants
           ADD COLUMN budget_currency TEXT NOT NULL DEFAULT 'CNY';
         ALTER TABLE open_commerce_grants
           ADD COLUMN used_invocations INTEGER NOT NULL DEFAULT 0
           CHECK(used_invocations >= 0);
         ALTER TABLE open_commerce_grants
           ADD COLUMN used_amount_micros INTEGER NOT NULL DEFAULT 0
           CHECK(used_amount_micros >= 0);

         CREATE TABLE open_commerce_grant_budget_reservations (
           invocation_id       TEXT PRIMARY KEY,
           grant_id            TEXT NOT NULL,
           reserved_invocations INTEGER NOT NULL DEFAULT 1
                                CHECK(reserved_invocations = 1),
           reserved_amount_micros INTEGER NOT NULL
                                  CHECK(reserved_amount_micros >= 0),
           currency            TEXT NOT NULL,
           status              TEXT NOT NULL DEFAULT 'reserved'
                               CHECK(status IN ('reserved', 'committed', 'released')),
           created_at          TEXT NOT NULL,
           completed_at        TEXT,
           FOREIGN KEY(invocation_id) REFERENCES open_commerce_invocations(id) ON DELETE CASCADE,
           FOREIGN KEY(grant_id) REFERENCES open_commerce_grants(id) ON DELETE CASCADE
         );
         CREATE INDEX idx_open_commerce_grant_budget_reservations_grant
           ON open_commerce_grant_budget_reservations(grant_id, status, created_at DESC);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_adds_grant_limits_and_reservations() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE open_commerce_grants(id TEXT PRIMARY KEY);
             CREATE TABLE open_commerce_invocations(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v119(&conn).unwrap();
        let columns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('open_commerce_grants')
                 WHERE name IN (
                   'max_invocations', 'max_amount_micros', 'budget_currency',
                   'used_invocations', 'used_amount_micros'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(columns, 5);
        let reservations: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'open_commerce_grant_budget_reservations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(reservations, 1);
    }
}
