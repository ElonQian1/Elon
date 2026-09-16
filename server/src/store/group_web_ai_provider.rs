//! Keep the legacy device-execution engine for old clients; bind the actual provider separately.
use anyhow::{ensure, Result};
use rusqlite::Connection;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "group_ai_reply_requests",
        "web_provider",
        "web_provider TEXT NOT NULL DEFAULT 'chatgpt_web' CHECK(web_provider IN ('chatgpt_web','google_web'))",
    )
}

pub(crate) fn validate(provider: &str) -> Result<()> {
    ensure!(
        matches!(provider, "chatgpt_web" | "google_web"),
        "不支持的群聊网页 AI"
    );
    Ok(())
}
