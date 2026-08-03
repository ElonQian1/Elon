use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    normalize_compute_call_id, normalize_required_id, BillingReservationOutcome,
    BillingReservationRequest,
};
use crate::store::{new_id, now};

pub(super) fn reserve_billing_call_compat_on(
    conn: &Connection,
    request: &BillingReservationRequest<'_>,
    expires_at: &str,
) -> Result<BillingReservationOutcome> {
    reserve_billing_call_on(conn, request, expires_at, false)
}

pub(super) fn reserve_billing_call_until_on(
    conn: &Connection,
    request: &BillingReservationRequest<'_>,
    expires_at: &str,
) -> Result<BillingReservationOutcome> {
    reserve_billing_call_on(conn, request, expires_at, true)
}

fn reserve_billing_call_on(
    conn: &Connection,
    request: &BillingReservationRequest<'_>,
    expires_at: &str,
    strict_replay: bool,
) -> Result<BillingReservationOutcome> {
    let user_id = normalize_required_id("user_id", request.user_id)?;
    let compute_call_id = normalize_compute_call_id(request.compute_call_id)?;
    let reserve_fen = request.reserve_fen.max(0);
    let ts = now();
    let expires_at = normalize_future_expiry(expires_at, &ts)?;

    if let Some(existing) = existing_reservation_on(conn, &user_id, &compute_call_id)? {
        if strict_replay {
            ensure_strict_replay_matches(request, reserve_fen, &expires_at, &existing)?;
        }
        return Ok(BillingReservationOutcome {
            reservation_id: existing.reservation_id,
            compute_call_id,
            reserved_fen: existing.reserved_fen,
            balance_after_fen: existing.balance_after_fen,
            status: existing.status,
            deduplicated: true,
        });
    }

    let mut balance = conn
        .query_row(
            "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
            params![user_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if balance.is_none() && request.bill_missing_balance {
        conn.execute(
            "INSERT INTO user_balance (user_id, balance_fen, updated_at) VALUES (?1, 0, ?2)",
            params![user_id, ts],
        )?;
        balance = Some(0);
    }

    let (status, balance_after) = if let Some(balance) = balance {
        if balance < reserve_fen {
            bail!(
                "余额不足（当前 {} 分，需要至少 {} 分预授权），请联系管理员充值后继续使用",
                balance,
                reserve_fen
            );
        }
        let balance_after = balance - reserve_fen;
        conn.execute(
            "UPDATE user_balance SET balance_fen = ?1, updated_at = ?2 WHERE user_id = ?3",
            params![balance_after, ts, user_id],
        )?;
        ("reserved", Some(balance_after))
    } else {
        ("skipped_no_balance", None)
    };

    let reservation_id = new_id("brv");
    conn.execute(
        "INSERT INTO billing_reservations (
           id, user_id, compute_call_id, feature, usage_mode, model,
           reserved_fen, settled_cost_fen, refunded_fen, status,
           created_at, updated_at, expires_at, balance_after_fen
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,0,0,?8,?9,?9,?10,?11)",
        params![
            reservation_id,
            user_id,
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
    Ok(BillingReservationOutcome {
        reservation_id,
        compute_call_id,
        reserved_fen: reserve_fen,
        balance_after_fen: balance_after,
        status: status.to_string(),
        deduplicated: false,
    })
}

struct ExistingBillingReservation {
    reservation_id: String,
    feature: String,
    usage_mode: String,
    model: Option<String>,
    reserved_fen: i64,
    balance_after_fen: Option<i64>,
    status: String,
    expires_at: Option<String>,
}

fn existing_reservation_on(
    conn: &Connection,
    user_id: &str,
    compute_call_id: &str,
) -> Result<Option<ExistingBillingReservation>> {
    conn.query_row(
        "SELECT id, feature, usage_mode, model, reserved_fen,
                balance_after_fen, status, expires_at
           FROM billing_reservations
          WHERE user_id=?1 AND compute_call_id=?2",
        params![user_id, compute_call_id],
        |row| {
            Ok(ExistingBillingReservation {
                reservation_id: row.get(0)?,
                feature: row.get(1)?,
                usage_mode: row.get(2)?,
                model: row.get(3)?,
                reserved_fen: row.get(4)?,
                balance_after_fen: row.get(5)?,
                status: row.get(6)?,
                expires_at: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn ensure_strict_replay_matches(
    request: &BillingReservationRequest<'_>,
    reserve_fen: i64,
    expires_at: &str,
    existing: &ExistingBillingReservation,
) -> Result<()> {
    if existing.feature != request.feature
        || existing.usage_mode != request.usage_mode
        || existing.model.as_deref() != request.model
        || existing.reserved_fen != reserve_fen
        || existing.expires_at.as_deref() != Some(expires_at)
        || existing.status != "reserved"
    {
        bail!("相同算力预算预授权键不能重放为不同合同或非 reserved 状态");
    }
    Ok(())
}

fn normalize_future_expiry(expires_at: &str, now_value: &str) -> Result<String> {
    let expires = DateTime::parse_from_rfc3339(expires_at.trim())
        .context("算力预算预授权到期时间不是 RFC3339")?;
    let recorded =
        DateTime::parse_from_rfc3339(now_value).context("算力预算预授权记录时间不是 RFC3339")?;
    if expires.offset().local_minus_utc() != 0 || expires <= recorded {
        bail!("算力预算预授权到期时间必须使用 UTC 且晚于当前记录时间");
    }
    Ok(expires.with_timezone(&Utc).to_rfc3339())
}
