use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, TransactionBehavior};

use crate::open_commerce_webhook_model::DeveloperWebhookHistoryReplayResult;

use super::{now, Store};

impl Store {
    pub(crate) fn replay_open_commerce_developer_webhook_history(
        &self,
        project_id: &str,
        app_record_id: &str,
        subscription_id: &str,
        after_sequence: i64,
        limit: usize,
    ) -> Result<DeveloperWebhookHistoryReplayResult> {
        let after_sequence = after_sequence.max(0);
        let limit = limit.clamp(1, 100);
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (
            owner_user_id,
            app_id,
            subscription_status,
            verification_status,
            app_status,
            deliver_on_succeeded,
            deliver_on_failed,
        ): (String, String, String, String, String, bool, bool) = tx
            .query_row(
                "SELECT subscription.owner_user_id, subscription.app_id,
                        subscription.status, subscription.verification_status, app.status,
                        subscription.deliver_on_succeeded, subscription.deliver_on_failed
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
            )
            .map_err(|error| anyhow!(error).context("Webhook 订阅不存在"))?;
        if app_status != "active"
            || subscription_status != "active"
            || verification_status != "verified"
        {
            bail!("Webhook 订阅必须处于已验证且启用状态才能补发历史事件");
        }
        let events: Vec<(i64, String, String)> = {
            let mut statement = tx.prepare(
                "SELECT event.seq, invocation.id, invocation.status
                   FROM open_commerce_invocation_terminal_events event
                   JOIN open_commerce_invocations invocation
                     ON invocation.id=event.invocation_id
                  WHERE invocation.requester_user_id=?1
                    AND invocation.requester_app_id=?2
                    AND event.seq>?3
                    AND ((invocation.status='succeeded' AND ?4=1)
                         OR (invocation.status<>'succeeded' AND ?5=1))
                  ORDER BY event.seq ASC LIMIT ?6",
            )?;
            statement
                .query_map(
                    params![
                        owner_user_id,
                        app_id,
                        after_sequence,
                        if deliver_on_succeeded { 1 } else { 0 },
                        if deliver_on_failed { 1 } else { 0 },
                        (limit + 1) as i64
                    ],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let has_more = events.len() > limit;
        let eligible = events.into_iter().take(limit).collect::<Vec<_>>();
        let mut enqueued_count = 0usize;
        for (sequence, invocation_id, status) in &eligible {
            enqueued_count += tx.execute(
                "INSERT OR IGNORE INTO open_commerce_developer_webhook_deliveries(
                   id, subscription_id, invocation_id, event_sequence, event_type,
                   enqueue_source, history_replay_requested_at, status, attempt_count,
                   next_attempt_at, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'history_replay', ?6, 'pending', 0, ?6, ?6)",
                params![
                    format!(
                        "webhook_delivery:{}:{}",
                        subscription_id.trim(),
                        invocation_id
                    ),
                    subscription_id.trim(),
                    invocation_id,
                    sequence,
                    if status == "succeeded" {
                        "invocation.succeeded"
                    } else {
                        "invocation.failed"
                    },
                    timestamp
                ],
            )?;
        }
        let processed_through_sequence = eligible
            .last()
            .map(|(sequence, _, _)| *sequence)
            .unwrap_or(after_sequence);
        let eligible_count = eligible.len();
        tx.commit()?;
        Ok(DeveloperWebhookHistoryReplayResult {
            schema: "open_commerce.developer_webhook_history_replay.v1",
            subscription_id: subscription_id.trim().to_string(),
            after_sequence,
            processed_through_sequence,
            eligible_count,
            enqueued_count,
            already_present_count: eligible_count.saturating_sub(enqueued_count),
            has_more,
        })
    }
}
