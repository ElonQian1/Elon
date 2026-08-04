use anyhow::{anyhow, bail, Context, Result};
use chrono::Duration;
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    super::{
        super::{
            compute_attempt_settlement_challenges::{
                settlement_challenge_gate_on, COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
            },
            compute_attempt_settlements::compute_attempt_settlement_on,
        },
        ComputeSettlementReleaseReceipt, COMPUTE_SETTLEMENT_RELEASE_POLICY_ID,
        COMPUTE_SETTLEMENT_RELEASE_POLICY_VERSION, COMPUTE_SETTLEMENT_RELEASE_SCHEMA,
    },
    challenge_gate_digest, money, normalize_release_request, parse_time, release_event_digest,
    release_request_digest, StoredRelease,
};

pub(super) fn audited_release_on(
    conn: &Connection,
    stored: &StoredRelease,
    replayed: bool,
) -> Result<ComputeSettlementReleaseReceipt> {
    let request = normalize_release_request(&serde_json::from_str(&stored.request_json)?)?;
    let stored_gate = serde_json::from_str(&stored.challenge_gate_json)?;
    let mut receipt: ComputeSettlementReleaseReceipt = serde_json::from_str(&stored.receipt_json)?;
    if request.lease_id != stored.lease_id
        || request.expected_settlement_receipt_id != stored.settlement_receipt_id
        || request.expected_settlement_event_digest != stored.settlement_event_digest
        || request.expected_posting_id != stored.source_posting_id
        || request.expected_posting_digest != stored.source_posting_digest
        || request.idempotency_key != stored.idempotency_key
        || request.released_by_user_id != stored.released_by_user_id
        || stored.idempotency_scope
            != format!("compute_settlement_release:{}", request.released_by_user_id)
        || release_request_digest(&request)? != stored.request_digest
        || receipt.schema != COMPUTE_SETTLEMENT_RELEASE_SCHEMA
        || receipt.release_id != stored.release_id
        || receipt.settlement_receipt_id != stored.settlement_receipt_id
        || receipt.settlement_event_digest != stored.settlement_event_digest
        || receipt.source_posting_id != stored.source_posting_id
        || receipt.source_posting_digest != stored.source_posting_digest
        || receipt.lease_id != stored.lease_id
        || receipt.consumer_account_id != stored.consumer_account_id
        || receipt.provider_account_id != stored.provider_account_id
        || receipt.provider_released_micros != stored.provider_released_micros
        || receipt.platform_released_micros != stored.platform_released_micros
        || receipt.challenge_deadline != stored.challenge_deadline
        || receipt.challenge_gate != stored_gate
        || receipt.challenge_gate_digest != stored.challenge_gate_digest
        || receipt.policy_id != stored.policy_id
        || receipt.policy_version != stored.policy_version
        || receipt.release_posting_id != stored.release_posting_id
        || receipt.release_posting_digest != stored.release_posting_digest
        || receipt.request_digest != stored.request_digest
        || receipt.event_digest != stored.event_digest
        || receipt.released_by_user_id != stored.released_by_user_id
        || receipt.released_at != stored.released_at
        || receipt.replayed
    {
        bail!("待结算释放数据库列、请求或回执 JSON 不一致");
    }
    if challenge_gate_digest(&receipt.challenge_gate)? != stored.challenge_gate_digest
        || release_event_digest(&receipt)? != stored.event_digest
    {
        bail!("待结算释放挑战门卫或事件摘要审计失败");
    }

    let settlement = compute_attempt_settlement_on(conn, &stored.lease_id)?;
    if settlement.settlement.settlement_receipt_id != stored.settlement_receipt_id
        || settlement.event_digest != stored.settlement_event_digest
        || settlement.posting_id != stored.source_posting_id
        || settlement.posting_digest != stored.source_posting_digest
        || settlement.settlement.consumer_account_id != stored.consumer_account_id
        || settlement.settlement.provider_account_id != stored.provider_account_id
        || settlement.settlement.amounts.provider_payable_micros != stored.provider_released_micros
        || settlement.settlement.amounts.platform_margin_micros != stored.platform_released_micros
        || settlement.settlement.balance_state != "pending"
    {
        bail!("待结算释放上游 Settlement Receipt 审计失败");
    }
    let expected_deadline = parse_time("Settlement 结算时间", &settlement.settled_at)?
        .checked_add_signed(Duration::seconds(
            COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
        ))
        .context("待结算释放挑战截止时间超出范围")?;
    let released_at = parse_time("Settlement 释放时间", &stored.released_at)?;
    if stored.challenge_deadline != expected_deadline.to_rfc3339()
        || released_at < expected_deadline
        || receipt.policy_id != COMPUTE_SETTLEMENT_RELEASE_POLICY_ID
        || receipt.policy_version != COMPUTE_SETTLEMENT_RELEASE_POLICY_VERSION
    {
        bail!("待结算释放窗口或策略版本审计失败");
    }
    let current_gate = settlement_challenge_gate_on(conn, &stored.settlement_receipt_id)?;
    if current_gate != receipt.challenge_gate
        || current_gate.blocked
        || current_gate.correction_required
    {
        bail!("待结算释放后的消费者挑战门卫状态不一致");
    }
    audit_posting(conn, &receipt)?;
    audit_account_projection(
        conn,
        "provider",
        &receipt.provider_account_id,
        "provider_pending",
    )?;
    audit_account_projection(
        conn,
        "platform",
        money::PLATFORM_ACCOUNT_ID,
        "platform_pending",
    )?;
    if receipt.platform_account_id != money::PLATFORM_ACCOUNT_ID
        || receipt.currency != "CNY"
        || receipt.balance_effect != "provider_and_platform_pending_moved_to_available"
        || receipt.withdrawal_effect != "no_external_transfer_or_withdrawal"
    {
        bail!("待结算释放资金效果字段无效");
    }
    receipt.replayed = replayed;
    Ok(receipt)
}

fn audit_posting(conn: &Connection, receipt: &ComputeSettlementReleaseReceipt) -> Result<()> {
    let row = money::release_posting_row_on(conn, &receipt.release_posting_id)?
        .ok_or_else(|| anyhow!("待结算释放 posting 不存在"))?;
    let input = money::PostReleaseMoneyInput {
        release_id: &receipt.release_id,
        settlement_receipt_id: &receipt.settlement_receipt_id,
        provider_account_id: &receipt.provider_account_id,
        provider_released_micros: receipt.provider_released_micros,
        platform_released_micros: receipt.platform_released_micros,
        released_at: &receipt.released_at,
    };
    let provider = money::AccountReleaseOutcome {
        pending_after_micros: receipt.provider_pending_balance_after_micros,
        available_after_micros: receipt.provider_available_balance_after_micros,
        revision_after: receipt.provider_account_revision_after,
    };
    let platform = money::AccountReleaseOutcome {
        pending_after_micros: receipt.platform_pending_balance_after_micros,
        available_after_micros: receipt.platform_available_balance_after_micros,
        revision_after: receipt.platform_account_revision_after,
    };
    let expected_digest =
        money::release_posting_digest(&receipt.release_posting_id, &input, &provider, &platform)?;
    if row.0 != receipt.release_id
        || row.1 != receipt.settlement_receipt_id
        || row.2 != receipt.provider_released_micros
        || row.3 != receipt.platform_released_micros
        || row.4 != receipt.release_posting_digest
        || row.4 != expected_digest
        || row.5 != receipt.released_at
    {
        bail!("待结算释放 posting 摘要或金额审计失败");
    }
    audit_ledger_legs(conn, receipt)
}

fn audit_ledger_legs(conn: &Connection, receipt: &ComputeSettlementReleaseReceipt) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT line_no, account_kind, leg_kind, account_id, direction,
                amount_micros, balance_state, balance_after_micros,
                account_revision_after
           FROM compute_settlement_release_ledger_legs
          WHERE posting_id=?1 ORDER BY line_no",
    )?;
    let legs = stmt
        .query_map(params![receipt.release_posting_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let expected = vec![
        (
            1,
            "provider".to_string(),
            "provider_pending_release".to_string(),
            receipt.provider_account_id.clone(),
            "debit".to_string(),
            receipt.provider_released_micros,
            "pending".to_string(),
            receipt.provider_pending_balance_after_micros,
            receipt.provider_account_revision_after,
        ),
        (
            2,
            "provider".to_string(),
            "provider_available_credit".to_string(),
            receipt.provider_account_id.clone(),
            "credit".to_string(),
            receipt.provider_released_micros,
            "available".to_string(),
            receipt.provider_available_balance_after_micros,
            receipt.provider_account_revision_after,
        ),
        (
            3,
            "platform".to_string(),
            "platform_pending_release".to_string(),
            money::PLATFORM_ACCOUNT_ID.to_string(),
            "debit".to_string(),
            receipt.platform_released_micros,
            "pending".to_string(),
            receipt.platform_pending_balance_after_micros,
            receipt.platform_account_revision_after,
        ),
        (
            4,
            "platform".to_string(),
            "platform_available_credit".to_string(),
            money::PLATFORM_ACCOUNT_ID.to_string(),
            "credit".to_string(),
            receipt.platform_released_micros,
            "available".to_string(),
            receipt.platform_available_balance_after_micros,
            receipt.platform_account_revision_after,
        ),
    ];
    if legs != expected {
        bail!("待结算释放账本腿或历史余额快照审计失败");
    }
    Ok(())
}

fn audit_account_projection(
    conn: &Connection,
    account_kind: &str,
    account_id: &str,
    settlement_leg_kind: &str,
) -> Result<()> {
    let current = conn
        .query_row(
            "SELECT pending_micros, available_micros
               FROM compute_settlement_account_balances
              WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'",
            params![account_kind, account_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("待结算释放账户投影不存在"))?;
    let credited_pending = conn.query_row(
        "SELECT COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_ledger_legs
          WHERE account_id=?1 AND currency='CNY' AND balance_state='pending'
            AND direction='credit' AND leg_kind=?2",
        params![account_id, settlement_leg_kind],
        |row| row.get::<_, i64>(0),
    )?;
    let released_pending = conn.query_row(
        "SELECT COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_release_ledger_legs
          WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'
            AND balance_state='pending' AND direction='debit'",
        params![account_kind, account_id],
        |row| row.get::<_, i64>(0),
    )?;
    let credited_available = conn.query_row(
        "SELECT COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_release_ledger_legs
          WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'
            AND balance_state='available' AND direction='credit'",
        params![account_kind, account_id],
        |row| row.get::<_, i64>(0),
    )?;
    if current.0 != credited_pending - released_pending
        || current.1 != credited_available
        || current.0 < 0
        || current.1 < 0
    {
        bail!("待结算释放账户余额与不可变账本投影不一致");
    }
    Ok(())
}
