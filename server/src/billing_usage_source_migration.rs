use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v85(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "token_usage_events",
        "billing_source",
        "billing_source TEXT NOT NULL DEFAULT 'platform'",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "token_usage_events",
        "resource_owner_user_id",
        "resource_owner_user_id TEXT",
    )?;
    conn.execute_batch(
        r#"
        UPDATE token_usage_events
           SET billing_source = CASE
             WHEN usage_mode = 'client_reported' THEN 'client_reported'
             WHEN usage_mode = 'user_api_key_proxy' THEN 'user_api_key'
             ELSE COALESCE(NULLIF(TRIM(billing_source), ''), 'platform')
           END
         WHERE billing_source IS NULL
            OR TRIM(billing_source) = ''
            OR (
              billing_source = 'platform'
              AND usage_mode IN ('client_reported', 'user_api_key_proxy')
            );

        CREATE INDEX IF NOT EXISTS idx_token_usage_billing_source
          ON token_usage_events(user_id, billing_source, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_token_usage_resource_owner
          ON token_usage_events(resource_owner_user_id, created_at DESC)
          WHERE resource_owner_user_id IS NOT NULL;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "billing_usage_source_migration_tests.rs"]
mod tests;
