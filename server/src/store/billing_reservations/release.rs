use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    normalize_compute_call_id, normalize_release_status, normalize_required_id,
    refund_reserved_fen, BillingReservationOutcome,
};
use crate::store::now;

pub(super) fn release_billing_call_compat_on(
    tx: &Transaction<'_>,
    user_id: &str,
    compute_call_id: &str,
    status: &str,
) -> Result<Option<BillingReservationOutcome>> {
    release_billing_call_on(tx, user_id, compute_call_id, None, status, false)
}

pub(super) fn release_billing_call_reservation_on(
    tx: &Transaction<'_>,
    user_id: &str,
    compute_call_id: &str,
    expected_reservation_id: &str,
    status: &str,
) -> Result<BillingReservationOutcome> {
    let expected_reservation_id =
        normalize_required_id("billing reservation id", expected_reservation_id)?;
    release_billing_call_on(
        tx,
        user_id,
        compute_call_id,
        Some(&expected_reservation_id),
        status,
        true,
    )?
    .ok_or_else(|| anyhow!("Broker 绑定的余额预授权不存在"))
}

fn release_billing_call_on(
    tx: &Transaction<'_>,
    user_id: &str,
    compute_call_id: &str,
    expected_reservation_id: Option<&str>,
    status: &str,
    strict_replay: bool,
) -> Result<Option<BillingReservationOutcome>> {
    let user_id = normalize_required_id("user_id", user_id)?;
    let compute_call_id = normalize_compute_call_id(compute_call_id)?;
    let release_status = normalize_release_status(status);
    if strict_replay && release_status != status {
        bail!("Broker 余额退款状态不受支持");
    }
    let existing = tx
        .query_row(
            "SELECT id, reserved_fen, refunded_fen, status, balance_after_fen
               FROM billing_reservations
              WHERE user_id=?1 AND compute_call_id=?2",
            params![user_id, compute_call_id],
            |row| {
                Ok(ExistingBillingRelease {
                    reservation_id: row.get(0)?,
                    reserved_fen: row.get(1)?,
                    refunded_fen: row.get(2)?,
                    status: row.get(3)?,
                    balance_after_fen: row.get(4)?,
                })
            },
        )
        .optional()?;
    let Some(existing) = existing else {
        return Ok(None);
    };
    if expected_reservation_id.is_some_and(|value| value != existing.reservation_id.as_str()) {
        bail!("Broker 余额预授权 ID 与原子预留回执不一致");
    }
    if existing.status != "reserved" {
        if strict_replay
            && existing.status == release_status
            && existing.refunded_fen == existing.reserved_fen
        {
            return Ok(Some(outcome(
                existing,
                compute_call_id,
                true,
                release_status,
            )));
        }
        if strict_replay {
            bail!("Broker 余额预授权已进入不兼容终态");
        }
        return Ok(None);
    }

    let ts = now();
    let balance_after = refund_reserved_fen(tx, &user_id, existing.reserved_fen, &ts)?;
    let changed = tx.execute(
        "UPDATE billing_reservations
            SET status=?2, refunded_fen=reserved_fen,
                balance_after_fen=?3, updated_at=?4
          WHERE id=?1 AND status='reserved'",
        params![existing.reservation_id, release_status, balance_after, ts],
    )?;
    if changed != 1 {
        bail!("Broker 余额预授权在退款事务中发生并发变化");
    }
    Ok(Some(BillingReservationOutcome {
        reservation_id: existing.reservation_id,
        compute_call_id,
        reserved_fen: existing.reserved_fen,
        balance_after_fen: Some(balance_after),
        status: release_status.to_string(),
        deduplicated: false,
    }))
}

struct ExistingBillingRelease {
    reservation_id: String,
    reserved_fen: i64,
    refunded_fen: i64,
    status: String,
    balance_after_fen: Option<i64>,
}

fn outcome(
    existing: ExistingBillingRelease,
    compute_call_id: String,
    deduplicated: bool,
    release_status: &str,
) -> BillingReservationOutcome {
    BillingReservationOutcome {
        reservation_id: existing.reservation_id,
        compute_call_id,
        reserved_fen: existing.reserved_fen,
        balance_after_fen: existing.balance_after_fen,
        status: release_status.to_string(),
        deduplicated,
    }
}
