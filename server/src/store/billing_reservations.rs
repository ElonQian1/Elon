//! Billing call reservations and reconciliation helpers.
//!
//! A reservation starts as a short-lived pre-dispatch balance hold keyed by the
//! same compute call id used as trusted token-usage idempotency key. Before work
//! can reach a node it becomes a durable, amount-bounded dispatch hold. Final
//! token accounting settles the hold by refunding the unused amount or deducting
//! the delta.

use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::Serialize;

use super::{now, Store};

mod release;
mod reserve;

use release::release_billing_call_compat_on;
pub(super) use release::release_billing_call_reservation_on;
use reserve::reserve_billing_call_compat_on;
pub(super) use reserve::reserve_billing_call_until_on;

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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActiveBillingReservation {
    pub reservation_id: String,
    pub user_id: String,
    pub compute_call_id: String,
    pub reserved_fen: i64,
    pub expires_at: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
pub struct AdminBillingReservationRow {
    pub id: String,
    pub user_id: String,
    pub account: Option<String>,
    pub nickname: Option<String>,
    pub compute_call_id: String,
    pub feature: String,
    pub usage_mode: String,
    pub model: Option<String>,
    pub reserved_fen: i64,
    pub settled_cost_fen: i64,
    pub refunded_fen: i64,
    pub status: String,
    pub token_usage_event_id: Option<String>,
    pub billing_event_id: Option<String>,
    pub balance_after_fen: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug)]
pub(super) struct BillingReservationForSettlement {
    pub id: String,
    pub reserved_fen: i64,
}

impl Store {
    pub fn get_active_billing_reservation(
        &self,
        user_id: &str,
        compute_call_id: &str,
    ) -> Result<Option<ActiveBillingReservation>> {
        let user_id = normalize_required_id("user_id", user_id)?;
        let compute_call_id = normalize_compute_call_id(compute_call_id)?;
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, compute_call_id, reserved_fen, expires_at
               FROM billing_reservations
              WHERE user_id = ?1
                AND compute_call_id = ?2
                AND (
                  status IN ('dispatch_hold', 'verification_hold')
                  OR (status = 'reserved' AND (expires_at IS NULL OR expires_at >= ?3))
                )",
            params![user_id, compute_call_id, ts],
            |row| {
                Ok(ActiveBillingReservation {
                    reservation_id: row.get(0)?,
                    user_id: row.get(1)?,
                    compute_call_id: row.get(2)?,
                    reserved_fen: row.get(3)?,
                    expires_at: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    /// Commits the crash-safety boundary before a prompt can be sent to a node.
    /// The originally reserved amount and authorization deadline remain frozen,
    /// while the expiry janitor can no longer refund work whose execution
    /// outcome is not yet known. Keeping `expires_at` is important: it is also
    /// the absolute execution deadline sent to cloud-controlled nodes.
    pub fn hold_billing_reservation_for_dispatch(
        &self,
        user_id: &str,
        compute_call_id: &str,
    ) -> Result<Option<ActiveBillingReservation>> {
        let user_id = normalize_required_id("user_id", user_id)?;
        let compute_call_id = normalize_compute_call_id(compute_call_id)?;
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE billing_reservations
                SET status = 'dispatch_hold',
                    updated_at = ?3
              WHERE user_id = ?1
                AND compute_call_id = ?2
                AND (
                  status = 'dispatch_hold'
                  OR (status = 'reserved' AND (expires_at IS NULL OR expires_at >= ?3))
                )",
            params![user_id, compute_call_id, ts],
        )?;
        let held = active_hold_with_status(&tx, &user_id, &compute_call_id, "dispatch_hold")?;
        tx.commit()?;
        Ok(held)
    }

    /// Converts a bounded reservation into a non-expiring manual verification
    /// hold without refunding or increasing the originally reserved amount.
    /// The transaction serializes against the expiry janitor.
    pub fn hold_billing_reservation_for_verification(
        &self,
        user_id: &str,
        compute_call_id: &str,
    ) -> Result<Option<ActiveBillingReservation>> {
        let user_id = normalize_required_id("user_id", user_id)?;
        let compute_call_id = normalize_compute_call_id(compute_call_id)?;
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE billing_reservations
                SET status = 'verification_hold',
                    expires_at = NULL,
                    updated_at = ?3
              WHERE user_id = ?1
                AND compute_call_id = ?2
                AND status IN ('reserved', 'dispatch_hold', 'verification_hold')",
            params![user_id, compute_call_id, ts],
        )?;
        let held = tx
            .query_row(
                "SELECT id, user_id, compute_call_id, reserved_fen, expires_at
                   FROM billing_reservations
                  WHERE user_id = ?1
                    AND compute_call_id = ?2
                    AND status = 'verification_hold'",
                params![user_id, compute_call_id],
                |row| {
                    Ok(ActiveBillingReservation {
                        reservation_id: row.get(0)?,
                        user_id: row.get(1)?,
                        compute_call_id: row.get(2)?,
                        reserved_fen: row.get(3)?,
                        expires_at: row.get(4)?,
                    })
                },
            )
            .optional()?;
        tx.commit()?;
        Ok(held)
    }

    /// Refunds a durable dispatch hold only when the caller has positive proof
    /// that no prompt was ever enqueued to the node writer.
    pub fn release_dispatch_billing_hold_before_send(
        &self,
        user_id: &str,
        compute_call_id: &str,
    ) -> Result<Option<BillingReservationOutcome>> {
        self.release_held_billing_call(
            user_id,
            compute_call_id,
            &["dispatch_hold"],
            "released_dispatch_not_sent",
        )
    }

    /// Explicit operator/verified-failure escape hatch for a hold that cannot
    /// be settled from a trusted completion. It is intentionally separate from
    /// ordinary error/no-usage release and the automatic expiry janitor.
    pub fn release_billing_hold_after_manual_verification(
        &self,
        user_id: &str,
        compute_call_id: &str,
    ) -> Result<Option<BillingReservationOutcome>> {
        self.release_held_billing_call(
            user_id,
            compute_call_id,
            &["dispatch_hold", "verification_hold"],
            "released_manual_verified",
        )
    }

    pub fn billing_reservation_is_still_reserved(
        &self,
        user_id: &str,
        compute_call_id: &str,
    ) -> Result<bool> {
        Ok(self
            .get_active_billing_reservation(user_id, compute_call_id)?
            .is_some())
    }

    pub fn release_expired_billing_reservations(&self) -> Result<usize> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let rows = {
            let mut stmt = tx.prepare(
                "SELECT b.id, b.user_id, b.reserved_fen
                 FROM billing_reservations AS b
                 WHERE b.status = 'reserved'
                   AND b.expires_at IS NOT NULL
                   AND b.expires_at < ?1
                   AND NOT EXISTS (
                     SELECT 1 FROM compute_broker_reserve_receipts AS broker
                      WHERE broker.budget_reservation_id = b.id
                   )",
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
        let expires_at = reservation_expires_at();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let outcome = reserve_billing_call_compat_on(&tx, request, &expires_at)?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn release_billing_call(
        &self,
        user_id: &str,
        compute_call_id: &str,
        status: &str,
    ) -> Result<Option<BillingReservationOutcome>> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let outcome = release_billing_call_compat_on(&tx, user_id, compute_call_id, status)?;
        tx.commit()?;
        Ok(outcome)
    }

    fn release_held_billing_call(
        &self,
        user_id: &str,
        compute_call_id: &str,
        allowed_statuses: &[&str],
        release_status: &str,
    ) -> Result<Option<BillingReservationOutcome>> {
        let user_id = normalize_required_id("user_id", user_id)?;
        let compute_call_id = normalize_compute_call_id(compute_call_id)?;
        let ts = now();
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let existing = tx
            .query_row(
                "SELECT id, reserved_fen, status
                   FROM billing_reservations
                  WHERE user_id = ?1 AND compute_call_id = ?2",
                params![user_id, compute_call_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        let Some((reservation_id, reserved_fen, current_status)) = existing else {
            tx.commit()?;
            return Ok(None);
        };
        if !allowed_statuses.contains(&current_status.as_str()) {
            tx.commit()?;
            return Ok(None);
        }

        let balance_after = refund_reserved_fen(&tx, &user_id, reserved_fen, &ts)?;
        let changed = tx.execute(
            "UPDATE billing_reservations
                SET status = ?2,
                    refunded_fen = reserved_fen,
                    balance_after_fen = ?3,
                    updated_at = ?4
              WHERE id = ?1 AND status = ?5",
            params![
                reservation_id,
                release_status,
                balance_after,
                ts,
                current_status
            ],
        )?;
        if changed != 1 {
            return Err(anyhow!("billing hold changed during verified release"));
        }
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
               (SELECT COUNT(*) FROM billing_reservations
                 WHERE status IN ('reserved', 'dispatch_hold', 'verification_hold')),
               (SELECT COUNT(*) FROM billing_reservations
                WHERE status = 'reserved' AND expires_at IS NOT NULL AND expires_at < ?2),
               (SELECT COALESCE(SUM(reserved_fen),0) FROM billing_reservations
                WHERE status IN ('reserved', 'dispatch_hold', 'verification_hold')),
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

    pub fn admin_billing_reservations(
        &self,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AdminBillingReservationRow>> {
        let conn = self.conn()?;
        let limit = limit.clamp(1, 500);
        let status = status
            .map(str::trim)
            .filter(|value| !value.is_empty() && *value != "all");
        let base_select = r#"SELECT b.id, b.user_id, COALESCE(u.phone, u.email), u.nickname,
                                    b.compute_call_id, b.feature, b.usage_mode, b.model,
                                    b.reserved_fen, b.settled_cost_fen, b.refunded_fen,
                                    b.status, b.token_usage_event_id, b.billing_event_id,
                                    b.balance_after_fen, b.created_at, b.updated_at, b.expires_at
                             FROM billing_reservations b
                             LEFT JOIN users u ON u.id = b.user_id"#;
        let sql = if status.is_some() {
            format!("{base_select} WHERE b.status = ?1 ORDER BY b.updated_at DESC LIMIT ?2")
        } else {
            format!("{base_select} ORDER BY b.updated_at DESC LIMIT ?1")
        };
        let mut stmt = conn.prepare(&sql)?;
        let read_row = |row: &rusqlite::Row<'_>| {
            Ok(AdminBillingReservationRow {
                id: row.get(0)?,
                user_id: row.get(1)?,
                account: row.get(2)?,
                nickname: row.get(3)?,
                compute_call_id: row.get(4)?,
                feature: row.get(5)?,
                usage_mode: row.get(6)?,
                model: row.get(7)?,
                reserved_fen: row.get(8)?,
                settled_cost_fen: row.get(9)?,
                refunded_fen: row.get(10)?,
                status: row.get(11)?,
                token_usage_event_id: row.get(12)?,
                billing_event_id: row.get(13)?,
                balance_after_fen: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
                expires_at: row.get(17)?,
            })
        };
        let rows = if let Some(status) = status {
            stmt.query_map(params![status, limit], read_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![limit], read_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(rows)
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
         WHERE user_id = ?1
           AND compute_call_id = ?2
           AND status IN ('reserved', 'dispatch_hold', 'verification_hold')",
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

pub(super) fn load_active_reservation_for_settlement(
    tx: &Transaction<'_>,
    user_id: &str,
    compute_call_id: &str,
    active_at: &str,
) -> Result<Option<BillingReservationForSettlement>> {
    let compute_call_id = normalize_compute_call_id(compute_call_id)?;
    tx.query_row(
        "SELECT id, reserved_fen
         FROM billing_reservations
         WHERE user_id = ?1
           AND compute_call_id = ?2
           AND (
             status IN ('dispatch_hold', 'verification_hold')
             OR (status = 'reserved' AND (expires_at IS NULL OR expires_at >= ?3))
           )",
        params![user_id, compute_call_id, active_at],
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

fn active_hold_with_status(
    tx: &Transaction<'_>,
    user_id: &str,
    compute_call_id: &str,
    status: &str,
) -> Result<Option<ActiveBillingReservation>> {
    tx.query_row(
        "SELECT id, user_id, compute_call_id, reserved_fen, expires_at
           FROM billing_reservations
          WHERE user_id = ?1 AND compute_call_id = ?2 AND status = ?3",
        params![user_id, compute_call_id, status],
        |row| {
            Ok(ActiveBillingReservation {
                reservation_id: row.get(0)?,
                user_id: row.get(1)?,
                compute_call_id: row.get(2)?,
                reserved_fen: row.get(3)?,
                expires_at: row.get(4)?,
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

fn normalize_required_id(field: &str, value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 200 || value.chars().any(char::is_control) {
        return Err(anyhow!("{field} 无效"));
    }
    Ok(value.to_string())
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
