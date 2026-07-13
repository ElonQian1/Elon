use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v99(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "project_execution_sessions",
        "task_id",
        "task_id TEXT",
    )?;

    crate::store_migrations::add_column_if_missing(
        conn,
        "node_compute_runs",
        "billing_source",
        "billing_source TEXT NOT NULL DEFAULT 'platform'",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "node_compute_runs",
        "resource_owner_user_id",
        "resource_owner_user_id TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "node_compute_runs",
        "lease_id",
        "lease_id TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "node_compute_runs",
        "offline_policy",
        "offline_policy TEXT NOT NULL DEFAULT 'online_only'",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "node_compute_runs",
        "replay_deadline",
        "replay_deadline TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "node_compute_runs",
        "max_cost_rmb_fen",
        "max_cost_rmb_fen INTEGER NOT NULL DEFAULT 0",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "node_compute_runs",
        "allowance_id",
        "allowance_id TEXT",
    )?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS node_cli_completion_receipts (
          event_id                  TEXT PRIMARY KEY,
          req_id                    TEXT NOT NULL UNIQUE,
          compute_call_id           TEXT NOT NULL UNIQUE,
          node_id                   TEXT NOT NULL,
          user_id                   TEXT NOT NULL,
          payload_json              TEXT NOT NULL,
          payload_sha256            TEXT NOT NULL,
          status                    TEXT NOT NULL DEFAULT 'pending'
                                    CHECK (status IN ('pending', 'processing', 'applied', 'retry', 'rejected')),
          token_usage_event_id      TEXT,
          billing_event_id          TEXT,
          node_transaction_id       TEXT,
          reason                    TEXT,
          attempt_count             INTEGER NOT NULL DEFAULT 0,
          received_at               TEXT NOT NULL,
          updated_at                TEXT NOT NULL,
          last_attempt_at           TEXT,
          applied_at                TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_node_cli_completion_receipts_status
          ON node_cli_completion_receipts(status, updated_at);
        CREATE INDEX IF NOT EXISTS idx_node_cli_completion_receipts_node
          ON node_cli_completion_receipts(node_id, received_at DESC);
        CREATE INDEX IF NOT EXISTS idx_node_cli_completion_receipts_user
          ON node_cli_completion_receipts(user_id, received_at DESC);

        CREATE INDEX IF NOT EXISTS idx_project_execution_sessions_task
          ON project_execution_sessions(task_id, updated_at DESC)
          WHERE task_id IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_node_compute_runs_replay_policy
          ON node_compute_runs(offline_policy, replay_deadline, status);
        CREATE INDEX IF NOT EXISTS idx_node_compute_runs_lease
          ON node_compute_runs(lease_id, updated_at DESC)
          WHERE lease_id IS NOT NULL;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::migration_v99;
    use rusqlite::Connection;

    #[test]
    fn migration_creates_durable_completion_inbox_and_replay_bindings() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        crate::store_schema::apply_migrations(&conn).expect("schema should apply");
        migration_v99(&conn).expect("offline completion migration should be idempotent");

        let receipt_table: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                  WHERE type = 'table' AND name = 'node_cli_completion_receipts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(receipt_table, 1);
        assert!(has_column(&conn, "project_execution_sessions", "task_id"));
        for column in [
            "billing_source",
            "resource_owner_user_id",
            "lease_id",
            "offline_policy",
            "replay_deadline",
            "max_cost_rmb_fen",
            "allowance_id",
        ] {
            assert!(has_column(&conn, "node_compute_runs", column), "{column}");
        }
    }

    fn has_column(conn: &Connection, table: &str, expected: &str) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("table info should prepare");
        let found = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .expect("table info should query")
            .filter_map(Result::ok)
            .any(|column| column == expected);
        found
    }
}
