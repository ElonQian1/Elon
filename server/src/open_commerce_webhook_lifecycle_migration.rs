//! Signing-secret lifecycle metadata for developer webhooks.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v147(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_subscriptions",
        "signing_secret_version",
        "signing_secret_version INTEGER NOT NULL DEFAULT 1",
    )?;
    conn.execute(
        "UPDATE open_commerce_developer_webhook_subscriptions
            SET signing_secret_version=1
          WHERE signing_secret_version IS NULL OR signing_secret_version < 1",
        [],
    )?;
    Ok(())
}
