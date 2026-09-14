//! Runs the actual revision SQL, migration and tests without linking the full server.
//! The facade supplies connections and time; HTTP authentication and the complete
//! migration catalogue remain covered by the server build/integration checks.
#![allow(dead_code)]

use anyhow::Result;
use rusqlite::Connection;

pub(crate) struct Store {
    path: std::path::PathBuf,
}
impl Store {
    fn conn(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        Ok(conn)
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[path = "../../../src/store/group_message_revisions.rs"]
mod revisions;

#[path = "../../../src/store_migrations/group_message_revisions.rs"]
mod migration;

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
        [table, column],
        |r| r.get(0),
    )?;
    if !exists {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {definition}"), [])?;
    }
    Ok(())
}

mod store_migrations {
    pub(crate) static MIGRATIONS: &[(
        u32,
        &str,
        fn(&rusqlite::Connection) -> anyhow::Result<()>,
    )] = &[(295, "群聊消息追加式修订历史", super::migration::migrate)];
}
