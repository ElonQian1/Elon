//! Provenance metadata for bounded developer-webhook history replay.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v150(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_deliveries",
        "enqueue_source",
        "enqueue_source TEXT NOT NULL DEFAULT 'live' CHECK(enqueue_source IN ('live', 'history_replay'))",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_deliveries",
        "history_replay_requested_at",
        "history_replay_requested_at TEXT",
    )?;
    Ok(())
}
