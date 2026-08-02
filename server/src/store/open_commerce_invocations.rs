use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Row};
use serde_json::Value;

use crate::open_commerce_model::{
    OpenCommerceAuditEvent, OpenCommerceInvocation, SETTLEMENT_RECORDED_NOT_CHARGED,
};

use super::{
    new_id, now,
    open_commerce_app_blocks::ensure_app_not_blocked_on,
    open_commerce_grant_budgets::{
        commit_grant_budget_reservation_on, release_grant_budget_reservation_on,
    },
    Store,
};

pub(crate) struct OpenCommerceInvocationStart<'a> {
    pub project_id: &'a str,
    pub merchant_id: &'a str,
    pub capability_id: &'a str,
    pub capability_key: &'a str,
    pub requester_user_id: &'a str,
    pub requester_app_id: &'a str,
    pub grant_id: Option<&'a str>,
    pub idempotency_key: &'a str,
    pub request_hash: &'a str,
    pub request_shape: &'a Value,
    pub unit_price_micros: i64,
    pub currency: &'a str,
}

pub(crate) struct OpenCommerceInvocationClaim {
    pub invocation: OpenCommerceInvocation,
    pub created: bool,
}

impl Store {
    pub(crate) fn start_open_commerce_invocation(
        &self,
        input: OpenCommerceInvocationStart<'_>,
    ) -> Result<OpenCommerceInvocationClaim> {
        let existing = self.find_open_commerce_invocation(
            input.requester_user_id,
            input.requester_app_id,
            input.merchant_id,
            input.capability_id,
            input.idempotency_key,
        )?;
        if let Some(invocation) = existing {
            if invocation.request_hash != input.request_hash {
                bail!("相同幂等键不能用于不同输入");
            }
            return Ok(OpenCommerceInvocationClaim {
                invocation,
                created: false,
            });
        }

        let id = new_id("invoke");
        let conn = self.conn()?;
        ensure_app_not_blocked_on(&conn, input.merchant_id, input.requester_app_id)?;
        conn.execute(
            "INSERT INTO open_commerce_invocations (
                    id, project_id, merchant_id, capability_id, capability_key,
                    requester_user_id, requester_app_id, grant_id, idempotency_key,
                    request_hash, request_shape_json, status, result_json, error_code,
                    units, unit_price_micros, amount_micros, currency,
                    settlement_status, created_at, completed_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'started',
                    NULL, NULL, 0, ?12, 0, ?13, ?14, ?15, NULL
                 )",
            params![
                id,
                input.project_id.trim(),
                input.merchant_id.trim(),
                input.capability_id.trim(),
                input.capability_key.trim(),
                input.requester_user_id.trim(),
                input.requester_app_id.trim(),
                input.grant_id,
                input.idempotency_key.trim(),
                input.request_hash,
                serde_json::to_string(input.request_shape)?,
                input.unit_price_micros,
                input.currency,
                SETTLEMENT_RECORDED_NOT_CHARGED,
                now()
            ],
        )
        .map_err(map_invocation_conflict)?;
        drop(conn);
        Ok(OpenCommerceInvocationClaim {
            invocation: self.open_commerce_invocation(&id)?,
            created: true,
        })
    }

    pub(crate) fn finish_open_commerce_invocation_success(
        &self,
        invocation_id: &str,
        result: &Value,
    ) -> Result<OpenCommerceInvocation> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let updated = tx.execute(
            "UPDATE open_commerce_invocations
                SET status = 'succeeded', result_json = ?1, error_code = NULL,
                    units = 1, amount_micros = unit_price_micros,
                    settlement_status = ?2, completed_at = ?3
              WHERE id = ?4 AND status = 'started'",
            params![
                serde_json::to_string(result)?,
                SETTLEMENT_RECORDED_NOT_CHARGED,
                timestamp,
                invocation_id.trim()
            ],
        )?;
        if updated == 0 {
            bail!("调用不存在或已经完成");
        }
        commit_grant_budget_reservation_on(&tx, invocation_id, &timestamp)?;
        tx.commit()?;
        drop(conn);
        self.open_commerce_invocation(invocation_id)
    }

    pub(crate) fn finish_open_commerce_invocation_failure(
        &self,
        invocation_id: &str,
        error_code: &str,
    ) -> Result<OpenCommerceInvocation> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let updated = tx.execute(
            "UPDATE open_commerce_invocations
                SET status = 'failed', result_json = NULL, error_code = ?1,
                    units = 0, amount_micros = 0,
                    settlement_status = ?2, completed_at = ?3
              WHERE id = ?4 AND status = 'started'",
            params![
                error_code.trim(),
                SETTLEMENT_RECORDED_NOT_CHARGED,
                timestamp,
                invocation_id.trim()
            ],
        )?;
        if updated == 0 {
            bail!("调用不存在或已经完成");
        }
        release_grant_budget_reservation_on(&tx, invocation_id, &timestamp)?;
        tx.commit()?;
        drop(conn);
        self.open_commerce_invocation(invocation_id)
    }

    pub(crate) fn open_commerce_invocation(
        &self,
        invocation_id: &str,
    ) -> Result<OpenCommerceInvocation> {
        self.conn()?
            .query_row(
                &format!("{INVOCATION_SELECT} WHERE id = ?1"),
                params![invocation_id.trim()],
                invocation_from_row,
            )
            .map_err(|error| anyhow!(error).context("调用记录不存在"))
    }

    pub(crate) fn list_project_open_commerce_invocations(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceInvocation>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{INVOCATION_SELECT}
             WHERE project_id = ?1 ORDER BY created_at DESC LIMIT ?2"
        ))?;
        let invocations = stmt
            .query_map(
                params![project_id.trim(), limit.clamp(1, 200) as i64],
                invocation_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(invocations)
    }

    pub(crate) fn record_open_commerce_audit(
        &self,
        project_id: &str,
        actor_user_id: &str,
        actor_app_id: Option<&str>,
        action: &str,
        subject_type: &str,
        subject_id: &str,
        metadata: &Value,
    ) -> Result<OpenCommerceAuditEvent> {
        if !metadata.is_object() {
            bail!("审计 metadata 必须是 JSON object");
        }
        let metadata_json = serde_json::to_string(metadata)?;
        if metadata_json.len() > 16 * 1024 {
            bail!("审计 metadata 不能超过 16 KiB");
        }
        let id = new_id("audit");
        self.conn()?.execute(
            "INSERT INTO open_commerce_audit_events (
                id, project_id, actor_user_id, actor_app_id, action,
                subject_type, subject_id, metadata_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                project_id.trim(),
                actor_user_id.trim(),
                actor_app_id.map(str::trim),
                action.trim(),
                subject_type.trim(),
                subject_id.trim(),
                metadata_json,
                now()
            ],
        )?;
        self.open_commerce_audit_event(&id)
    }

    pub(crate) fn list_project_open_commerce_audit(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceAuditEvent>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{AUDIT_SELECT}
             WHERE project_id = ?1 ORDER BY created_at DESC LIMIT ?2"
        ))?;
        let events = stmt
            .query_map(
                params![project_id.trim(), limit.clamp(1, 200) as i64],
                audit_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(events)
    }

    fn find_open_commerce_invocation(
        &self,
        requester_user_id: &str,
        requester_app_id: &str,
        merchant_id: &str,
        capability_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<OpenCommerceInvocation>> {
        self.conn()?
            .query_row(
                &format!(
                    "{INVOCATION_SELECT}
                     WHERE requester_user_id = ?1 AND requester_app_id = ?2
                       AND merchant_id = ?3 AND capability_id = ?4
                       AND idempotency_key = ?5"
                ),
                params![
                    requester_user_id.trim(),
                    requester_app_id.trim(),
                    merchant_id.trim(),
                    capability_id.trim(),
                    idempotency_key.trim()
                ],
                invocation_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    fn open_commerce_audit_event(&self, audit_id: &str) -> Result<OpenCommerceAuditEvent> {
        self.conn()?
            .query_row(
                &format!("{AUDIT_SELECT} WHERE id = ?1"),
                params![audit_id],
                audit_from_row,
            )
            .map_err(Into::into)
    }
}

pub(super) fn invocation_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceInvocation> {
    Ok(OpenCommerceInvocation {
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        capability_id: row.get(3)?,
        capability_key: row.get(4)?,
        requester_user_id: row.get(5)?,
        requester_app_id: row.get(6)?,
        grant_id: row.get(7)?,
        idempotency_key: row.get(8)?,
        request_hash: row.get(9)?,
        request_shape: parse_json(row.get(10)?, "调用摘要 JSON 无效")?,
        status: row.get(11)?,
        result: row
            .get::<_, Option<String>>(12)?
            .map(|value| parse_json(value, "调用结果 JSON 无效"))
            .transpose()?,
        error_code: row.get(13)?,
        units: row.get(14)?,
        unit_price_micros: row.get(15)?,
        amount_micros: row.get(16)?,
        currency: row.get(17)?,
        settlement_status: row.get(18)?,
        created_at: row.get(19)?,
        completed_at: row.get(20)?,
    })
}

fn audit_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceAuditEvent> {
    Ok(OpenCommerceAuditEvent {
        id: row.get(0)?,
        project_id: row.get(1)?,
        actor_user_id: row.get(2)?,
        actor_app_id: row.get(3)?,
        action: row.get(4)?,
        subject_type: row.get(5)?,
        subject_id: row.get(6)?,
        metadata: parse_json(row.get(7)?, "审计 metadata JSON 无效")?,
        created_at: row.get(8)?,
    })
}

fn parse_json(value: String, label: &str) -> rusqlite::Result<Value> {
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            anyhow!("{label}: {error}").into(),
        )
    })
}

pub(super) fn map_invocation_conflict(error: rusqlite::Error) -> anyhow::Error {
    if error.to_string().contains("UNIQUE constraint failed") {
        anyhow!("调用幂等键发生并发冲突，请读取已有调用结果")
    } else {
        anyhow!(error)
    }
}

pub(super) const INVOCATION_SELECT: &str =
    "SELECT id, project_id, merchant_id, capability_id, capability_key,
            requester_user_id, requester_app_id, grant_id, idempotency_key,
            request_hash, request_shape_json, status, result_json, error_code,
            units, unit_price_micros, amount_micros, currency, settlement_status,
            created_at, completed_at
       FROM open_commerce_invocations";

const AUDIT_SELECT: &str = "SELECT id, project_id, actor_user_id, actor_app_id, action,
            subject_type, subject_id, metadata_json, created_at
       FROM open_commerce_audit_events";
