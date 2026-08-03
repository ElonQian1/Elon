use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, TransactionBehavior};

use crate::open_commerce_webhook_model::DeveloperWebhookDelivery;

use super::{
    now,
    open_commerce_developer_webhook_rows::{delivery_from_row, DELIVERY_SELECT},
    Store,
};

impl Store {
    pub(crate) fn acknowledge_open_commerce_developer_webhook_dead_letter(
        &self,
        project_id: &str,
        app_record_id: &str,
        subscription_id: &str,
        delivery_id: &str,
        acknowledged_by_user_id: &str,
        reason: &str,
    ) -> Result<DeveloperWebhookDelivery> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = tx
            .query_row(
                &format!(
                    "{DELIVERY_SELECT}
                  WHERE subscription_id=?1 AND id=?2
                    AND EXISTS(
                      SELECT 1 FROM open_commerce_developer_webhook_subscriptions subscription
                       WHERE subscription.id=open_commerce_developer_webhook_deliveries.subscription_id
                         AND subscription.project_id=?3 AND subscription.app_record_id=?4
                    )"
                ),
                params![
                    subscription_id.trim(),
                    delivery_id.trim(),
                    project_id.trim(),
                    app_record_id.trim()
                ],
                delivery_from_row,
            )
            .map_err(|error| anyhow!(error).context("Webhook 死信投递不存在"))?;
        if current.status != "dead" {
            bail!("只有死信投递可以确认处理");
        }
        if current.dead_letter_acknowledged_at.is_some() {
            if current.dead_letter_acknowledged_by_user_id.as_deref()
                == Some(acknowledged_by_user_id.trim())
                && current.dead_letter_acknowledgement_reason.as_deref() == Some(reason.trim())
            {
                tx.commit()?;
                return Ok(current);
            }
            bail!("该死信已经确认，不能覆盖原处理证据");
        }
        let changed = tx.execute(
            "UPDATE open_commerce_developer_webhook_deliveries
                SET dead_letter_acknowledged_at=?1,
                    dead_letter_acknowledged_by_user_id=?2,
                    dead_letter_acknowledgement_reason=?3
              WHERE subscription_id=?4 AND id=?5 AND status='dead'
                AND dead_letter_acknowledged_at IS NULL",
            params![
                timestamp,
                acknowledged_by_user_id.trim(),
                reason.trim(),
                subscription_id.trim(),
                delivery_id.trim()
            ],
        )?;
        if changed != 1 {
            bail!("死信状态已被其他请求改变");
        }
        let delivery = tx.query_row(
            &format!("{DELIVERY_SELECT} WHERE subscription_id=?1 AND id=?2"),
            params![subscription_id.trim(), delivery_id.trim()],
            delivery_from_row,
        )?;
        tx.commit()?;
        Ok(delivery)
    }
}
