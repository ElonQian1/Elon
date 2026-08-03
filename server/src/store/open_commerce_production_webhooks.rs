//! Production Webhook cleanup when the owning credential is no longer usable.

use anyhow::Result;
use rusqlite::{params, Transaction};

pub(super) fn disable_unusable_production_webhooks_on(
    tx: &Transaction<'_>,
    timestamp: &str,
) -> Result<()> {
    tx.execute(
        "UPDATE open_commerce_developer_webhook_deliveries
            SET status='dead', error_code='production_credential_unavailable',
                lease_owner=NULL, lease_expires_at=NULL
          WHERE subscription_id IN (
                SELECT subscription.id
                  FROM open_commerce_developer_webhook_subscriptions subscription
                 WHERE subscription.environment='production'
                   AND subscription.status='active'
                   AND NOT EXISTS(
                     SELECT 1
                       FROM open_commerce_developer_production_credentials credential
                       JOIN open_commerce_developer_apps app
                         ON app.id=credential.app_record_id
                       JOIN open_commerce_developer_app_admissions admission
                         ON admission.id=credential.admission_id
                      WHERE credential.app_record_id=subscription.app_record_id
                        AND credential.project_id=subscription.project_id
                        AND credential.status='active'
                        AND julianday(credential.expires_at) > julianday(?1)
                        AND app.project_id=subscription.project_id
                        AND app.status='active' AND app.manifest_status='approved'
                        AND app.manifest_revision=credential.manifest_revision
                        AND app.domain_verification_status='verified'
                        AND app.domain_verification_revision=credential.manifest_revision
                        AND admission.app_record_id=subscription.app_record_id
                        AND admission.project_id=subscription.project_id
                        AND admission.status='approved'
                        AND admission.manifest_revision=credential.manifest_revision
                   )
          ) AND status IN ('pending', 'retry', 'delivering')",
        params![timestamp],
    )?;
    tx.execute(
        "UPDATE open_commerce_developer_webhook_subscriptions
            SET status='disabled', last_error_code='production_credential_unavailable',
                updated_at=?1, disabled_at=?1
          WHERE environment='production' AND status='active'
            AND NOT EXISTS(
              SELECT 1
                FROM open_commerce_developer_production_credentials credential
                JOIN open_commerce_developer_apps app
                  ON app.id=credential.app_record_id
                JOIN open_commerce_developer_app_admissions admission
                  ON admission.id=credential.admission_id
               WHERE credential.app_record_id=open_commerce_developer_webhook_subscriptions.app_record_id
                 AND credential.project_id=open_commerce_developer_webhook_subscriptions.project_id
                 AND credential.status='active'
                 AND julianday(credential.expires_at) > julianday(?1)
                 AND app.project_id=open_commerce_developer_webhook_subscriptions.project_id
                 AND app.status='active' AND app.manifest_status='approved'
                 AND app.manifest_revision=credential.manifest_revision
                 AND app.domain_verification_status='verified'
                 AND app.domain_verification_revision=credential.manifest_revision
                 AND admission.app_record_id=open_commerce_developer_webhook_subscriptions.app_record_id
                 AND admission.project_id=open_commerce_developer_webhook_subscriptions.project_id
                 AND admission.status='approved'
                 AND admission.manifest_revision=credential.manifest_revision
            )",
        params![timestamp],
    )?;
    Ok(())
}
