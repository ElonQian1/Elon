use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension, Row, Transaction, TransactionBehavior};
use serde_json::Value;

use crate::{
    open_commerce_action_confirmation_model::{
        OpenCommerceActionConfirmation, ACTION_CONFIRMATION_RETENTION_DAYS,
        MAX_ACTIVE_ACTION_CONFIRMATIONS_PER_APP,
    },
    open_commerce_model::SETTLEMENT_RECORDED_NOT_CHARGED,
};

use super::{
    new_id, now,
    open_commerce_app_blocks::ensure_app_not_blocked_on,
    open_commerce_invocations::{
        invocation_from_row, map_invocation_conflict, OpenCommerceInvocationClaim,
        OpenCommerceInvocationStart, INVOCATION_SELECT,
    },
    Store,
};

pub(crate) struct CreateOpenCommerceActionConfirmation<'a> {
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
    pub expires_at: &'a str,
}

impl Store {
    pub(crate) fn create_open_commerce_action_confirmation(
        &self,
        input: CreateOpenCommerceActionConfirmation<'_>,
    ) -> Result<OpenCommerceActionConfirmation> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let timestamp = now();
        tx.execute(
            "UPDATE open_commerce_action_confirmations
                SET status = 'expired'
              WHERE status IN ('pending', 'confirmed') AND expires_at <= ?1",
            params![timestamp],
        )?;
        let retention_cutoff =
            (Utc::now() - Duration::days(ACTION_CONFIRMATION_RETENTION_DAYS)).to_rfc3339();
        tx.execute(
            "DELETE FROM open_commerce_action_confirmations
              WHERE status = 'expired' AND invocation_id IS NULL AND created_at < ?1",
            params![retention_cutoff],
        )?;

        let existing = tx
            .query_row(
                &format!(
                    "{CONFIRMATION_SELECT}
                     WHERE requester_user_id = ?1 AND requester_app_id = ?2
                       AND merchant_id = ?3 AND capability_id = ?4
                       AND idempotency_key = ?5
                     ORDER BY created_at DESC LIMIT 1"
                ),
                params![
                    input.requester_user_id.trim(),
                    input.requester_app_id.trim(),
                    input.merchant_id.trim(),
                    input.capability_id.trim(),
                    input.idempotency_key.trim(),
                ],
                confirmation_from_row,
            )
            .optional()?;
        if let Some(existing) = existing {
            let same_grant =
                existing.grant_id.as_deref().map(str::trim) == input.grant_id.map(str::trim);
            if existing.request_hash != input.request_hash.trim() || !same_grant {
                bail!("相同幂等键不能用于不同输入或授权");
            }
            match existing.status.as_str() {
                "pending" | "confirmed" => {
                    tx.commit()?;
                    return Ok(existing);
                }
                "consumed" if existing.invocation_id.is_some() => {
                    tx.commit()?;
                    return Ok(existing);
                }
                "consumed" => bail!("已消费动作确认缺少调用记录"),
                "expired" => {}
                _ => bail!("动作确认状态无效"),
            }
        }

        let active_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM open_commerce_action_confirmations
              WHERE requester_user_id = ?1 AND requester_app_id = ?2
                AND status IN ('pending', 'confirmed') AND expires_at > ?3",
            params![
                input.requester_user_id.trim(),
                input.requester_app_id.trim(),
                timestamp,
            ],
            |row| row.get(0),
        )?;
        if active_count >= MAX_ACTIVE_ACTION_CONFIRMATIONS_PER_APP {
            bail!("当前用户与应用的活动动作确认过多，请先完成或等待过期");
        }

        let id = new_id("action_confirm");
        tx.execute(
            "INSERT INTO open_commerce_action_confirmations (
                id, project_id, merchant_id, capability_id, capability_key,
                requester_user_id, requester_app_id, grant_id, idempotency_key,
                request_hash, request_shape_json, status, expires_at, created_at,
                confirmed_at, consumed_at, invocation_id
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'pending',
                ?12, ?13, NULL, NULL, NULL
             )",
            params![
                id,
                input.project_id.trim(),
                input.merchant_id.trim(),
                input.capability_id.trim(),
                input.capability_key.trim(),
                input.requester_user_id.trim(),
                input.requester_app_id.trim(),
                input.grant_id.map(str::trim),
                input.idempotency_key.trim(),
                input.request_hash.trim(),
                serde_json::to_string(input.request_shape)?,
                input.expires_at.trim(),
                timestamp,
            ],
        )?;
        let confirmation = confirmation_on(&tx, &id)?;
        tx.commit()?;
        Ok(confirmation)
    }

    pub(crate) fn open_commerce_action_confirmation(
        &self,
        confirmation_id: &str,
    ) -> Result<OpenCommerceActionConfirmation> {
        self.conn()?
            .query_row(
                &format!("{CONFIRMATION_SELECT} WHERE id = ?1"),
                params![confirmation_id.trim()],
                confirmation_from_row,
            )
            .map_err(|error| anyhow!(error).context("动作确认不存在"))
    }

    pub(crate) fn confirm_open_commerce_action_confirmation(
        &self,
        confirmation_id: &str,
        requester_user_id: &str,
        requester_app_id: &str,
    ) -> Result<OpenCommerceActionConfirmation> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let confirmation = confirmation_on(&tx, confirmation_id)?;
        ensure_actor(&confirmation, requester_user_id, requester_app_id)?;
        match confirmation.status.as_str() {
            "pending" if is_expired(&confirmation.expires_at)? => {
                tx.execute(
                    "UPDATE open_commerce_action_confirmations
                        SET status = 'expired' WHERE id = ?1 AND status = 'pending'",
                    params![confirmation.id],
                )?;
                tx.commit()?;
                bail!("动作确认已过期，请重新发起确认");
            }
            "pending" => {
                let timestamp = now();
                tx.execute(
                    "UPDATE open_commerce_action_confirmations
                        SET status = 'confirmed', confirmed_at = ?1
                      WHERE id = ?2 AND status = 'pending'",
                    params![timestamp, confirmation.id],
                )?;
            }
            "confirmed" | "consumed" => {}
            "expired" => bail!("动作确认已过期，请重新发起确认"),
            _ => bail!("动作确认状态无效"),
        }
        tx.commit()?;
        drop(conn);
        self.open_commerce_action_confirmation(confirmation_id)
    }

    pub(crate) fn start_confirmed_open_commerce_invocation(
        &self,
        input: OpenCommerceInvocationStart<'_>,
        confirmation_id: &str,
    ) -> Result<OpenCommerceInvocationClaim> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let confirmation = confirmation_on(&tx, confirmation_id)?;
        ensure_confirmation_binding(&confirmation, &input)?;

        let existing = find_invocation_on(&tx, &input)?;
        if let Some(invocation) = existing {
            if invocation.request_hash != input.request_hash {
                bail!("相同幂等键不能用于不同输入");
            }
            if confirmation.status != "consumed"
                || confirmation.invocation_id.as_deref() != Some(invocation.id.as_str())
            {
                bail!("动作确认未绑定当前幂等调用");
            }
            tx.commit()?;
            return Ok(OpenCommerceInvocationClaim {
                invocation,
                created: false,
            });
        }

        if confirmation.status != "confirmed" {
            bail!("动作能力必须先完成一次性确认");
        }
        if is_expired(&confirmation.expires_at)? {
            tx.execute(
                "UPDATE open_commerce_action_confirmations
                    SET status = 'expired' WHERE id = ?1 AND status = 'confirmed'",
                params![confirmation.id],
            )?;
            tx.commit()?;
            bail!("动作确认已过期，请重新发起确认");
        }

        ensure_app_not_blocked_on(&tx, input.merchant_id, input.requester_app_id)?;
        let invocation_id = new_id("invoke");
        insert_invocation_on(&tx, &invocation_id, &input)?;
        let timestamp = now();
        let consumed = tx.execute(
            "UPDATE open_commerce_action_confirmations
                SET status = 'consumed', consumed_at = ?1, invocation_id = ?2
              WHERE id = ?3 AND status = 'confirmed' AND invocation_id IS NULL",
            params![timestamp, invocation_id, confirmation.id],
        )?;
        if consumed != 1 {
            bail!("动作确认已被其他调用使用");
        }
        tx.commit()?;
        drop(conn);
        Ok(OpenCommerceInvocationClaim {
            invocation: self.open_commerce_invocation(&invocation_id)?,
            created: true,
        })
    }
}

fn ensure_actor(
    confirmation: &OpenCommerceActionConfirmation,
    requester_user_id: &str,
    requester_app_id: &str,
) -> Result<()> {
    if confirmation.requester_user_id != requester_user_id.trim()
        || confirmation.requester_app_id != requester_app_id.trim()
    {
        bail!("动作确认不属于当前用户和应用");
    }
    Ok(())
}

fn ensure_confirmation_binding(
    confirmation: &OpenCommerceActionConfirmation,
    input: &OpenCommerceInvocationStart<'_>,
) -> Result<()> {
    ensure_actor(
        confirmation,
        input.requester_user_id,
        input.requester_app_id,
    )?;
    let same_grant =
        confirmation.grant_id.as_deref().map(str::trim) == input.grant_id.map(str::trim);
    if confirmation.project_id != input.project_id.trim()
        || confirmation.merchant_id != input.merchant_id.trim()
        || confirmation.capability_id != input.capability_id.trim()
        || confirmation.capability_key != input.capability_key.trim()
        || confirmation.idempotency_key != input.idempotency_key.trim()
        || confirmation.request_hash != input.request_hash.trim()
        || !same_grant
    {
        bail!("动作确认与当前商户、能力、授权、幂等键或输入不一致");
    }
    Ok(())
}

fn is_expired(expires_at: &str) -> Result<bool> {
    Ok(DateTime::parse_from_rfc3339(expires_at)
        .context("动作确认过期时间无效")?
        .with_timezone(&Utc)
        <= Utc::now())
}

fn confirmation_on(
    tx: &Transaction<'_>,
    confirmation_id: &str,
) -> Result<OpenCommerceActionConfirmation> {
    tx.query_row(
        &format!("{CONFIRMATION_SELECT} WHERE id = ?1"),
        params![confirmation_id.trim()],
        confirmation_from_row,
    )
    .map_err(|error| anyhow!(error).context("动作确认不存在"))
}

fn find_invocation_on(
    tx: &Transaction<'_>,
    input: &OpenCommerceInvocationStart<'_>,
) -> Result<Option<crate::open_commerce_model::OpenCommerceInvocation>> {
    tx.query_row(
        &format!(
            "{INVOCATION_SELECT}
             WHERE requester_user_id = ?1 AND requester_app_id = ?2
               AND merchant_id = ?3 AND capability_id = ?4 AND idempotency_key = ?5"
        ),
        params![
            input.requester_user_id.trim(),
            input.requester_app_id.trim(),
            input.merchant_id.trim(),
            input.capability_id.trim(),
            input.idempotency_key.trim(),
        ],
        invocation_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn insert_invocation_on(
    tx: &Transaction<'_>,
    invocation_id: &str,
    input: &OpenCommerceInvocationStart<'_>,
) -> Result<()> {
    tx.execute(
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
            invocation_id,
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
            now(),
        ],
    )
    .map_err(map_invocation_conflict)?;
    Ok(())
}

fn confirmation_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceActionConfirmation> {
    Ok(OpenCommerceActionConfirmation {
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
        request_shape: parse_json(row.get(10)?)?,
        status: row.get(11)?,
        expires_at: row.get(12)?,
        created_at: row.get(13)?,
        confirmed_at: row.get(14)?,
        consumed_at: row.get(15)?,
        invocation_id: row.get(16)?,
    })
}

fn parse_json(value: String) -> rusqlite::Result<Value> {
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            anyhow!("动作确认调用摘要 JSON 无效: {error}").into(),
        )
    })
}

const CONFIRMATION_SELECT: &str =
    "SELECT id, project_id, merchant_id, capability_id, capability_key,
            requester_user_id, requester_app_id, grant_id, idempotency_key,
            request_hash, request_shape_json, status, expires_at, created_at,
            confirmed_at, consumed_at, invocation_id
       FROM open_commerce_action_confirmations";
