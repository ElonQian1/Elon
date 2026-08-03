use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, TransactionBehavior};

use crate::open_commerce_webhook_model::DeveloperWebhookDelivery;

use super::{
    now,
    open_commerce_developer_webhook_rows::{delivery_from_row, DELIVERY_SELECT},
    Store,
};

impl Store {
    pub(crate) fn retry_open_commerce_developer_webhook_delivery(
        &self,
        project_id: &str,
        app_record_id: &str,
        subscription_id: &str,
        delivery_id: &str,
    ) -> Result<DeveloperWebhookDelivery> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (subscription_status, verification_status, app_status, environment): (
            String,
            String,
            String,
            String,
        ) = tx
            .query_row(
                "SELECT subscription.status, subscription.verification_status, app.status,
                        subscription.environment
                   FROM open_commerce_developer_webhook_subscriptions subscription
                   JOIN open_commerce_developer_apps app
                     ON app.id=subscription.app_record_id
                  WHERE subscription.project_id=?1 AND subscription.app_record_id=?2
                    AND subscription.id=?3",
                params![
                    project_id.trim(),
                    app_record_id.trim(),
                    subscription_id.trim()
                ],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|error| anyhow!(error).context("Webhook 订阅不存在"))?;
        if app_status != "active"
            || subscription_status != "active"
            || verification_status != "verified"
        {
            bail!("Webhook 订阅必须处于已验证且启用状态才能重试死信");
        }
        if environment == "production" {
            super::open_commerce_developer_credentials::ensure_current_production_credential_on(
                &tx,
                project_id,
                app_record_id,
            )?;
        }
        let current = tx
            .query_row(
                &format!(
                    "{DELIVERY_SELECT}
                      WHERE subscription_id=?1 AND id=?2"
                ),
                params![subscription_id.trim(), delivery_id.trim()],
                delivery_from_row,
            )
            .map_err(|error| anyhow!(error).context("Webhook 投递不存在"))?;
        let credential_environment: String = tx.query_row(
            "SELECT credential_environment FROM open_commerce_invocations WHERE id=?1",
            params![current.invocation_id.as_str()],
            |row| row.get(0),
        )?;
        let environment_matches = match environment.as_str() {
            "sandbox" => matches!(credential_environment.as_str(), "legacy" | "sandbox"),
            "production" => credential_environment == "production",
            _ => false,
        };
        if !environment_matches {
            bail!("Webhook 投递与订阅凭据环境不一致");
        }
        match current.status.as_str() {
            "dead" => {}
            "delivered" => bail!("Webhook 投递已经成功，不能重复发送"),
            _ => bail!("Webhook 投递仍在处理或等待自动重试"),
        }
        let changed = tx.execute(
            "UPDATE open_commerce_developer_webhook_deliveries
                SET status='pending', attempt_count=0,
                    manual_retry_count=manual_retry_count+1,
                    next_attempt_at=?1, response_status=NULL, error_code=NULL,
                    last_attempt_at=NULL, last_manual_retry_at=?1, delivered_at=NULL,
                    lease_owner=NULL, lease_expires_at=NULL
              WHERE subscription_id=?2 AND id=?3 AND status='dead'",
            params![timestamp, subscription_id.trim(), delivery_id.trim()],
        )?;
        if changed != 1 {
            bail!("Webhook 死信状态已被其他请求改变");
        }
        let delivery = tx.query_row(
            &format!(
                "{DELIVERY_SELECT}
                  WHERE subscription_id=?1 AND id=?2"
            ),
            params![subscription_id.trim(), delivery_id.trim()],
            delivery_from_row,
        )?;
        tx.commit()?;
        Ok(delivery)
    }
}
