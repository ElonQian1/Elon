//! Executes the production selection SQL and migration without linking unrelated server modules.
#![allow(dead_code)]
use anyhow::Result;
use rusqlite::Connection;

pub(crate) struct Store {
    connection: std::sync::Mutex<Connection>,
}
impl Store {
    fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        // Match the production single-connection lock, but fail instead of hanging a test.
        self.connection
            .try_lock()
            .map_err(|_| anyhow::anyhow!("database lock already held"))
    }
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4())
}

#[path = "../../../src/store/group_ai_source_access.rs"]
mod source;
use source::ensure_member_and_source;
#[path = "../../../src/store/group_ai_reply_context.rs"]
mod context;
#[path = "../../../src/store/group_ai_context_share.rs"]
mod context_share;
mod store {
    pub struct FriendGroupMessage {
        pub id: String,
        pub recalled_at: Option<String>,
        pub ai_reply: Option<serde_json::Value>,
    }
}
#[path = "../../../src/store/group_ai_selection_schema.rs"]
mod migration;
#[path = "../../../src/store/group_web_ai_selection.rs"]
mod selection;
#[path = "../../../src/store/group_web_ai_requests.rs"]
mod web;

// The web action module shares the production provider validator, whose migration
// needs only this schema helper normally supplied by the server's store module.
#[path = "../../../src/store/group_web_ai_provider.rs"]
mod provider;

pub(crate) fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let found: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name=?2)",
        [table, column],
        |r| r.get(0),
    )?;
    if !found {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {definition}"), [])?;
    }
    Ok(())
}
mod store_migrations {
    pub(crate) use super::add_column_if_missing;
}

#[cfg(test)]
mod tests;
