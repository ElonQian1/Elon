use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::open_commerce_webhook_health_model::DeveloperWebhookEnvironmentHealth;

use super::Store;

impl Store {
    pub(crate) fn open_commerce_developer_webhook_environment_health(
        &self,
        project_id: &str,
        app_record_id: &str,
    ) -> Result<Vec<DeveloperWebhookEnvironmentHealth>> {
        let conn = self.conn()?;
        ["sandbox", "production"]
            .into_iter()
            .map(|environment| environment_health_on(&conn, project_id, app_record_id, environment))
            .collect()
    }
}

fn environment_health_on(
    conn: &Connection,
    project_id: &str,
    app_record_id: &str,
    environment: &str,
) -> Result<DeveloperWebhookEnvironmentHealth> {
    let (
        subscription_count,
        active_subscription_count,
        verified_subscription_count,
        latest_subscription_delivery_at,
    ): (i64, i64, i64, Option<String>) = conn.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(CASE WHEN status='active' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN verification_status='verified' THEN 1 ELSE 0 END), 0),
                MAX(last_delivery_at)
           FROM open_commerce_developer_webhook_subscriptions
          WHERE project_id=?1 AND app_record_id=?2 AND environment=?3",
        params![project_id.trim(), app_record_id.trim(), environment],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let (
        pending_delivery_count,
        retry_delivery_count,
        delivering_delivery_count,
        unresolved_dead_delivery_count,
        acknowledged_dead_delivery_count,
        oldest_queued_at,
        latest_delivery_at,
    ): (i64, i64, i64, i64, i64, Option<String>, Option<String>) = conn.query_row(
        "SELECT
            COALESCE(SUM(CASE WHEN delivery.status='pending' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN delivery.status='retry' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN delivery.status='delivering' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN delivery.status='dead'
                               AND delivery.dead_letter_acknowledged_at IS NULL
                              THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN delivery.status='dead'
                               AND delivery.dead_letter_acknowledged_at IS NOT NULL
                              THEN 1 ELSE 0 END), 0),
            MIN(CASE WHEN delivery.status IN ('pending', 'retry', 'delivering')
                     THEN delivery.created_at END),
            MAX(delivery.delivered_at)
           FROM open_commerce_developer_webhook_deliveries delivery
           JOIN open_commerce_developer_webhook_subscriptions subscription
             ON subscription.id=delivery.subscription_id
          WHERE subscription.project_id=?1 AND subscription.app_record_id=?2
            AND subscription.environment=?3",
        params![project_id.trim(), app_record_id.trim(), environment],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )?;
    let latest_delivery_error = conn
        .query_row(
            "SELECT delivery.error_code
               FROM open_commerce_developer_webhook_deliveries delivery
               JOIN open_commerce_developer_webhook_subscriptions subscription
                 ON subscription.id=delivery.subscription_id
              WHERE subscription.project_id=?1 AND subscription.app_record_id=?2
                AND subscription.environment=?3 AND delivery.error_code IS NOT NULL
              ORDER BY COALESCE(delivery.last_attempt_at, delivery.created_at) DESC
              LIMIT 1",
            params![project_id.trim(), app_record_id.trim(), environment],
            |row| row.get(0),
        )
        .optional()?;
    let latest_subscription_error = conn
        .query_row(
            "SELECT COALESCE(verification_error_code, last_error_code)
               FROM open_commerce_developer_webhook_subscriptions
              WHERE project_id=?1 AND app_record_id=?2 AND environment=?3
                AND COALESCE(verification_error_code, last_error_code) IS NOT NULL
              ORDER BY updated_at DESC LIMIT 1",
            params![project_id.trim(), app_record_id.trim(), environment],
            |row| row.get(0),
        )
        .optional()?;
    Ok(DeveloperWebhookEnvironmentHealth {
        environment: environment.to_string(),
        status: String::new(),
        subscription_count,
        active_subscription_count,
        verified_subscription_count,
        pending_delivery_count,
        retry_delivery_count,
        delivering_delivery_count,
        unresolved_dead_delivery_count,
        acknowledged_dead_delivery_count,
        oldest_queued_at,
        latest_delivery_at: latest_delivery_at.or(latest_subscription_delivery_at),
        latest_error_code: latest_delivery_error.or(latest_subscription_error),
    })
}
