use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::open_commerce_webhook_model::DeveloperWebhookSubscription;

use super::{now, Store};

impl Store {
    pub(crate) fn rotate_open_commerce_developer_webhook_secret(
        &self,
        project_id: &str,
        app_record_id: &str,
        subscription_id: &str,
        expected_version: i64,
        next_version: i64,
        signing_key_id: &str,
    ) -> Result<DeveloperWebhookSubscription> {
        if expected_version < 1 || next_version != expected_version.saturating_add(1) {
            bail!("Webhook 签名密钥版本轮换请求无效");
        }
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let app_status: String = tx.query_row(
            "SELECT status FROM open_commerce_developer_apps
              WHERE project_id=?1 AND id=?2",
            params![project_id.trim(), app_record_id.trim()],
            |row| row.get(0),
        )?;
        if app_status != "active" {
            bail!("开发者 App 已停用，不能轮换 Webhook 密钥");
        }
        let changed = tx.execute(
            "UPDATE open_commerce_developer_webhook_subscriptions
                SET signing_key_id=?1, signing_secret_version=?2,
                    status='disabled', verification_status='pending',
                    verification_attempted_at=NULL, verification_error_code=NULL,
                    verified_at=NULL, consecutive_failures=0, last_error_code=NULL,
                    updated_at=?3, disabled_at=?3
              WHERE project_id=?4 AND app_record_id=?5 AND id=?6
                AND signing_secret_version=?7",
            params![
                signing_key_id.trim(),
                next_version,
                timestamp,
                project_id.trim(),
                app_record_id.trim(),
                subscription_id.trim(),
                expected_version
            ],
        )?;
        if changed != 1 {
            bail!("Webhook 订阅不存在或签名密钥已被其他请求轮换");
        }
        tx.execute(
            "UPDATE open_commerce_developer_webhook_deliveries
                SET status='dead', error_code='webhook_signing_secret_rotated',
                    lease_owner=NULL, lease_expires_at=NULL
              WHERE subscription_id=?1 AND status IN ('pending', 'retry', 'delivering')",
            params![subscription_id.trim()],
        )?;
        tx.commit()?;
        self.open_commerce_developer_webhook_for_app(project_id, app_record_id, subscription_id)
    }
}
