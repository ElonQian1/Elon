//! Invocation credential provenance and fail-closed production webhook isolation.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v155(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_invocations",
        "credential_environment",
        "credential_environment TEXT NOT NULL DEFAULT 'legacy' CHECK(credential_environment IN ('legacy', 'platform', 'sandbox', 'production'))",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_invocations",
        "credential_id",
        "credential_id TEXT",
    )?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_open_commerce_invocations_app_environment_time
           ON open_commerce_invocations(
             requester_user_id, requester_app_id, credential_environment, created_at DESC
           );

         DROP TRIGGER IF EXISTS trg_open_commerce_terminal_event_webhook;
         CREATE TRIGGER trg_open_commerce_terminal_event_webhook
         AFTER INSERT ON open_commerce_invocation_terminal_events
         BEGIN
           INSERT OR IGNORE INTO open_commerce_developer_webhook_deliveries(
             id, subscription_id, invocation_id, event_sequence, event_type,
             status, attempt_count, next_attempt_at, created_at
           )
           SELECT 'webhook_delivery:' || subscription.id || ':' || NEW.invocation_id,
                  subscription.id,
                  NEW.invocation_id,
                  NEW.seq,
                  CASE invocation.status
                    WHEN 'succeeded' THEN 'invocation.succeeded'
                    ELSE 'invocation.failed'
                  END,
                  'pending',
                  0,
                  NEW.recorded_at,
                  NEW.recorded_at
             FROM open_commerce_invocations invocation
             JOIN open_commerce_developer_webhook_subscriptions subscription
               ON subscription.owner_user_id = invocation.requester_user_id
              AND subscription.app_id = invocation.requester_app_id
              AND subscription.status = 'active'
              AND NEW.seq > subscription.start_sequence
              AND invocation.credential_environment IN ('legacy', 'platform', 'sandbox')
              AND (
                (invocation.status = 'succeeded' AND subscription.deliver_on_succeeded = 1)
                OR
                (invocation.status <> 'succeeded' AND subscription.deliver_on_failed = 1)
              )
            WHERE invocation.id = NEW.invocation_id;
         END;",
    )?;
    Ok(())
}
