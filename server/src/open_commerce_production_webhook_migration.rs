//! Environment-bound developer Webhook subscriptions.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v156(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_developer_webhook_subscriptions",
        "environment",
        "environment TEXT NOT NULL DEFAULT 'sandbox' CHECK(environment IN ('sandbox', 'production'))",
    )?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_open_commerce_webhook_subscription_environment
           ON open_commerce_developer_webhook_subscriptions(
             project_id, app_record_id, environment, status, updated_at
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
              AND (
                (subscription.environment = 'sandbox'
                 AND invocation.credential_environment IN ('legacy', 'sandbox'))
                OR
                (subscription.environment = 'production'
                 AND invocation.credential_environment = 'production'
                 AND EXISTS(
                   SELECT 1
                     FROM open_commerce_developer_production_credentials credential
                     JOIN open_commerce_developer_apps credential_app
                       ON credential_app.id = credential.app_record_id
                     JOIN open_commerce_developer_app_admissions admission
                       ON admission.id = credential.admission_id
                    WHERE credential.app_record_id = subscription.app_record_id
                      AND credential.project_id = subscription.project_id
                      AND credential.status = 'active'
                      AND julianday(credential.expires_at) > julianday('now')
                      AND credential_app.project_id = subscription.project_id
                      AND credential_app.status = 'active'
                      AND credential_app.manifest_status = 'approved'
                      AND credential_app.manifest_revision = credential.manifest_revision
                      AND credential_app.domain_verification_status = 'verified'
                      AND credential_app.domain_verification_revision = credential.manifest_revision
                      AND admission.app_record_id = subscription.app_record_id
                      AND admission.project_id = subscription.project_id
                      AND admission.status = 'approved'
                      AND admission.manifest_revision = credential.manifest_revision
                 ))
              )
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
