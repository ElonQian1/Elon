use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    super::{
        super::compute_provider_registry::registered_provider_version_on,
        ComputeSettlementWithdrawalRequestReceipt, COMPUTE_SETTLEMENT_WITHDRAWAL_REQUEST_SCHEMA,
    },
    event_digest, money, normalize_request, request_digest, StoredWithdrawalRequest,
};

pub(super) fn audited_request_on(
    conn: &Connection,
    stored: &StoredWithdrawalRequest,
    replayed: bool,
) -> Result<ComputeSettlementWithdrawalRequestReceipt> {
    let request = normalize_request(&serde_json::from_str(&stored.request_json)?)?;
    let mut receipt: ComputeSettlementWithdrawalRequestReceipt =
        serde_json::from_str(&stored.receipt_json)?;
    if request.provider_id != stored.provider_id
        || request.expected_provider_policy_revision != stored.provider_policy_revision
        || request.expected_provider_digest != stored.provider_digest
        || request.provider_account_id != stored.provider_account_id
        || request.owner_user_id != stored.owner_user_id
        || request.amount_micros != stored.amount_micros
        || request.destination_kind != stored.destination_kind
        || request.destination_ref != stored.destination_ref
        || request.idempotency_key != stored.idempotency_key
        || stored.idempotency_scope
            != format!(
                "compute_settlement_withdrawal_request:{}:{}",
                stored.owner_user_id, stored.provider_id
            )
        || request_digest(&request)? != stored.request_digest
        || receipt.schema != COMPUTE_SETTLEMENT_WITHDRAWAL_REQUEST_SCHEMA
        || receipt.withdrawal_id != stored.withdrawal_id
        || receipt.provider_id != stored.provider_id
        || receipt.provider_policy_revision != stored.provider_policy_revision
        || receipt.provider_digest != stored.provider_digest
        || receipt.provider_account_id != stored.provider_account_id
        || receipt.owner_user_id != stored.owner_user_id
        || receipt.amount_micros != stored.amount_micros
        || receipt.destination_kind != stored.destination_kind
        || receipt.destination_ref != stored.destination_ref
        || receipt.request_posting_id != stored.request_posting_id
        || receipt.request_posting_digest != stored.request_posting_digest
        || receipt.request_digest != stored.request_digest
        || receipt.event_digest != stored.event_digest
        || receipt.requested_at != stored.requested_at
        || receipt.replayed
    {
        bail!("提现申请数据库列、请求或回执 JSON 不一致");
    }
    if event_digest(&receipt)? != stored.event_digest
        || receipt.currency != "CNY"
        || receipt.fund_effect != "provider_available_moved_to_withdrawn_reserve"
        || receipt.external_transfer_effect != "not_executed"
    {
        bail!("提现申请摘要或资金效果字段审计失败");
    }

    let provider =
        registered_provider_version_on(conn, &stored.provider_id, stored.provider_policy_revision)?
            .ok_or_else(|| anyhow!("提现申请引用的 Provider 版本不存在"))?;
    let account_id = provider
        .provider
        .settlement_account_id
        .as_deref()
        .unwrap_or(provider.provider.owner_account_id.as_str());
    if provider.provider_digest != stored.provider_digest
        || provider.provider.owner_account_id != stored.owner_user_id
        || account_id != stored.provider_account_id
    {
        bail!("提现申请引用的 Provider 所有权或结算账户审计失败");
    }

    audit_posting(conn, &receipt)?;
    audit_account_projection(conn, &stored.provider_account_id)?;
    receipt.replayed = replayed;
    Ok(receipt)
}

fn audit_posting(
    conn: &Connection,
    receipt: &ComputeSettlementWithdrawalRequestReceipt,
) -> Result<()> {
    let row = money::posting_row_on(conn, &receipt.request_posting_id)?
        .ok_or_else(|| anyhow!("提现申请 Posting 不存在"))?;
    let digest = money::posting_digest(
        &receipt.request_posting_id,
        &receipt.withdrawal_id,
        &receipt.provider_account_id,
        receipt.amount_micros,
        receipt.available_balance_after_micros,
        receipt.withdrawn_balance_after_micros,
        receipt.account_revision_after,
        &receipt.requested_at,
    )?;
    if row.0 != receipt.withdrawal_id
        || row.1 != receipt.provider_account_id
        || row.2 != receipt.amount_micros
        || row.3 != receipt.request_posting_digest
        || row.3 != digest
        || row.4 != receipt.requested_at
    {
        bail!("提现申请 Posting 摘要或金额审计失败");
    }
    audit_legs(conn, receipt)
}

fn audit_legs(
    conn: &Connection,
    receipt: &ComputeSettlementWithdrawalRequestReceipt,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT line_no, leg_kind, account_id, direction, amount_micros,
                balance_state, balance_after_micros, account_revision_after
           FROM compute_settlement_withdrawal_request_ledger_legs
          WHERE posting_id=?1 ORDER BY line_no",
    )?;
    let legs = stmt
        .query_map(params![receipt.request_posting_id], |row| {
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
    let expected = vec![
        (
            1,
            "provider_available_withdrawal_reserve".to_string(),
            receipt.provider_account_id.clone(),
            "debit".to_string(),
            receipt.amount_micros,
            "available".to_string(),
            receipt.available_balance_after_micros,
            receipt.account_revision_after,
        ),
        (
            2,
            "provider_withdrawn_reserve_credit".to_string(),
            receipt.provider_account_id.clone(),
            "credit".to_string(),
            receipt.amount_micros,
            "withdrawn".to_string(),
            receipt.withdrawn_balance_after_micros,
            receipt.account_revision_after,
        ),
    ];
    if legs != expected {
        bail!("提现申请账本腿或历史余额快照审计失败");
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
        .ok_or_else(|| anyhow!("提现申请 Provider 结算账户不存在"))?;
    let released_available = conn.query_row(
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
    if current.0 != released_available - reserved + returned
        || current.1 != reserved - returned
        || current.0 < 0
        || current.1 < 0
    {
        bail!("提现申请账户余额与不可变账本投影不一致");
    }
    Ok(())
}
