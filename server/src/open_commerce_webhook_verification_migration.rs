//! Endpoint-control verification metadata for developer webhooks.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v146(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_subscriptions",
        "verification_status",
        "verification_status TEXT NOT NULL DEFAULT 'verified'",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_subscriptions",
        "verification_attempted_at",
        "verification_attempted_at TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_subscriptions",
        "verification_error_code",
        "verification_error_code TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_subscriptions",
        "verified_at",
        "verified_at TEXT",
    )?;
    conn.execute(
        "UPDATE open_commerce_developer_webhook_subscriptions
            SET verification_status='verified', verified_at=COALESCE(verified_at, created_at)
          WHERE verification_status='verified' AND verified_at IS NULL",
        [],
    )?;
    Ok(())
}
