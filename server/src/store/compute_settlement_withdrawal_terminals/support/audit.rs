use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    super::{
        super::compute_settlement_withdrawal_requests::compute_settlement_withdrawal_request_on,
        ComputeSettlementWithdrawalTerminalReceipt, COMPUTE_SETTLEMENT_WITHDRAWAL_TERMINAL_SCHEMA,
    },
    event_digest, money, normalize_request, terminal_digest, StoredWithdrawalTerminal,
};

pub(super) fn audited_terminal_on(
    conn: &Connection,
    stored: &StoredWithdrawalTerminal,
    replayed: bool,
) -> Result<ComputeSettlementWithdrawalTerminalReceipt> {
    let request = normalize_request(&serde_json::from_str(&stored.request_json)?)?;
    let mut receipt: ComputeSettlementWithdrawalTerminalReceipt =
        serde_json::from_str(&stored.receipt_json)?;
    audit_stored_contract(stored, &request, &receipt)?;
    if terminal_digest(&request)? != stored.request_digest
        || event_digest(&receipt)? != stored.event_digest
    {
        bail!("提现终态请求或事件摘要审计失败");
    }

    let withdrawal = compute_settlement_withdrawal_request_on(conn, &stored.withdrawal_id)?;
    if withdrawal.event_digest != stored.withdrawal_event_digest
        || withdrawal.request_posting_id != stored.request_posting_id
        || withdrawal.request_posting_digest != stored.request_posting_digest
        || withdrawal.provider_id != stored.provider_id
        || withdrawal.provider_account_id != stored.provider_account_id
        || withdrawal.owner_user_id != stored.owner_user_id
        || withdrawal.amount_micros != stored.amount_micros
    {
        bail!("提现终态上游 Withdrawal Request Receipt 审计失败");
    }
    audit_action_semantics(&request, &receipt)?;
    audit_posting(conn, &receipt)?;
    audit_account_projection(conn, &stored.provider_account_id)?;
    receipt.replayed = replayed;
    Ok(receipt)
}

fn audit_stored_contract(
    stored: &StoredWithdrawalTerminal,
    request: &super::super::TerminalizeComputeSettlementWithdrawalRequest,
    receipt: &ComputeSettlementWithdrawalTerminalReceipt,
) -> Result<()> {
    if request.withdrawal_id != stored.withdrawal_id
        || request.expected_withdrawal_event_digest != stored.withdrawal_event_digest
        || request.expected_request_posting_id != stored.request_posting_id
        || request.expected_request_posting_digest != stored.request_posting_digest
        || request.action != stored.action
        || request.reason_code != stored.reason_code
        || request.reason_detail != stored.reason_detail
        || request.external_evidence_kind != stored.external_evidence_kind
        || request.external_evidence_ref != stored.external_evidence_ref
        || request.external_evidence_digest != stored.external_evidence_digest
        || request.actor_user_id != stored.actor_user_id
        || request.actor_role != stored.actor_role
        || request.idempotency_key != stored.idempotency_key
        || stored.idempotency_scope
            != format!(
                "compute_settlement_withdrawal_terminal:{}:{}",
                stored.actor_role, stored.actor_user_id
            )
        || receipt.schema != COMPUTE_SETTLEMENT_WITHDRAWAL_TERMINAL_SCHEMA
        || receipt.terminal_id != stored.terminal_id
        || receipt.withdrawal_id != stored.withdrawal_id
        || receipt.withdrawal_event_digest != stored.withdrawal_event_digest
        || receipt.request_posting_id != stored.request_posting_id
        || receipt.request_posting_digest != stored.request_posting_digest
        || receipt.provider_id != stored.provider_id
        || receipt.provider_account_id != stored.provider_account_id
        || receipt.owner_user_id != stored.owner_user_id
        || receipt.amount_micros != stored.amount_micros
        || receipt.action != stored.action
        || receipt.reason_code != stored.reason_code
        || receipt.reason_detail != stored.reason_detail
        || receipt.external_evidence_kind != stored.external_evidence_kind
        || receipt.external_evidence_ref != stored.external_evidence_ref
        || receipt.external_evidence_digest != stored.external_evidence_digest
        || receipt.balance_returned_micros != stored.balance_returned_micros
        || receipt.terminal_posting_id != stored.terminal_posting_id
        || receipt.terminal_posting_digest != stored.terminal_posting_digest
        || receipt.request_digest != stored.request_digest
        || receipt.event_digest != stored.event_digest
        || receipt.actor_user_id != stored.actor_user_id
        || receipt.actor_role != stored.actor_role
        || receipt.terminal_at != stored.terminal_at
        || receipt.currency != "CNY"
        || receipt.replayed
    {
        bail!("提现终态数据库列、请求或回执 JSON 不一致");
    }
    Ok(())
}

fn audit_action_semantics(
    request: &super::super::TerminalizeComputeSettlementWithdrawalRequest,
    receipt: &ComputeSettlementWithdrawalTerminalReceipt,
) -> Result<()> {
    let returns_balance = matches!(receipt.action.as_str(), "cancelled" | "rejected");
    if returns_balance {
        if receipt.balance_returned_micros != receipt.amount_micros
            || receipt.fund_effect != "provider_withdrawn_returned_to_available"
            || receipt.external_transfer_effect != "not_executed"
        {
            bail!("提现取消或拒绝的退款语义审计失败");
        }
    } else if receipt.action == "external_paid_attested" {
        if receipt.balance_returned_micros != 0
            || receipt.fund_effect != "provider_withdrawn_balance_retained"
            || receipt.external_transfer_effect
                != "external_payment_attested_not_executed_or_verified"
            || !request.confirm_external_payment_already_completed
        {
            bail!("外部已付款声明语义审计失败");
        }
    } else {
        bail!("提现终态动作审计失败");
    }
    Ok(())
}

fn audit_posting(
    conn: &Connection,
    receipt: &ComputeSettlementWithdrawalTerminalReceipt,
) -> Result<()> {
    let row = money::posting_row_on(conn, &receipt.terminal_posting_id)?
        .ok_or_else(|| anyhow!("提现终态 Posting 不存在"))?;
    let input = money::PostTerminalInput {
        terminal_id: &receipt.terminal_id,
        withdrawal_id: &receipt.withdrawal_id,
        provider_account_id: &receipt.provider_account_id,
        action: &receipt.action,
        amount_micros: receipt.amount_micros,
        balance_returned_micros: receipt.balance_returned_micros,
        terminal_at: &receipt.terminal_at,
    };
    let digest = money::posting_digest(
        &receipt.terminal_posting_id,
        &input,
        receipt.available_balance_after_micros,
        receipt.withdrawn_balance_after_micros,
        receipt.account_revision_after,
    )?;
    if row.0 != receipt.terminal_id
        || row.1 != receipt.withdrawal_id
        || row.2 != receipt.action
        || row.3 != receipt.amount_micros
        || row.4 != receipt.balance_returned_micros
        || row.5 != receipt.terminal_posting_digest
        || row.5 != digest
        || row.6 != receipt.terminal_at
    {
        bail!("提现终态 Posting 摘要或金额审计失败");
    }
    audit_legs(conn, receipt)
}

fn audit_legs(
    conn: &Connection,
    receipt: &ComputeSettlementWithdrawalTerminalReceipt,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT line_no, leg_kind, account_id, direction, amount_micros,
                balance_state, balance_after_micros, account_revision_after
           FROM compute_settlement_withdrawal_terminal_ledger_legs
          WHERE posting_id=?1 ORDER BY line_no",
    )?;
    let legs = stmt
        .query_map(params![receipt.terminal_posting_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let expected = if receipt.balance_returned_micros > 0 {
        vec![
            (
                1,
                "provider_withdrawn_terminal_release".to_string(),
                receipt.provider_account_id.clone(),
                "debit".to_string(),
                receipt.balance_returned_micros,
                "withdrawn".to_string(),
                receipt.withdrawn_balance_after_micros,
                receipt.account_revision_after,
            ),
            (
                2,
                "provider_available_terminal_return".to_string(),
                receipt.provider_account_id.clone(),
                "credit".to_string(),
                receipt.balance_returned_micros,
                "available".to_string(),
                receipt.available_balance_after_micros,
                receipt.account_revision_after,
            ),
        ]
    } else {
        Vec::new()
    };
    if legs != expected {
        bail!("提现终态账本腿或历史余额快照审计失败");
    }
    Ok(())
}

fn audit_account_projection(conn: &Connection, account_id: &str) -> Result<()> {
    let current = conn
        .query_row(
            "SELECT available_micros, withdrawn_micros
               FROM compute_settlement_account_balances
              WHERE account_kind='provider' AND account_id=?1 AND currency='CNY'",
            params![account_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("提现终态 Provider 结算账户不存在"))?;
    let released = conn.query_row(
        "SELECT COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_release_ledger_legs
          WHERE account_kind='provider' AND account_id=?1 AND currency='CNY'
            AND balance_state='available' AND direction='credit'",
        params![account_id],
        |row| row.get::<_, i64>(0),
    )?;
    let reserved = conn.query_row(
        "SELECT COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_withdrawal_request_ledger_legs
          WHERE account_kind='provider' AND account_id=?1 AND currency='CNY'
            AND balance_state='available' AND direction='debit'",
        params![account_id],
        |row| row.get::<_, i64>(0),
    )?;
    let returned = conn.query_row(
        "SELECT COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_withdrawal_terminal_ledger_legs
          WHERE account_kind='provider' AND account_id=?1 AND currency='CNY'
            AND balance_state='available' AND direction='credit'",
        params![account_id],
        |row| row.get::<_, i64>(0),
    )?;
    if current.0 != released - reserved + returned
        || current.1 != reserved - returned
        || current.0 < 0
        || current.1 < 0
    {
        bail!("提现终态账户余额与不可变账本投影不一致");
    }
    Ok(())
}
