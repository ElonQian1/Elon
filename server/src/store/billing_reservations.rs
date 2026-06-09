//! Billing call reservations and reconciliation helpers.
//!
//! A reservation is a short-lived balance hold keyed by the same compute call id
//! used as trusted token-usage idempotency key. Final token accounting settles
//! the hold by refunding the unused amount or deducting the delta.

use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::Serialize;

use super::{new_id, now, Store};

#[derive(Debug, Clone)]
pub struct BillingReservationRequest<'a> {
    pub user_id: &'a str,
    pub compute_call_id: &'a str,
    pub feature: &'a str,
    pub usage_mode: &'a str,
    pub model: Option<&'a str>,
    pub reserve_fen: i64,
    pub bill_missing_balance: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingReservationOutcome {
    pub reservation_id: String,
    pub compute_call_id: String,
    pub reserved_fen: i64,
    pub balance_after_fen: Option<i64>,
    pub status: String,
    pub deduplicated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingReconciliationSummary {
    pub period_days: i64,
    pub unbilled_events: i64,
    pub legacy_events: i64,
    pub duplicate_idempotency_keys: i64,
    pub negative_balance_users: i64,
    pub open_reservations: i64,
    pub expired_reservations: i64,
    pub reserved_fen_open: i64,
    pub billed_cost_rmb_fen_period: i64,
}

#[derive(Debug)]
pub(super) struct BillingReservationForSettlement {
    pub id: String,
    pub reserved_fen: i64,
}

impl Store {
    pub fn release_expired_billing_reservations(&self) -> Result<usize> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let rows = {
            let mut stmt = tx.prepare(
                "SELECT id, user_id, reserved_fen
                 FROM billing_reservations
                 WHERE status = 'reserved'
                   AND expires_at IS NOT NULL
                   AND expires_at < ?1",
            )?;
            let mapped = stmt.query_map(params![&ts], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;
            mapped.collect::<rusqlite::Result<Vec<_>>>()?
        };
        for (id, user_id, reserved_fen) in &rows {
            let balance_after = refund_reserved_fen(&tx, user_id, *reserved_fen, &ts)?;
            tx.execute(
                "UPDATE billing_reservations
                 SET status = 'expired_released',
                     refunded_fen = reserved_fen,
                     balance_after_fen = ?2,
                     updated_at = ?3
                 WHERE id = ?1 AND status = 'reserved'",
                params![id, balance_after, &ts],
            )?;
        }
        tx.commit()?;
        Ok(rows.len())
    }

    pub fn reserve_billing_call(
        &self,
        request: &BillingReservationRequest<'_>,
    ) -> Result<BillingReservationOutcome> {
        let compute_call_id = normalize_compute_call_id(request.compute_call_id)?;
        let reserve_fen = request.reserve_fen.max(0);
        let ts = now();
        let expires_at = reservation_expires_at();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        if let Some(existing) = tx
            .query_row(
                "SELECT id, reserved_fen, balance_after_fen, status
                 FROM billing_reservations
                 WHERE user_id = ?1 AND compute_call_id = ?2",
                params![request.user_id, compute_call_id],
                |row| {
                    Ok(BillingReservationOutcome {
                        reservation_id: row.get(0)?,
                        compute_call_id: compute_call_id.clone(),
                        reserved_fen: row.get(1)?,
                        balance_after_fen: row.get(2)?,
                        status: row.get(3)?,
                        deduplicated: true,
                    })
                },
            )
            .optional()?
        {
            tx.commit()?;
            return Ok(existing);
        }

        let mut balance = tx
            .query_row(
                "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
                params![request.user_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        if balance.is_none() && request.bill_missing_balance {
            tx.execute(
                "INSERT INTO user_balance (user_id, balance_fen, updated_at) VALUES (?1, 0, ?2)",
                params![request.user_id, ts],
            )?;
            balance = Some(0);
        }

        let (status, balance_after) = if let Some(balance) = balance {
            if balance < reserve_fen {
                return Err(anyhow!(
                    "余额不足（当前 {} 分，需要至少 {} 分预授权），请联系管理员充值后继续使用",
                    balance,
                    reserve_fen
                ));
            }
            let balance_after = balance - reserve_fen;
            tx.execute(
                "UPDATE user_balance SET balance_fen = ?1, updated_at = ?2 WHERE user_id = ?3",
                params![balance_after, ts, request.user_id],
            )?;
            ("reserved", Some(balance_after))
        } else {
            ("skipped_no_balance", None)
        };

        let reservation_id = new_id("brv");
        tx.execute(
            "INSERT INTO billing_reservations (
               id, user_id, compute_call_id, feature, usage_mode, model,
               reserved_fen, settled_cost_fen, refunded_fen, status,
               created_at, updated_at, expires_at, balance_after_fen
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,0,0,?8,?9,?9,?10,?11)",
            params![
                reservation_id,
                request.user_id,
                compute_call_id,
                request.feature,
                request.usage_mode,
                request.model,
                reserve_fen,
                status,
                ts,
                expires_at,
                balance_after,
            ],
        )?;
        tx.commit()?;

        Ok(BillingReservationOutcome {
            reservation_id,
            compute_call_id,
            reserved_fen: reserve_fen,
            balance_after_fen: balance_after,
            status: status.to_string(),
            deduplicated: false,
        })
    }

    pub fn release_billing_call(
        &self,
        user_id: &str,
        compute_call_id: &str,
        status: &str,
    ) -> Result<Option<BillingReservationOutcome>> {
        let compute_call_id = normalize_compute_call_id(compute_call_id)?;
        let release_status = normalize_release_status(status);
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let existing = tx
            .query_row(
                "SELECT id, reserved_fen
                 FROM billing_reservations
                 WHERE user_id = ?1 AND compute_call_id = ?2 AND status = 'reserved'",
                params![user_id, compute_call_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;
        let Some((reservation_id, reserved_fen)) = existing else {
            tx.commit()?;
            return Ok(None);
        };

        let balance_after = refund_reserved_fen(&tx, user_id, reserved_fen, &ts)?;
        tx.execute(
            "UPDATE billing_reservations
             SET status = ?2,
                 refunded_fen = reserved_fen,
                 balance_after_fen = ?3,
                 updated_at = ?4
             WHERE id = ?1 AND status = 'reserved'",
            params![reservation_id, release_status, balance_after, ts],
        )?;
        tx.commit()?;

        Ok(Some(BillingReservationOutcome {
            reservation_id,
            compute_call_id,
            reserved_fen,
            balance_after_fen: Some(balance_after),
            status: release_status.to_string(),
            deduplicated: false,
        }))
    }

    pub fn admin_billing_reconciliation_summary(
        &self,
        days: i64,
    ) -> Result<BillingReconciliationSummary> {
        let conn = self.conn()?;
        let days = days.clamp(1, 365);
        let since = format!("-{} days", days);
        let now_ts = now();
        let (
            unbilled_events,
            legacy_events,
            duplicate_idempotency_keys,
            negative_balance_users,
            open_reservations,
            expired_reservations,
            reserved_fen_open,
            billed_cost,
        ): (i64, i64, i64, i64, i64, i64, i64, i64) = conn.query_row(
            "SELECT
               (SELECT COUNT(*) FROM token_usage_events
                WHERE usage_mode != 'client_reported'
                  AND accounting_status = 'unbilled_no_balance'
                  AND created_at >= datetime('now', ?1)),
               (SELECT COUNT(*) FROM token_usage_events
                WHERE usage_mode != 'client_reported'
                  AND accounting_status = 'legacy'
                  AND created_at >= datetime('now', ?1)),
               (SELECT COUNT(*) FROM (
                  SELECT user_id, idempotency_key
                  FROM token_usage_events
                  WHERE idempotency_key IS NOT NULL
                  GROUP BY user_id, idempotency_key
                  HAVING COUNT(*) > 1
                )),
               (SELECT COUNT(*) FROM user_balance WHERE balance_fen < 0),
               (SELECT COUNT(*) FROM billing_reservations WHERE status = 'reserved'),
               (SELECT COUNT(*) FROM billing_reservations
                WHERE status = 'reserved' AND expires_at IS NOT NULL AND expires_at < ?2),
               (SELECT COALESCE(SUM(reserved_fen),0) FROM billing_reservations WHERE status = 'reserved'),
               (SELECT COALESCE(SUM(cost_rmb_fen),0) FROM token_usage_events
                WHERE usage_mode != 'client_reported'
                  AND created_at >= datetime('now', ?1))",
            params![since, now_ts],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )?;

        Ok(BillingReconciliationSummary {
            period_days: days,
            unbilled_events,
            legacy_events,
            duplicate_idempotency_keys,
            negative_balance_users,
            open_reservations,
            expired_reservations,
            reserved_fen_open,
            billed_cost_rmb_fen_period: billed_cost,
        })
    }
}

pub(super) fn load_reservation_for_settlement(
    tx: &Transaction<'_>,
    user_id: &str,
    compute_call_id: &str,
) -> Result<Option<BillingReservationForSettlement>> {
    let compute_call_id = normalize_compute_call_id(compute_call_id)?;
    tx.query_row(
        "SELECT id, reserved_fen
         FROM billing_reservations
         WHERE user_id = ?1 AND compute_call_id = ?2 AND status = 'reserved'",
        params![user_id, compute_call_id],
        |row| {
            Ok(BillingReservationForSettlement {
                id: row.get(0)?,
                reserved_fen: row.get(1)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn mark_reservation_settled(
    tx: &Transaction<'_>,
    reservation_id: &str,
    token_usage_event_id: &str,
    billing_event_id: Option<&str>,
    actual_cost_fen: i64,
    refunded_fen: i64,
    ts: &str,
) -> Result<()> {
    tx.execute(
        "UPDATE billing_reservations
         SET status = 'settled',
             settled_cost_fen = ?2,
             refunded_fen = ?3,
             token_usage_event_id = ?4,
             billing_event_id = ?5,
             updated_at = ?6
         WHERE id = ?1",
        params![
            reservation_id,
            actual_cost_fen.max(0),
            refunded_fen.max(0),
            token_usage_event_id,
            billing_event_id,
            ts,
        ],
    )?;
    Ok(())
}

fn normalize_compute_call_id(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("compute_call_id 不能为空"));
    }
    Ok(value.chars().take(200).collect())
}

fn reservation_expires_at() -> String {
    (chrono::Utc::now() + chrono::Duration::minutes(30)).to_rfc3339()
}

fn refund_reserved_fen(
    tx: &Transaction<'_>,
    user_id: &str,
    reserved_fen: i64,
    ts: &str,
) -> Result<i64> {
    tx.execute(
        "UPDATE user_balance
         SET balance_fen = balance_fen + ?1,
             updated_at = ?2
         WHERE user_id = ?3",
        params![reserved_fen.max(0), ts, user_id],
    )?;
    let balance_after = tx
        .query_row(
            "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
            params![user_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(balance_after)
}

fn normalize_release_status(status: &str) -> &str {
    match status {
        "released_no_usage" | "released_error" | "expired_released" => status,
        _ => "released_error",
    }
}
