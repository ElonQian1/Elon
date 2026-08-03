use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Row, Transaction, TransactionBehavior};

use crate::open_commerce_data_request_model::{
    OpenCommerceConsumerDataRequest, DATA_REQUEST_STATUS_COMPLETED,
    DATA_REQUEST_STATUS_IN_PROGRESS, DATA_REQUEST_STATUS_REJECTED, DATA_REQUEST_STATUS_REQUESTED,
    DATA_REQUEST_STATUS_WITHDRAWN, DATA_REQUEST_TYPE_ERASURE,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_open_commerce_consumer_data_erasure_request(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        relationship_id: &str,
    ) -> Result<(OpenCommerceConsumerDataRequest, bool)> {
        let id = new_id("data_request");
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let relationship = tx
            .query_row(
                "SELECT merchant_project_id, merchant_id, subject_alias
                   FROM open_commerce_consumer_relationships
                  WHERE id=?1 AND consumer_project_id=?2 AND consumer_user_id=?3",
                params![
                    relationship_id.trim(),
                    consumer_project_id.trim(),
                    consumer_user_id.trim()
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("消费者关系凭证不存在"))?;
        tx.execute(
            "UPDATE open_commerce_consumer_relationships
                SET status='revoked', revoked_at=COALESCE(revoked_at, ?1), updated_at=?1
              WHERE id=?2 AND consumer_project_id=?3 AND consumer_user_id=?4
                AND status='active'",
            params![
                timestamp,
                relationship_id.trim(),
                consumer_project_id.trim(),
                consumer_user_id.trim()
            ],
        )?;
        if let Some(existing) = open_request_for_relationship(&tx, relationship_id)? {
            tx.commit()?;
            return Ok((existing, false));
        }
        tx.execute(
            "INSERT INTO open_commerce_consumer_data_requests (
               id, consumer_project_id, consumer_user_id, merchant_project_id,
               merchant_id, relationship_id, subject_alias, request_type, status,
               resolution_kind, resolution_note, requested_at, accepted_at,
               resolved_at, withdrawn_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                       NULL, NULL, ?10, NULL, NULL, NULL, ?10)",
            params![
                id,
                consumer_project_id.trim(),
                consumer_user_id.trim(),
                relationship.0,
                relationship.1,
                relationship_id.trim(),
                relationship.2,
                DATA_REQUEST_TYPE_ERASURE,
                DATA_REQUEST_STATUS_REQUESTED,
                timestamp,
            ],
        )?;
        tx.commit()?;
        drop(conn);
        Ok((self.open_commerce_consumer_data_request(&id)?, true))
    }

    pub(crate) fn list_open_commerce_consumer_data_requests(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceConsumerDataRequest>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{DATA_REQUEST_SELECT}
              WHERE consumer_project_id=?1 AND consumer_user_id=?2
              ORDER BY CASE status
                WHEN 'requested' THEN 0 WHEN 'in_progress' THEN 1 ELSE 2 END,
                updated_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                consumer_project_id.trim(),
                consumer_user_id.trim(),
                limit.clamp(1, 200) as i64
            ],
            data_request_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn list_open_commerce_merchant_data_requests(
        &self,
        merchant_project_id: &str,
        merchant_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceConsumerDataRequest>> {
        self.open_commerce_merchant_for_project(merchant_project_id, merchant_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{DATA_REQUEST_SELECT}
              WHERE merchant_project_id=?1 AND merchant_id=?2
              ORDER BY CASE status
                WHEN 'requested' THEN 0 WHEN 'in_progress' THEN 1 ELSE 2 END,
                updated_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                merchant_project_id.trim(),
                merchant_id.trim(),
                limit.clamp(1, 200) as i64
            ],
            data_request_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn withdraw_open_commerce_consumer_data_request(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        request_id: &str,
    ) -> Result<(OpenCommerceConsumerDataRequest, bool)> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current =
            consumer_owned_request(&tx, consumer_project_id, consumer_user_id, request_id)?
                .ok_or_else(|| anyhow!("消费者数据请求不存在"))?;
        if current.status == DATA_REQUEST_STATUS_WITHDRAWN {
            tx.commit()?;
            return Ok((current, false));
        }
        if current.status != DATA_REQUEST_STATUS_REQUESTED {
            bail!("商户已处理该请求，当前状态不能撤回");
        }
        let timestamp = now();
        tx.execute(
            "UPDATE open_commerce_consumer_data_requests
                SET status=?1, resolution_kind='consumer_withdrawn',
                    withdrawn_at=?2, updated_at=?2
              WHERE id=?3 AND consumer_project_id=?4 AND consumer_user_id=?5
                AND status=?6",
            params![
                DATA_REQUEST_STATUS_WITHDRAWN,
                timestamp,
                request_id.trim(),
                consumer_project_id.trim(),
                consumer_user_id.trim(),
                DATA_REQUEST_STATUS_REQUESTED,
            ],
        )?;
        tx.commit()?;
        drop(conn);
        Ok((self.open_commerce_consumer_data_request(request_id)?, true))
    }

    pub(crate) fn decide_open_commerce_consumer_data_request(
        &self,
        merchant_project_id: &str,
        merchant_id: &str,
        request_id: &str,
        action: &str,
        note: Option<&str>,
    ) -> Result<(OpenCommerceConsumerDataRequest, bool)> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = merchant_owned_request(&tx, merchant_project_id, merchant_id, request_id)?
            .ok_or_else(|| anyhow!("消费者数据请求不存在"))?;
        let target = match action {
            "accept" => DATA_REQUEST_STATUS_IN_PROGRESS,
            "complete" => DATA_REQUEST_STATUS_COMPLETED,
            "reject" => DATA_REQUEST_STATUS_REJECTED,
            _ => bail!("消费者数据请求处理动作无效"),
        };
        if current.status == target {
            tx.commit()?;
            return Ok((current, false));
        }
        let allowed = match action {
            "accept" => current.status == DATA_REQUEST_STATUS_REQUESTED,
            "complete" => current.status == DATA_REQUEST_STATUS_IN_PROGRESS,
            "reject" => matches!(
                current.status.as_str(),
                DATA_REQUEST_STATUS_REQUESTED | DATA_REQUEST_STATUS_IN_PROGRESS
            ),
            _ => false,
        };
        if !allowed {
            bail!("消费者数据请求不能从当前状态执行该动作");
        }
        let timestamp = now();
        let resolution_kind = match action {
            "accept" => "merchant_processing",
            "complete" => "merchant_attested_completed",
            "reject" => "merchant_rejected",
            _ => unreachable!(),
        };
        tx.execute(
            "UPDATE open_commerce_consumer_data_requests
                SET status=?1, resolution_kind=?2, resolution_note=?3,
                    accepted_at=CASE WHEN ?4='accept' THEN COALESCE(accepted_at, ?5)
                                     ELSE accepted_at END,
                    resolved_at=CASE WHEN ?4 IN ('complete', 'reject') THEN ?5
                                     ELSE resolved_at END,
                    updated_at=?5
              WHERE id=?6 AND merchant_project_id=?7 AND merchant_id=?8",
            params![
                target,
                resolution_kind,
                note,
                action,
                timestamp,
                request_id.trim(),
                merchant_project_id.trim(),
                merchant_id.trim(),
            ],
        )?;
        tx.commit()?;
        drop(conn);
        Ok((self.open_commerce_consumer_data_request(request_id)?, true))
    }

    fn open_commerce_consumer_data_request(
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

fn open_request_for_relationship(
    tx: &Transaction<'_>,
    relationship_id: &str,
) -> Result<Option<OpenCommerceConsumerDataRequest>> {
    tx.query_row(
        &format!(
            "{DATA_REQUEST_SELECT}
              WHERE relationship_id=?1 AND status IN ('requested', 'in_progress')
              ORDER BY updated_at DESC, rowid DESC LIMIT 1"
        ),
        params![relationship_id.trim()],
        data_request_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn consumer_owned_request(
    tx: &Transaction<'_>,
    consumer_project_id: &str,
    consumer_user_id: &str,
    request_id: &str,
) -> Result<Option<OpenCommerceConsumerDataRequest>> {
    tx.query_row(
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
    .optional()
    .map_err(Into::into)
}

fn merchant_owned_request(
    tx: &Transaction<'_>,
    merchant_project_id: &str,
    merchant_id: &str,
    request_id: &str,
) -> Result<Option<OpenCommerceConsumerDataRequest>> {
    tx.query_row(
        &format!(
            "{DATA_REQUEST_SELECT}
              WHERE id=?1 AND merchant_project_id=?2 AND merchant_id=?3"
        ),
        params![
            request_id.trim(),
            merchant_project_id.trim(),
            merchant_id.trim()
        ],
        data_request_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn data_request_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<OpenCommerceConsumerDataRequest> {
    Ok(OpenCommerceConsumerDataRequest {
        id: row.get(0)?,
        relationship_id: row.get(1)?,
        merchant_id: row.get(2)?,
        subject_alias: row.get(3)?,
        request_type: row.get(4)?,
        status: row.get(5)?,
        resolution_kind: row.get(6)?,
        resolution_note: row.get(7)?,
        requested_at: row.get(8)?,
        accepted_at: row.get(9)?,
        resolved_at: row.get(10)?,
        withdrawn_at: row.get(11)?,
        updated_at: row.get(12)?,
        operational_target_at: None,
        is_operationally_overdue: false,
        reminder_count: 0,
        last_reminded_at: None,
        next_reminder_at: None,
        consumer_escalated_at: None,
        can_send_reminder: false,
        can_escalate_attention: false,
    })
}

pub(super) const DATA_REQUEST_SELECT: &str = "SELECT id, relationship_id, merchant_id,
       subject_alias, request_type, status, resolution_kind, resolution_note,
       requested_at, accepted_at, resolved_at, withdrawn_at, updated_at
  FROM open_commerce_consumer_data_requests";
