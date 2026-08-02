//! Durable terminal invocation sequence for developer App result polling.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v134(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_invocation_terminal_events (
           seq           INTEGER PRIMARY KEY AUTOINCREMENT,
           invocation_id TEXT NOT NULL UNIQUE,
           recorded_at   TEXT NOT NULL,
           FOREIGN KEY(invocation_id) REFERENCES open_commerce_invocations(id) ON DELETE CASCADE
         );

         INSERT OR IGNORE INTO open_commerce_invocation_terminal_events(invocation_id, recorded_at)
         SELECT id, COALESCE(completed_at, created_at)
           FROM open_commerce_invocations
          WHERE status IN ('succeeded', 'failed')
          ORDER BY COALESCE(completed_at, created_at), id;

         CREATE TRIGGER IF NOT EXISTS trg_open_commerce_invocation_terminal_update
         AFTER UPDATE OF status ON open_commerce_invocations
         WHEN OLD.status = 'started' AND NEW.status IN ('succeeded', 'failed')
         BEGIN
           INSERT OR IGNORE INTO open_commerce_invocation_terminal_events(invocation_id, recorded_at)
           VALUES (NEW.id, COALESCE(NEW.completed_at, NEW.created_at));
         END;

         CREATE TRIGGER IF NOT EXISTS trg_open_commerce_invocation_terminal_insert
         AFTER INSERT ON open_commerce_invocations
         WHEN NEW.status IN ('succeeded', 'failed')
         BEGIN
           INSERT OR IGNORE INTO open_commerce_invocation_terminal_events(invocation_id, recorded_at)
           VALUES (NEW.id, COALESCE(NEW.completed_at, NEW.created_at));
         END;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_backfills_and_sequences_terminal_transitions_once() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE open_commerce_invocations(
               id TEXT PRIMARY KEY,
               status TEXT NOT NULL,
               created_at TEXT NOT NULL,
               completed_at TEXT
             );
             INSERT INTO open_commerce_invocations VALUES
               ('done-before', 'succeeded', '2026-01-01T00:00:00Z', '2026-01-01T00:01:00Z'),
               ('still-running', 'started', '2026-01-01T00:02:00Z', NULL);",
        )
        .unwrap();

        migration_v134(&conn).unwrap();
        assert_eq!(event_count(&conn), 1);

        conn.execute(
            "UPDATE open_commerce_invocations
                SET status='failed', completed_at='2026-01-01T00:03:00Z'
              WHERE id='still-running'",
            [],
        )
        .unwrap();
        assert_eq!(event_count(&conn), 2);

        conn.execute(
            "UPDATE open_commerce_invocations SET completed_at=completed_at
              WHERE id='still-running'",
            [],
        )
        .unwrap();
        assert_eq!(event_count(&conn), 2);
    }

    fn event_count(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM open_commerce_invocation_terminal_events",
            [],
            |row| row.get(0),
        )
        .unwrap()
    }
}
