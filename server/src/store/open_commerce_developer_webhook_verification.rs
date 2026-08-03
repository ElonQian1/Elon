use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::open_commerce_webhook_model::DeveloperWebhookSubscription;

use super::{now, Store};

impl Store {
    pub(crate) fn record_open_commerce_developer_webhook_verification_failure(
        &self,
        project_id: &str,
        app_record_id: &str,
        subscription_id: &str,
        error_code: &str,
    ) -> Result<DeveloperWebhookSubscription> {
        let timestamp = now();
        let changed = self.conn()?.execute(
            "UPDATE open_commerce_developer_webhook_subscriptions
                SET status='disabled', verification_status='failed',
                    verification_attempted_at=?1, verification_error_code=?2,
                    updated_at=?1, disabled_at=COALESCE(disabled_at, ?1)
              WHERE project_id=?3 AND app_record_id=?4 AND id=?5",
            params![
                timestamp,
                error_code.trim(),
                project_id.trim(),
                app_record_id.trim(),
                subscription_id.trim()
            ],
        )?;
        if changed != 1 {
            bail!("Webhook 订阅不存在");
        }
        self.open_commerce_developer_webhook_for_app(project_id, app_record_id, subscription_id)
    }

    pub(crate) fn verify_open_commerce_developer_webhook(
        &self,
        project_id: &str,
        app_record_id: &str,
        subscription_id: &str,
    ) -> Result<DeveloperWebhookSubscription> {
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
            bail!("开发者 App 已停用，不能验证 Webhook");
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
        let changed = tx.execute(
            "UPDATE open_commerce_developer_webhook_subscriptions
                SET status='active', verification_status='verified',
                    verification_attempted_at=?1, verification_error_code=NULL,
                    verified_at=?1, start_sequence=?2, consecutive_failures=0,
                    last_error_code=NULL, updated_at=?1, disabled_at=NULL
              WHERE project_id=?3 AND app_record_id=?4 AND id=?5",
            params![
                timestamp,
                start_sequence,
                project_id.trim(),
                app_record_id.trim(),
                subscription_id.trim()
            ],
        )?;
        if changed != 1 {
            bail!("Webhook 订阅不存在");
        }
        tx.commit()?;
        self.open_commerce_developer_webhook_for_app(project_id, app_record_id, subscription_id)
    }
}
