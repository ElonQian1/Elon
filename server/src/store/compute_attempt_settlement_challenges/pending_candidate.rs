use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};

use super::{
    super::compute_attempt_settlements::compute_attempt_settlement_on,
    settlement_challenge_gate_on, ComputePendingSettlementChallengeCandidate,
    COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
};

pub(super) fn build_pending_challenge_candidate_on(
    conn: &Connection,
    lease_id: &str,
    consumer_user_id: &str,
    as_of: &DateTime<Utc>,
) -> Result<ComputePendingSettlementChallengeCandidate> {
    let settlement = compute_attempt_settlement_on(conn, lease_id)?;
    if settlement.settlement.consumer_account_id != consumer_user_id
        || settlement.settlement.balance_state != "pending"
    {
        bail!("待申诉 Settlement Receipt 的消费者身份或余额状态不匹配");
    }

    let gate = settlement_challenge_gate_on(conn, &settlement.settlement.settlement_receipt_id)?;
    if gate.status != "none" || gate.blocked || gate.correction_required {
        bail!("待申诉候选已经存在挑战或纠正门卫");
    }
    let already_released = conn.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM compute_settlement_releases
            WHERE settlement_receipt_id=?1
         )",
        params![settlement.settlement.settlement_receipt_id],
        |row| row.get::<_, bool>(0),
    )?;
    if already_released {
        bail!("待申诉 Settlement Receipt 已释放到 available");
    }

    let settled_at = DateTime::parse_from_rfc3339(&settlement.settled_at)
        .context("Settlement 结算时间不是 RFC3339")?
        .with_timezone(&Utc);
    let challenge_deadline = settled_at
        .checked_add_signed(Duration::seconds(
            COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
        ))
        .context("结算挑战候选截止时间超出范围")?;
    if settled_at > *as_of || *as_of > challenge_deadline {
        bail!("Settlement Receipt 不在消费者挑战窗口内");
    }

    Ok(ComputePendingSettlementChallengeCandidate {
        settlement,
        challenge_deadline: challenge_deadline.to_rfc3339(),
        balance_effect: "provider_and_platform_pending_unchanged",
        settlement_release_effect: "blocked_by_open_challenge",
        external_payment_effect: "none",
    })
}
