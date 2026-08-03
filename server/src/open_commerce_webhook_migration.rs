//! Durable, App-scoped delivery queue for developer terminal-event webhooks.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v145(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_developer_webhook_subscriptions (
           id                   TEXT PRIMARY KEY,
           project_id           TEXT NOT NULL,
           owner_user_id        TEXT NOT NULL,
           app_record_id        TEXT NOT NULL,
           app_id               TEXT NOT NULL,
           callback_url         TEXT NOT NULL,
           signing_key_id       TEXT NOT NULL,
           status               TEXT NOT NULL CHECK(status IN ('active', 'disabled')),
           start_sequence       INTEGER NOT NULL CHECK(start_sequence >= 0),
           consecutive_failures INTEGER NOT NULL DEFAULT 0 CHECK(consecutive_failures >= 0),
           last_delivery_at     TEXT,
           last_error_code      TEXT,
           created_at           TEXT NOT NULL,
           updated_at           TEXT NOT NULL,
           disabled_at          TEXT,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(app_record_id) REFERENCES open_commerce_developer_apps(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_webhook_subscription_app
           ON open_commerce_developer_webhook_subscriptions(
             project_id, app_record_id, status, updated_at
           );

         CREATE TABLE IF NOT EXISTS open_commerce_developer_webhook_deliveries (
           id                TEXT PRIMARY KEY,
           subscription_id   TEXT NOT NULL,
           invocation_id     TEXT NOT NULL,
           event_sequence    INTEGER NOT NULL CHECK(event_sequence > 0),
           event_type        TEXT NOT NULL
                             CHECK(event_type IN ('invocation.succeeded', 'invocation.failed')),
           status            TEXT NOT NULL
                             CHECK(status IN ('pending', 'delivering', 'retry', 'delivered', 'dead')),
           attempt_count     INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count >= 0),
           next_attempt_at   TEXT NOT NULL,
           lease_owner       TEXT,
           lease_expires_at  TEXT,
           response_status   INTEGER,
           error_code        TEXT,
           created_at        TEXT NOT NULL,
           last_attempt_at   TEXT,
           delivered_at      TEXT,
           UNIQUE(subscription_id, invocation_id),
           FOREIGN KEY(subscription_id)
             REFERENCES open_commerce_developer_webhook_subscriptions(id) ON DELETE CASCADE,
           FOREIGN KEY(invocation_id) REFERENCES open_commerce_invocations(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_webhook_delivery_due
           ON open_commerce_developer_webhook_deliveries(
             status, next_attempt_at, lease_expires_at, event_sequence
           );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_webhook_delivery_subscription
           ON open_commerce_developer_webhook_deliveries(
             subscription_id, created_at DESC
           );

         CREATE TRIGGER IF NOT EXISTS trg_open_commerce_terminal_event_webhook
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
            WHERE invocation.id = NEW.invocation_id;
         END;",
    )?;
    Ok(())
}
