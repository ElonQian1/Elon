//! Subscription-level terminal-event filters for developer webhooks.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v149(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_subscriptions",
        "deliver_on_succeeded",
        "deliver_on_succeeded INTEGER NOT NULL DEFAULT 1 CHECK(deliver_on_succeeded IN (0, 1))",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_subscriptions",
        "deliver_on_failed",
        "deliver_on_failed INTEGER NOT NULL DEFAULT 1 CHECK(deliver_on_failed IN (0, 1))",
    )?;
    conn.execute_batch(
        "DROP TRIGGER IF EXISTS trg_open_commerce_terminal_event_webhook;
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
