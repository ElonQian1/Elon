use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v132(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_open_commerce_invocations_requester_time
           ON open_commerce_invocations(requester_user_id, status, created_at DESC);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_adds_requester_receipt_index() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE open_commerce_invocations(
                requester_user_id TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL
             );",
        )
        .unwrap();
        migration_v132(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='index' AND name='idx_open_commerce_invocations_requester_time'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
