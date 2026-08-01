use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::{
    open_commerce_grant_budget_model::{
        OpenCommerceGrantBudgetDecision, OpenCommerceGrantBudgetExceeded,
    },
    open_commerce_model::OpenCommerceInvocation,
};

use super::{now, Store};

struct GrantBudgetState {
    grant_id: String,
    max_invocations: Option<i64>,
    max_amount_micros: Option<i64>,
    budget_currency: String,
    used_invocations: i64,
    used_amount_micros: i64,
}

impl Store {
    pub(crate) fn reserve_open_commerce_grant_budget(
        &self,
        invocation: &OpenCommerceInvocation,
    ) -> Result<Option<OpenCommerceGrantBudgetDecision>> {
        let Some(grant_id) = invocation.grant_id.as_deref() else {
            return Ok(None);
        };
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut state = active_budget_state(
            &tx,
            grant_id,
            &invocation.merchant_id,
            &invocation.requester_app_id,
        )?;
        if state.max_invocations.is_none() && state.max_amount_micros.is_none() {
            tx.commit()?;
            return Ok(None);
        }
        if state.max_amount_micros.is_some()
            && state.budget_currency != invocation.currency.trim().to_ascii_uppercase()
        {
            bail!(
                "授权预算币种 {} 与能力币种 {} 不一致",
                state.budget_currency,
                invocation.currency
            );
        }
        if let Some(status) = tx
            .query_row(
                "SELECT status FROM open_commerce_grant_budget_reservations
                 WHERE invocation_id = ?1",
                params![invocation.id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            if status == "released" {
                bail!("调用预算预留已经释放，不能重复执行");
            }
            return Ok(Some(decision(&state, &invocation.id)));
        }

        let next_invocations = state
            .used_invocations
            .checked_add(1)
            .ok_or_else(|| anyhow!("授权调用计数溢出"))?;
        let next_amount = state
            .used_amount_micros
            .checked_add(invocation.unit_price_micros)
            .ok_or_else(|| anyhow!("授权计量金额溢出"))?;
        if let Some(limit) = state.max_invocations {
            if next_invocations > limit {
                return Err(OpenCommerceGrantBudgetExceeded {
                    grant_id: state.grant_id,
                    limit_kind: "invocations",
                    limit,
                    used: state.used_invocations,
                }
                .into());
            }
        }
        if let Some(limit) = state.max_amount_micros {
            if next_amount > limit {
                return Err(OpenCommerceGrantBudgetExceeded {
                    grant_id: state.grant_id,
                    limit_kind: "amount_micros",
                    limit,
                    used: state.used_amount_micros,
                }
                .into());
            }
        }

        tx.execute(
            "UPDATE open_commerce_grants
                SET used_invocations = ?1, used_amount_micros = ?2, updated_at = ?3
              WHERE id = ?4",
            params![next_invocations, next_amount, now(), state.grant_id],
        )?;
        tx.execute(
            "INSERT INTO open_commerce_grant_budget_reservations (
               invocation_id, grant_id, reserved_invocations,
               reserved_amount_micros, currency, status, created_at, completed_at
             ) VALUES (?1, ?2, 1, ?3, ?4, 'reserved', ?5, NULL)",
            params![
                invocation.id,
                state.grant_id,
                invocation.unit_price_micros,
                state.budget_currency,
                now()
            ],
        )?;
        state.used_invocations = next_invocations;
        state.used_amount_micros = next_amount;
        let result = decision(&state, &invocation.id);
        tx.commit()?;
        Ok(Some(result))
    }
}

pub(super) fn commit_grant_budget_reservation_on(
    tx: &Transaction<'_>,
    invocation_id: &str,
    timestamp: &str,
) -> Result<()> {
    tx.execute(
        "UPDATE open_commerce_grant_budget_reservations
            SET status = 'committed', completed_at = ?1
          WHERE invocation_id = ?2 AND status = 'reserved'",
        params![timestamp, invocation_id.trim()],
    )?;
    Ok(())
}

pub(super) fn release_grant_budget_reservation_on(
    tx: &Transaction<'_>,
    invocation_id: &str,
    timestamp: &str,
) -> Result<()> {
    let reservation = tx
        .query_row(
            "SELECT grant_id, reserved_invocations, reserved_amount_micros
             FROM open_commerce_grant_budget_reservations
             WHERE invocation_id = ?1 AND status = 'reserved'",
            params![invocation_id.trim()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((grant_id, invocations, amount_micros)) = reservation else {
        return Ok(());
    };
    let updated = tx.execute(
        "UPDATE open_commerce_grants
            SET used_invocations = used_invocations - ?1,
                used_amount_micros = used_amount_micros - ?2,
                updated_at = ?3
          WHERE id = ?4 AND used_invocations >= ?1 AND used_amount_micros >= ?2",
        params![invocations, amount_micros, timestamp, grant_id],
    )?;
    if updated != 1 {
        bail!("授权预算释放失败，计数状态不一致");
    }
    tx.execute(
        "UPDATE open_commerce_grant_budget_reservations
            SET status = 'released', completed_at = ?1
          WHERE invocation_id = ?2 AND status = 'reserved'",
        params![timestamp, invocation_id.trim()],
    )?;
    Ok(())
}

fn active_budget_state(
    tx: &Transaction<'_>,
    grant_id: &str,
    merchant_id: &str,
    requester_app_id: &str,
) -> Result<GrantBudgetState> {
    tx.query_row(
        "SELECT id, max_invocations, max_amount_micros, budget_currency,
                used_invocations, used_amount_micros
         FROM open_commerce_grants
         WHERE id = ?1 AND merchant_id = ?2 AND grantee_app_id = ?3
           AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > ?4)",
        params![
            grant_id.trim(),
            merchant_id.trim(),
            requester_app_id.trim(),
            now()
        ],
        |row| {
            Ok(GrantBudgetState {
                grant_id: row.get(0)?,
                max_invocations: row.get(1)?,
                max_amount_micros: row.get(2)?,
                budget_currency: row.get(3)?,
                used_invocations: row.get(4)?,
                used_amount_micros: row.get(5)?,
            })
        },
    )
    .map_err(|error| anyhow!(error).context("授权不存在、已撤销、已过期或不属于当前调用方"))
}

fn decision(state: &GrantBudgetState, invocation_id: &str) -> OpenCommerceGrantBudgetDecision {
    OpenCommerceGrantBudgetDecision {
        grant_id: state.grant_id.clone(),
        invocation_id: invocation_id.to_string(),
        max_invocations: state.max_invocations,
        max_amount_micros: state.max_amount_micros,
        budget_currency: state.budget_currency.clone(),
        used_invocations: state.used_invocations,
        used_amount_micros: state.used_amount_micros,
        remaining_invocations: state
            .max_invocations
            .map(|limit| (limit - state.used_invocations).max(0)),
        remaining_amount_micros: state
            .max_amount_micros
            .map(|limit| (limit - state.used_amount_micros).max(0)),
    }
}
