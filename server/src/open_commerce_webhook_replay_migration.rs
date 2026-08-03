//! Manual dead-letter retry metadata for developer webhooks.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v148(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_deliveries",
        "manual_retry_count",
        "manual_retry_count INTEGER NOT NULL DEFAULT 0",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_deliveries",
        "last_manual_retry_at",
        "last_manual_retry_at TEXT",
    )?;
    Ok(())
}
