use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::open_commerce_data_request_model::{
    OpenCommerceConsumerDataRequest, DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE,
    DATA_REQUEST_FOLLOWUP_ACTION_REMINDER, DATA_REQUEST_STATUS_IN_PROGRESS,
    DATA_REQUEST_STATUS_REQUESTED,
};

use super::{
    new_id, now,
    open_commerce_consumer_data_requests::{data_request_from_row, DATA_REQUEST_SELECT},
    Store,
};

const FIRST_REMINDER_DELAY_HOURS: i64 = 24;
const REMINDER_COOLDOWN_HOURS: i64 = 24;
const MAX_REMINDERS: u32 = 3;
const OPERATIONAL_TARGET_DAYS: i64 = 7;

#[derive(Debug)]
struct FollowupSummary {
    reminder_count: u32,
    last_reminded_at: Option<String>,
    escalated_at: Option<String>,
}

impl Store {
    pub(crate) fn populate_open_commerce_data_request_operations(
        &self,
        request: &mut OpenCommerceConsumerDataRequest,
    ) -> Result<()> {
        let conn = self.conn()?;
        let summary = conn.query_row(
            "SELECT
               SUM(CASE WHEN action_kind='reminder' THEN 1 ELSE 0 END),
               MAX(CASE WHEN action_kind='reminder' THEN created_at END),
               MAX(CASE WHEN action_kind='escalate_attention' THEN created_at END)
             FROM open_commerce_data_request_followups
            WHERE data_request_id=?1",
            params![request.id.trim()],
            |row| {
                Ok(FollowupSummary {
                    reminder_count: row.get::<_, Option<i64>>(0)?.unwrap_or(0).max(0) as u32,
                    last_reminded_at: row.get(1)?,
                    escalated_at: row.get(2)?,
                })
            },
        )?;
        apply_operational_fields(request, summary, &now())
    }

    pub(crate) fn follow_up_open_commerce_consumer_data_request(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        request_id: &str,
        action: &str,
        idempotency_key: &str,
        note: Option<&str>,
    ) -> Result<(OpenCommerceConsumerDataRequest, bool)> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let request = tx
            .query_row(
                &format!(
                    "{DATA_REQUEST_SELECT}
                      WHERE id=?1 AND consumer_project_id=?2 AND consumer_user_id=?3"
                ),
                params![
                    request_id.trim(),
                    consumer_project_id.trim(),
                    consumer_user_id.trim()
                ],
                data_request_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("消费者数据请求不存在"))?;
        let existing = tx
            .query_row(
                "SELECT action_kind, note
                   FROM open_commerce_data_request_followups
                  WHERE data_request_id=?1 AND consumer_user_id=?2 AND idempotency_key=?3",
                params![request.id, consumer_user_id.trim(), idempotency_key.trim()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        if let Some((existing_action, existing_note)) = existing {
            if existing_action != action || existing_note.as_deref() != note {
                bail!("幂等键已用于不同的删除请求跟进操作");
            }
            tx.commit()?;
            drop(conn);
            let mut current = self.open_commerce_data_request_for_followup(request_id)?;
            self.populate_open_commerce_data_request_operations(&mut current)?;
            return Ok((current, false));
        }
        if !is_open(&request.status) {
            bail!("只有待处理或处理中的消费者数据请求可以催办或升级关注");
        }

        let timestamp = now();
        let summary = followup_summary_in_transaction(&tx, &request.id)?;
        let mut operational = request.clone();
        apply_operational_fields(&mut operational, summary, &timestamp)?;
        match action {
            DATA_REQUEST_FOLLOWUP_ACTION_REMINDER if !operational.can_send_reminder => {
                if operational.reminder_count >= MAX_REMINDERS {
                    bail!("该删除请求已达到最多三次催办上限");
                }
                bail!("当前尚未到下一次允许催办的时间");
            }
            DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE if !operational.can_escalate_attention => {
                bail!("删除请求需超过七天运营目标且至少催办一次后才能升级关注");
            }
            DATA_REQUEST_FOLLOWUP_ACTION_REMINDER | DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE => {}
            _ => bail!("消费者数据请求跟进动作无效"),
        }

        tx.execute(
            "INSERT INTO open_commerce_data_request_followups (
               id, data_request_id, consumer_project_id, consumer_user_id,
               merchant_project_id, merchant_id, action_kind, idempotency_key,
               note, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                new_id("data_request_followup"),
                request.id,
                consumer_project_id.trim(),
                consumer_user_id.trim(),
                request_merchant_project_id(&tx, request_id)?,
                request.merchant_id,
                action,
                idempotency_key.trim(),
                note,
                timestamp,
            ],
        )?;
        tx.commit()?;
        drop(conn);
        let mut current = self.open_commerce_data_request_for_followup(request_id)?;
        self.populate_open_commerce_data_request_operations(&mut current)?;
        Ok((current, true))
    }

    fn open_commerce_data_request_for_followup(
        &self,
        request_id: &str,
    ) -> Result<OpenCommerceConsumerDataRequest> {
        self.conn()?
            .query_row(
                &format!("{DATA_REQUEST_SELECT} WHERE id=?1"),
                params![request_id.trim()],
                data_request_from_row,
            )
            .map_err(|error| anyhow!(error).context("消费者数据请求不存在"))
    }
}

fn followup_summary_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    request_id: &str,
) -> Result<FollowupSummary> {
    Ok(tx.query_row(
        "SELECT
           SUM(CASE WHEN action_kind='reminder' THEN 1 ELSE 0 END),
           MAX(CASE WHEN action_kind='reminder' THEN created_at END),
           MAX(CASE WHEN action_kind='escalate_attention' THEN created_at END)
         FROM open_commerce_data_request_followups
        WHERE data_request_id=?1",
        params![request_id.trim()],
        |row| {
            Ok(FollowupSummary {
                reminder_count: row.get::<_, Option<i64>>(0)?.unwrap_or(0).max(0) as u32,
                last_reminded_at: row.get(1)?,
                escalated_at: row.get(2)?,
            })
        },
    )?)
}

fn request_merchant_project_id(tx: &rusqlite::Transaction<'_>, request_id: &str) -> Result<String> {
    Ok(tx.query_row(
        "SELECT merchant_project_id FROM open_commerce_consumer_data_requests WHERE id=?1",
        params![request_id.trim()],
        |row| row.get(0),
    )?)
}

fn apply_operational_fields(
    request: &mut OpenCommerceConsumerDataRequest,
    summary: FollowupSummary,
    current_time: &str,
) -> Result<()> {
    let requested_at = parse_time(&request.requested_at, "requested_at")?;
    let now_at = parse_time(current_time, "current_time")?;
    let target_at = requested_at + Duration::days(OPERATIONAL_TARGET_DAYS);
    let first_reminder_at = requested_at + Duration::hours(FIRST_REMINDER_DELAY_HOURS);
    let next_reminder = summary
        .last_reminded_at
        .as_deref()
        .map(|value| parse_time(value, "last_reminded_at"))
        .transpose()?
        .map(|value| value + Duration::hours(REMINDER_COOLDOWN_HOURS))
        .unwrap_or(first_reminder_at);
    let open = is_open(&request.status);

    request.operational_target_at = Some(target_at.to_rfc3339());
    request.is_operationally_overdue = open && now_at >= target_at;
    request.reminder_count = summary.reminder_count;
    request.last_reminded_at = summary.last_reminded_at;
    request.next_reminder_at = open
        .then(|| next_reminder.to_rfc3339())
        .filter(|_| summary.reminder_count < MAX_REMINDERS);
    request.consumer_escalated_at = summary.escalated_at;
    request.can_send_reminder =
        open && summary.reminder_count < MAX_REMINDERS && now_at >= next_reminder;
    request.can_escalate_attention = open
        && now_at >= target_at
        && summary.reminder_count > 0
        && request.consumer_escalated_at.is_none();
    Ok(())
}

fn is_open(status: &str) -> bool {
    matches!(
        status,
        DATA_REQUEST_STATUS_REQUESTED | DATA_REQUEST_STATUS_IN_PROGRESS
    )
}

fn parse_time(value: &str, label: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value.trim())
        .with_context(|| format!("消费者数据请求 {label} 时间无效"))?
        .with_timezone(&Utc))
}
