use anyhow::{anyhow, bail, Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::{
    open_commerce_developer_model::OpenCommerceDeveloperApp,
    open_commerce_webhook_model::{
        DeveloperWebhookDelivery, DeveloperWebhookDeliveryClaim, DeveloperWebhookSubscription,
        DEVELOPER_WEBHOOK_DELIVERY_SCHEMA, DEVELOPER_WEBHOOK_SUBSCRIPTION_SCHEMA,
    },
};

use super::{new_id, now, Store};

const SUBSCRIPTION_SELECT: &str =
    "SELECT id, project_id, app_record_id, app_id, callback_url, signing_key_id,
            signing_secret_version, status, verification_status, verification_attempted_at,
            verification_error_code, verified_at, consecutive_failures,
            last_delivery_at, last_error_code, created_at, updated_at, disabled_at
       FROM open_commerce_developer_webhook_subscriptions";

pub(super) const DELIVERY_SELECT: &str =
    "SELECT id, subscription_id, invocation_id, event_sequence, event_type,
            status, attempt_count, manual_retry_count, next_attempt_at,
            response_status, error_code, created_at, last_attempt_at,
            last_manual_retry_at, delivered_at
       FROM open_commerce_developer_webhook_deliveries";

impl Store {
    pub(crate) fn create_open_commerce_developer_webhook(
        &self,
        app: &OpenCommerceDeveloperApp,
        callback_url: &str,
        signing_key_id: &str,
    ) -> Result<DeveloperWebhookSubscription> {
        let id = new_id("devhook");
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let total_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM open_commerce_developer_webhook_subscriptions
              WHERE project_id=?1 AND app_record_id=?2",
            params![app.project_id, app.id],
            |row| row.get(0),
        )?;
        if total_count >= 20 {
            bail!("每个开发者 App 最多保留 20 个 Webhook 订阅记录");
        }
        let active_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM open_commerce_developer_webhook_subscriptions
              WHERE project_id=?1 AND app_record_id=?2 AND status='active'",
            params![app.project_id, app.id],
            |row| row.get(0),
        )?;
        if active_count >= 5 {
            bail!("每个开发者 App 最多创建 5 个 Webhook 订阅");
        }
        let start_sequence: i64 = tx.query_row(
            "SELECT COALESCE(MAX(seq), 0) FROM open_commerce_invocation_terminal_events",
            [],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT INTO open_commerce_developer_webhook_subscriptions(
               id, project_id, owner_user_id, app_record_id, app_id,
               callback_url, signing_key_id, signing_secret_version, status, verification_status,
               start_sequence, consecutive_failures, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 'disabled', 'pending', ?8, 0, ?9, ?9)",
            params![
                id,
                app.project_id,
                app.owner_user_id,
                app.id,
                app.app_id,
                callback_url.trim(),
                signing_key_id.trim(),
                start_sequence,
                timestamp
            ],
        )?;
        tx.commit()?;
        self.open_commerce_developer_webhook_for_app(&app.project_id, &app.id, &id)
    }

    pub(crate) fn list_open_commerce_developer_webhooks(
        &self,
        project_id: &str,
        app_record_id: &str,
    ) -> Result<Vec<DeveloperWebhookSubscription>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "{SUBSCRIPTION_SELECT}
              WHERE project_id=?1 AND app_record_id=?2
              ORDER BY created_at DESC"
        ))?;
        Ok(statement
            .query_map(
                params![project_id.trim(), app_record_id.trim()],
                subscription_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn open_commerce_developer_webhook_for_app(
        &self,
        project_id: &str,
        app_record_id: &str,
        subscription_id: &str,
    ) -> Result<DeveloperWebhookSubscription> {
        self.conn()?
            .query_row(
                &format!(
                    "{SUBSCRIPTION_SELECT}
                      WHERE project_id=?1 AND app_record_id=?2 AND id=?3"
                ),
                params![
                    project_id.trim(),
                    app_record_id.trim(),
                    subscription_id.trim()
                ],
                subscription_from_row,
            )
            .map_err(|error| anyhow!(error).context("Webhook 订阅不存在"))
    }

    pub(crate) fn set_open_commerce_developer_webhook_enabled(
        &self,
        project_id: &str,
        app_record_id: &str,
        subscription_id: &str,
        enabled: bool,
    ) -> Result<DeveloperWebhookSubscription> {
        let current = self.open_commerce_developer_webhook_for_app(
            project_id,
            app_record_id,
            subscription_id,
        )?;
        if (enabled && current.status == "active") || (!enabled && current.status == "disabled") {
            return Ok(current);
        }
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if enabled {
            let app_status: String = tx.query_row(
                "SELECT status FROM open_commerce_developer_apps
                  WHERE project_id=?1 AND id=?2",
                params![project_id.trim(), app_record_id.trim()],
                |row| row.get(0),
            )?;
            if app_status != "active" {
                bail!("开发者 App 已停用，不能启用 Webhook");
            }
            let verification_status: String = tx.query_row(
                "SELECT verification_status
                   FROM open_commerce_developer_webhook_subscriptions
                  WHERE project_id=?1 AND app_record_id=?2 AND id=?3",
                params![
                    project_id.trim(),
                    app_record_id.trim(),
                    subscription_id.trim()
                ],
                |row| row.get(0),
            )?;
            if verification_status != "verified" {
                bail!("Webhook 回调端点尚未验证");
            }
            let active_count: i64 = tx.query_row(
                "SELECT COUNT(*) FROM open_commerce_developer_webhook_subscriptions
                  WHERE project_id=?1 AND app_record_id=?2 AND status='active' AND id<>?3",
                params![
                    project_id.trim(),
                    app_record_id.trim(),
                    subscription_id.trim()
                ],
                |row| row.get(0),
            )?;
            if active_count >= 5 {
                bail!("每个开发者 App 最多同时启用 5 个 Webhook 订阅");
            }
            let start_sequence: i64 = tx.query_row(
                "SELECT COALESCE(MAX(seq), 0) FROM open_commerce_invocation_terminal_events",
                [],
                |row| row.get(0),
            )?;
            tx.execute(
                "UPDATE open_commerce_developer_webhook_subscriptions
                    SET status='active', start_sequence=?1, consecutive_failures=0,
                        last_error_code=NULL, updated_at=?2, disabled_at=NULL
                  WHERE project_id=?3 AND app_record_id=?4 AND id=?5",
                params![
                    start_sequence,
                    timestamp,
                    project_id.trim(),
                    app_record_id.trim(),
                    subscription_id.trim()
                ],
            )?;
        } else {
            tx.execute(
                "UPDATE open_commerce_developer_webhook_subscriptions
                    SET status='disabled', updated_at=?1, disabled_at=?1
                  WHERE project_id=?2 AND app_record_id=?3 AND id=?4",
                params![
                    timestamp,
                    project_id.trim(),
                    app_record_id.trim(),
                    subscription_id.trim()
                ],
            )?;
            tx.execute(
                "UPDATE open_commerce_developer_webhook_deliveries
                    SET status='dead', error_code='subscription_disabled',
                        lease_owner=NULL, lease_expires_at=NULL
                  WHERE subscription_id=?1 AND status IN ('pending', 'retry', 'delivering')",
                params![subscription_id.trim()],
            )?;
        }
        tx.commit()?;
        self.open_commerce_developer_webhook_for_app(project_id, app_record_id, subscription_id)
    }

    pub(crate) fn list_open_commerce_developer_webhook_deliveries(
        &self,
        project_id: &str,
        app_record_id: &str,
        subscription_id: &str,
        limit: usize,
    ) -> Result<Vec<DeveloperWebhookDelivery>> {
        self.open_commerce_developer_webhook_for_app(project_id, app_record_id, subscription_id)?;
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "{DELIVERY_SELECT}
              WHERE subscription_id=?1
              ORDER BY event_sequence DESC LIMIT ?2"
        ))?;
        Ok(statement
            .query_map(
                params![subscription_id.trim(), limit.clamp(1, 100) as i64],
                delivery_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn claim_open_commerce_developer_webhook_delivery(
        &self,
        lease_owner: &str,
    ) -> Result<Option<DeveloperWebhookDeliveryClaim>> {
        let timestamp = Utc::now();
        let timestamp_text = timestamp.to_rfc3339();
        let lease_expires_at = (timestamp + Duration::seconds(30)).to_rfc3339();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE open_commerce_developer_webhook_deliveries
                SET status='retry', lease_owner=NULL, lease_expires_at=NULL,
                    next_attempt_at=?1, error_code='delivery_lease_expired'
              WHERE status='delivering'
                AND julianday(lease_expires_at) <= julianday(?1)",
            params![timestamp_text],
        )?;
        let candidate: Option<String> = tx
            .query_row(
                "SELECT delivery.id
                   FROM open_commerce_developer_webhook_deliveries delivery
                   JOIN open_commerce_developer_webhook_subscriptions subscription
                     ON subscription.id=delivery.subscription_id
                   JOIN open_commerce_developer_apps app
                     ON app.id=subscription.app_record_id
                  WHERE delivery.status IN ('pending', 'retry')
                    AND julianday(delivery.next_attempt_at) <= julianday(?1)
                    AND subscription.status='active'
                    AND app.status='active'
                  ORDER BY delivery.next_attempt_at, delivery.event_sequence
                  LIMIT 1",
                params![timestamp_text],
                |row| row.get(0),
            )
            .optional()?;
        let Some(delivery_id) = candidate else {
            tx.commit()?;
            return Ok(None);
        };
        let changed = tx.execute(
            "UPDATE open_commerce_developer_webhook_deliveries
                SET status='delivering', attempt_count=attempt_count+1,
                    lease_owner=?1, lease_expires_at=?2, last_attempt_at=?3
              WHERE id=?4 AND status IN ('pending', 'retry')",
            params![
                lease_owner.trim(),
                lease_expires_at,
                timestamp_text,
                delivery_id
            ],
        )?;
        if changed != 1 {
            tx.commit()?;
            return Ok(None);
        }
        let claim = tx.query_row(
            &format!(
                "SELECT delivery.id, delivery.subscription_id, delivery.invocation_id,
                        delivery.event_sequence, delivery.event_type, delivery.status,
                        delivery.attempt_count, delivery.manual_retry_count,
                        delivery.next_attempt_at, delivery.response_status,
                        delivery.error_code, delivery.created_at, delivery.last_attempt_at,
                        delivery.last_manual_retry_at, delivery.delivered_at,
                        subscription.owner_user_id, subscription.app_id,
                        subscription.callback_url, subscription.signing_key_id,
                        subscription.signing_secret_version
                   FROM open_commerce_developer_webhook_deliveries delivery
                   JOIN open_commerce_developer_webhook_subscriptions subscription
                     ON subscription.id=delivery.subscription_id
                  WHERE delivery.id=?1 AND delivery.lease_owner=?2"
            ),
            params![delivery_id, lease_owner.trim()],
            |row| {
                Ok(DeveloperWebhookDeliveryClaim {
                    delivery: DeveloperWebhookDelivery {
                        schema: DEVELOPER_WEBHOOK_DELIVERY_SCHEMA,
                        id: row.get(0)?,
                        subscription_id: row.get(1)?,
                        invocation_id: row.get(2)?,
                        event_sequence: row.get(3)?,
                        event_type: row.get(4)?,
                        status: row.get(5)?,
                        attempt_count: row.get(6)?,
                        manual_retry_count: row.get(7)?,
                        next_attempt_at: row.get(8)?,
                        response_status: row.get(9)?,
                        error_code: row.get(10)?,
                        created_at: row.get(11)?,
                        last_attempt_at: row.get(12)?,
                        last_manual_retry_at: row.get(13)?,
                        delivered_at: row.get(14)?,
                    },
                    owner_user_id: row.get(15)?,
                    app_id: row.get(16)?,
                    callback_url: row.get(17)?,
                    signing_key_id: row.get(18)?,
                    signing_secret_version: row.get(19)?,
                    lease_owner: lease_owner.trim().to_string(),
                })
            },
        )?;
        tx.commit()?;
        Ok(Some(claim))
    }

    pub(crate) fn complete_open_commerce_developer_webhook_delivery(
        &self,
        claim: &DeveloperWebhookDeliveryClaim,
        response_status: i64,
    ) -> Result<()> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE open_commerce_developer_webhook_deliveries
                SET status='delivered', response_status=?1, error_code=NULL,
                    delivered_at=?2, lease_owner=NULL, lease_expires_at=NULL
              WHERE id=?3 AND status='delivering' AND lease_owner=?4",
            params![
                response_status,
                timestamp,
                claim.delivery.id,
                claim.lease_owner
            ],
        )?;
        if changed != 1 {
            bail!("Webhook 投递租约已失效");
        }
        tx.execute(
            "UPDATE open_commerce_developer_webhook_subscriptions
                SET consecutive_failures=0, last_delivery_at=?1,
                    last_error_code=NULL, updated_at=?1
              WHERE id=?2",
            params![timestamp, claim.delivery.subscription_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn fail_open_commerce_developer_webhook_delivery(
        &self,
        claim: &DeveloperWebhookDeliveryClaim,
        response_status: Option<i64>,
        error_code: &str,
        retry_after_seconds: Option<i64>,
        force_disable: bool,
    ) -> Result<()> {
        let timestamp = Utc::now();
        let timestamp_text = timestamp.to_rfc3339();
        let next_attempt_at = (timestamp
            + Duration::seconds(retry_after_seconds.unwrap_or(0).clamp(0, 3600)))
        .to_rfc3339();
        let retrying = retry_after_seconds.is_some() && claim.delivery.attempt_count < 8;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE open_commerce_developer_webhook_deliveries
                SET status=?1, response_status=?2, error_code=?3,
                    next_attempt_at=?4, lease_owner=NULL, lease_expires_at=NULL
              WHERE id=?5 AND status='delivering' AND lease_owner=?6",
            params![
                if retrying { "retry" } else { "dead" },
                response_status,
                error_code.trim(),
                next_attempt_at,
                claim.delivery.id,
                claim.lease_owner
            ],
        )?;
        if changed != 1 {
            bail!("Webhook 投递租约已失效");
        }
        tx.execute(
            "UPDATE open_commerce_developer_webhook_subscriptions
                SET consecutive_failures=consecutive_failures+1,
                    last_error_code=?1, updated_at=?2
              WHERE id=?3",
            params![
                error_code.trim(),
                timestamp_text,
                claim.delivery.subscription_id
            ],
        )?;
        let failures: i64 = tx.query_row(
            "SELECT consecutive_failures
               FROM open_commerce_developer_webhook_subscriptions WHERE id=?1",
            params![claim.delivery.subscription_id],
            |row| row.get(0),
        )?;
        if force_disable || failures >= 8 {
            tx.execute(
                "UPDATE open_commerce_developer_webhook_subscriptions
                    SET status='disabled', disabled_at=?1, updated_at=?1
                  WHERE id=?2",
                params![timestamp_text, claim.delivery.subscription_id],
            )?;
            tx.execute(
                "UPDATE open_commerce_developer_webhook_deliveries
                    SET status='dead', error_code='subscription_auto_disabled',
                        lease_owner=NULL, lease_expires_at=NULL
                  WHERE subscription_id=?1 AND status IN ('pending', 'retry')",
                params![claim.delivery.subscription_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

fn subscription_from_row(row: &Row<'_>) -> rusqlite::Result<DeveloperWebhookSubscription> {
    Ok(DeveloperWebhookSubscription {
        schema: DEVELOPER_WEBHOOK_SUBSCRIPTION_SCHEMA,
        id: row.get(0)?,
        project_id: row.get(1)?,
        app_record_id: row.get(2)?,
        app_id: row.get(3)?,
        callback_url: row.get(4)?,
        signing_key_id: row.get(5)?,
        signing_secret_version: row.get(6)?,
        status: row.get(7)?,
        verification_status: row.get(8)?,
        verification_attempted_at: row.get(9)?,
        verification_error_code: row.get(10)?,
        verified_at: row.get(11)?,
        consecutive_failures: row.get(12)?,
        last_delivery_at: row.get(13)?,
        last_error_code: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        disabled_at: row.get(17)?,
    })
}

pub(super) fn delivery_from_row(row: &Row<'_>) -> rusqlite::Result<DeveloperWebhookDelivery> {
    Ok(DeveloperWebhookDelivery {
        schema: DEVELOPER_WEBHOOK_DELIVERY_SCHEMA,
        id: row.get(0)?,
        subscription_id: row.get(1)?,
        invocation_id: row.get(2)?,
        event_sequence: row.get(3)?,
        event_type: row.get(4)?,
        status: row.get(5)?,
        attempt_count: row.get(6)?,
        manual_retry_count: row.get(7)?,
        next_attempt_at: row.get(8)?,
        response_status: row.get(9)?,
        error_code: row.get(10)?,
        created_at: row.get(11)?,
        last_attempt_at: row.get(12)?,
        last_manual_retry_at: row.get(13)?,
        delivered_at: row.get(14)?,
    })
}
