//! Auditable operator acknowledgement for developer Webhook dead letters.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v157(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_deliveries",
        "dead_letter_acknowledged_at",
        "dead_letter_acknowledged_at TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_deliveries",
        "dead_letter_acknowledged_by_user_id",
        "dead_letter_acknowledged_by_user_id TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_deliveries",
        "dead_letter_acknowledgement_reason",
        "dead_letter_acknowledgement_reason TEXT",
    )?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_open_commerce_webhook_unresolved_dead_letter
           ON open_commerce_developer_webhook_deliveries(
             status, dead_letter_acknowledged_at, last_attempt_at
           );",
    )?;
    Ok(())
}
