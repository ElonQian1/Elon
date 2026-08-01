//! Durable token-budget reservation for shared node inference admission.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v126(conn: &Connection) -> Result<()> {
    let exists = conn
        .prepare("PRAGMA table_info(node_compute_runs)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|name| name == "reserved_token_budget");
    if !exists {
        conn.execute_batch(
            "ALTER TABLE node_compute_runs
               ADD COLUMN reserved_token_budget INTEGER NOT NULL DEFAULT 0
                 CHECK(reserved_token_budget BETWEEN 0 AND 1000000000000);",
        )?;
    }
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_node_compute_runs_shared_budget
           ON node_compute_runs(node_id, usage_mode, status, updated_at, reserved_token_budget);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_idempotent_fail_closed_token_budget_reservation() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE node_compute_runs (
               id TEXT PRIMARY KEY,
               node_id TEXT NOT NULL,
               usage_mode TEXT NOT NULL,
               status TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );",
        )
        .unwrap();

        migration_v126(&conn).unwrap();
        migration_v126(&conn).unwrap();

        let columns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('node_compute_runs')
                  WHERE name='reserved_token_budget'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(columns, 1);
        let default_value: String = conn
            .query_row(
                "SELECT dflt_value FROM pragma_table_info('node_compute_runs')
                  WHERE name='reserved_token_budget'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(default_value, "0");
    }
}
