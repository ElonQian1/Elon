use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Transaction};

#[derive(Debug, Clone)]
pub(super) struct ComputeBillingSettlementOutcome {
    pub reservation_id: String,
    pub reserved_fen: i64,
    pub charged_fen: i64,
    pub refunded_fen: i64,
    pub consumer_balance_after_fen: i64,
}

pub(super) fn settle_compute_billing_reservation_on(
    tx: &Transaction<'_>,
    reservation_id: &str,
    consumer_user_id: &str,
    expected_reserved_fen: i64,
    charged_fen: i64,
    settled_at: &str,
) -> Result<ComputeBillingSettlementOutcome> {
    if charged_fen < 0 || charged_fen > expected_reserved_fen {
        bail!("通用算力消费者结算金额超出预授权");
    }
    let stored = tx
        .query_row(
            "SELECT id, reserved_fen, settled_cost_fen, refunded_fen, status
               FROM billing_reservations
              WHERE id=?1 AND user_id=?2",
            params![reservation_id.trim(), consumer_user_id.trim()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("通用算力消费者预授权不存在"))?;
    if stored.1 != expected_reserved_fen
        || stored.2 != 0
        || stored.3 != 0
        || !matches!(
            stored.4.as_str(),
            "reserved" | "dispatch_hold" | "verification_hold"
        )
    {
        bail!("通用算力消费者预授权不再处于可结算状态");
    }

    let refunded_fen = expected_reserved_fen
        .checked_sub(charged_fen)
        .ok_or_else(|| anyhow!("通用算力消费者退款金额下溢"))?;
    let current_balance = tx
        .query_row(
            "SELECT balance_fen FROM user_balance WHERE user_id=?1",
            params![consumer_user_id.trim()],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("通用算力消费者余额账户不存在"))?;
    let balance_after = current_balance
        .checked_add(refunded_fen)
        .ok_or_else(|| anyhow!("通用算力消费者退款后余额溢出"))?;
    let changed = tx.execute(
        "UPDATE user_balance
            SET balance_fen=?1, updated_at=?2
          WHERE user_id=?3",
        params![balance_after, settled_at, consumer_user_id.trim()],
    )?;
    if changed != 1 {
        bail!("通用算力消费者余额账户不存在");
    }
    let changed = tx.execute(
        "UPDATE billing_reservations
            SET status='settled', settled_cost_fen=?2, refunded_fen=?3,
                token_usage_event_id=NULL, billing_event_id=NULL,
                balance_after_fen=?4, updated_at=?5
          WHERE id=?1 AND status IN ('reserved','dispatch_hold','verification_hold')",
        params![
            stored.0,
            charged_fen,
            refunded_fen,
            balance_after,
            settled_at
        ],
    )?;
    if changed != 1 {
        bail!("通用算力消费者预授权并发结算失败");
    }
    Ok(ComputeBillingSettlementOutcome {
        reservation_id: stored.0,
        reserved_fen: expected_reserved_fen,
        charged_fen,
        refunded_fen,
        consumer_balance_after_fen: balance_after,
    })
}
