//! Project-scoped policy for selecting existing AI resource sources.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v112(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS project_ai_resource_policies (
           project_id                          TEXT PRIMARY KEY,
           enabled_classes_json                TEXT NOT NULL,
           priority_json                       TEXT NOT NULL,
           allow_fallback                      INTEGER NOT NULL DEFAULT 1,
           privacy_mode                        TEXT NOT NULL DEFAULT 'prefer_local'
                                               CHECK(privacy_mode IN (
                                                 'prefer_local',
                                                 'balanced',
                                                 'prefer_available'
                                               )),
           max_estimated_unit_cost_micros     INTEGER,
           updated_by_user_id                  TEXT NOT NULL,
           created_at                          TEXT NOT NULL,
           updated_at                          TEXT NOT NULL,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(updated_by_user_id) REFERENCES users(id) ON DELETE CASCADE,
           CHECK(max_estimated_unit_cost_micros IS NULL
                 OR max_estimated_unit_cost_micros >= 0)
         );",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_project_policy_table() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v112(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'project_ai_resource_policies'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
