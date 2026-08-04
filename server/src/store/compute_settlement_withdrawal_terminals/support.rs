use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use sha2::{Digest, Sha256};

use super::{
    super::{
        common::new_id,
        compute_settlement_withdrawal_requests::compute_settlement_withdrawal_request_on,
    },
    ComputeSettlementWithdrawalTerminalReceipt, TerminalizeComputeSettlementWithdrawalRequest,
    COMPUTE_SETTLEMENT_WITHDRAWAL_TERMINAL_SCHEMA,
};

mod audit;
mod money;

#[derive(Debug, Clone)]
pub(super) struct StoredWithdrawalTerminal {
    pub terminal_id: String,
    pub withdrawal_id: String,
    pub withdrawal_event_digest: String,
    pub request_posting_id: String,
    pub request_posting_digest: String,
    pub provider_id: String,
    pub provider_account_id: String,
    pub owner_user_id: String,
    pub amount_micros: i64,
    pub action: String,
    pub reason_code: String,
    pub reason_detail: Option<String>,
    pub external_evidence_kind: Option<String>,
    pub external_evidence_ref: Option<String>,
    pub external_evidence_digest: Option<String>,
    pub balance_returned_micros: i64,
    pub terminal_posting_id: String,
    pub terminal_posting_digest: String,
    pub request_json: String,
    pub request_digest: String,
    pub receipt_json: String,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub actor_user_id: String,
    pub actor_role: String,
    pub terminal_at: String,
}

impl StoredWithdrawalTerminal {
    pub(super) fn into_receipt(
        &self,
        conn: &Connection,
        replayed: bool,
    ) -> Result<ComputeSettlementWithdrawalTerminalReceipt> {
        audit::audited_terminal_on(conn, self, replayed)
    }
}

pub(super) fn normalize_request(
    input: &TerminalizeComputeSettlementWithdrawalRequest,
) -> Result<TerminalizeComputeSettlementWithdrawalRequest> {
    let mut normalized = input.clone();
    for (label, value, max_len) in [
        ("Withdrawal ID", &mut normalized.withdrawal_id, 240),
        (
            "Withdrawal Request Posting ID",
            &mut normalized.expected_request_posting_id,
            240,
        ),
        ("终态动作", &mut normalized.action, 80),
        ("终态原因代码", &mut normalized.reason_code, 120),
        ("操作用户 ID", &mut normalized.actor_user_id, 240),
        ("操作角色", &mut normalized.actor_role, 80),
        ("幂等键", &mut normalized.idempotency_key, 240),
    ] {
        *value = value.trim().to_string();
        validate_exact(label, value, max_len)?;
    }
    for (label, value) in [
        (
            "Withdrawal 事件摘要",
            &mut normalized.expected_withdrawal_event_digest,
        ),
        (
            "Withdrawal Request Posting 摘要",
            &mut normalized.expected_request_posting_digest,
        ),
    ] {
        *value = value.trim().to_ascii_lowercase();
        validate_digest(label, value)?;
    }
    normalized.reason_detail = clean_optional(normalized.reason_detail, "终态原因说明", 1000)?;
    normalized.external_evidence_kind =
        clean_optional(normalized.external_evidence_kind, "外部付款证据类型", 100)?;
    normalized.external_evidence_ref =
        clean_optional(normalized.external_evidence_ref, "外部付款证据引用", 1000)?;
    normalized.external_evidence_digest = normalized
        .external_evidence_digest
        .map(|value| value.trim().to_ascii_lowercase());
    validate_action(&normalized)?;
    Ok(normalized)
}

fn validate_action(request: &TerminalizeComputeSettlementWithdrawalRequest) -> Result<()> {
    if !request.confirm_refund_or_attestation_only {
        bail!("必须确认终态只执行内部退款或登记外部付款声明");
    }
    match request.action.as_str() {
        "cancelled" => {
            if request.actor_role != "provider_owner" {
                bail!("只有 Provider 所有者可以取消提款申请");
            }
            ensure_no_external_evidence(request)?;
        }
        "rejected" => {
            if request.actor_role != "platform_admin" {
                bail!("只有平台管理员可以拒绝提款申请");
            }
            ensure_no_external_evidence(request)?;
        }
        "external_paid_attested" => {
            if request.actor_role != "platform_admin"
                || !request.confirm_external_payment_already_completed
            {
                bail!("只有平台管理员确认外部付款已完成后才能登记付款声明");
            }
            let kind = request
                .external_evidence_kind
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("外部已付款声明缺少证据类型"))?;
            if !matches!(
                kind,
                "bank_receipt"
                    | "payment_provider_receipt"
                    | "sui_transaction_digest"
                    | "other_receipt"
            ) {
                bail!("外部付款证据类型不受支持");
            }
            if request.external_evidence_ref.is_none() {
                bail!("外部已付款声明缺少证据引用");
            }
            validate_digest(
                "外部付款证据摘要",
                request
                    .external_evidence_digest
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("外部已付款声明缺少证据摘要"))?,
            )?;
        }
        _ => bail!("提现终态动作不受支持"),
    }
    if request.action != "external_paid_attested"
        && request.confirm_external_payment_already_completed
    {
        bail!("取消或拒绝动作不能声明外部付款已经完成");
    }
    Ok(())
}

fn ensure_no_external_evidence(
    request: &TerminalizeComputeSettlementWithdrawalRequest,
) -> Result<()> {
    if request.external_evidence_kind.is_some()
        || request.external_evidence_ref.is_some()
        || request.external_evidence_digest.is_some()
    {
        bail!("取消或拒绝动作不能携带外部付款证据");
    }
    Ok(())
}

pub(super) fn terminalize_on(
    tx: &Transaction<'_>,
    request: &TerminalizeComputeSettlementWithdrawalRequest,
    request_digest: &str,
) -> Result<ComputeSettlementWithdrawalTerminalReceipt> {
    let withdrawal = compute_settlement_withdrawal_request_on(tx, &request.withdrawal_id)?;
    if withdrawal.event_digest != request.expected_withdrawal_event_digest
        || withdrawal.request_posting_id != request.expected_request_posting_id
        || withdrawal.request_posting_digest != request.expected_request_posting_digest
    {
        bail!("提现终态引用的 Withdrawal Request Receipt 或 Posting 不匹配");
    }
    if request.actor_role == "provider_owner" && request.actor_user_id != withdrawal.owner_user_id {
        bail!("提款申请不属于当前 Provider 所有者");
    }

    let terminal_at = Utc::now().to_rfc3339();
    let terminal_id = new_id("compute_settlement_withdrawal_terminal");
    let returned = if matches!(request.action.as_str(), "cancelled" | "rejected") {
        withdrawal.amount_micros
    } else {
        0
    };
    let money = money::post_terminal_on(
        tx,
        money::PostTerminalInput {
            terminal_id: &terminal_id,
            withdrawal_id: &withdrawal.withdrawal_id,
            provider_account_id: &withdrawal.provider_account_id,
            action: &request.action,
            amount_micros: withdrawal.amount_micros,
            balance_returned_micros: returned,
            terminal_at: &terminal_at,
        },
    )?;
    let mut receipt = ComputeSettlementWithdrawalTerminalReceipt {
        schema: COMPUTE_SETTLEMENT_WITHDRAWAL_TERMINAL_SCHEMA.to_string(),
        terminal_id,
        withdrawal_id: withdrawal.withdrawal_id,
        withdrawal_event_digest: withdrawal.event_digest,
        request_posting_id: withdrawal.request_posting_id,
        request_posting_digest: withdrawal.request_posting_digest,
        provider_id: withdrawal.provider_id,
        provider_account_id: withdrawal.provider_account_id,
        owner_user_id: withdrawal.owner_user_id,
        currency: "CNY".to_string(),
        amount_micros: withdrawal.amount_micros,
        action: request.action.clone(),
        reason_code: request.reason_code.clone(),
        reason_detail: request.reason_detail.clone(),
        external_evidence_kind: request.external_evidence_kind.clone(),
        external_evidence_ref: request.external_evidence_ref.clone(),
        external_evidence_digest: request.external_evidence_digest.clone(),
        balance_returned_micros: returned,
        available_balance_after_micros: money.available_after_micros,
        withdrawn_balance_after_micros: money.withdrawn_after_micros,
        account_revision_after: money.revision_after,
        terminal_posting_id: money.posting_id,
        terminal_posting_digest: money.posting_digest,
        request_digest: request_digest.to_string(),
        event_digest: String::new(),
        actor_user_id: request.actor_user_id.clone(),
        actor_role: request.actor_role.clone(),
        terminal_at,
        fund_effect: if returned > 0 {
            "provider_withdrawn_returned_to_available".to_string()
        } else {
            "provider_withdrawn_balance_retained".to_string()
        },
        external_transfer_effect: if request.action == "external_paid_attested" {
            "external_payment_attested_not_executed_or_verified".to_string()
        } else {
            "not_executed".to_string()
        },
        replayed: false,
    };
    receipt.event_digest = event_digest(&receipt)?;
    Ok(receipt)
}

pub(super) fn terminal_digest(
    request: &TerminalizeComputeSettlementWithdrawalRequest,
) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(request)?)))
}

pub(super) fn event_digest(receipt: &ComputeSettlementWithdrawalTerminalReceipt) -> Result<String> {
    let mut canonical = receipt.clone();
    canonical.event_digest.clear();
    canonical.replayed = false;
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}

pub(super) fn persist_terminal_on(
    conn: &Connection,
    request: &TerminalizeComputeSettlementWithdrawalRequest,
    receipt: &ComputeSettlementWithdrawalTerminalReceipt,
    idempotency_scope: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_settlement_withdrawal_terminals (
           terminal_id, withdrawal_id, withdrawal_event_digest,
           request_posting_id, request_posting_digest, provider_id,
           provider_account_id, owner_user_id, currency, amount_micros,
           action, reason_code, reason_detail, external_evidence_kind,
           external_evidence_ref, external_evidence_digest,
           balance_returned_micros, terminal_posting_id,
           terminal_posting_digest, request_json, request_digest,
           receipt_json, event_digest, idempotency_scope, idempotency_key,
           actor_user_id, actor_role, terminal_at, created_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'CNY',?9,?10,?11,?12,?13,?14,
                   ?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?27)",
        params![
            receipt.terminal_id,
            receipt.withdrawal_id,
            receipt.withdrawal_event_digest,
            receipt.request_posting_id,
            receipt.request_posting_digest,
            receipt.provider_id,
            receipt.provider_account_id,
            receipt.owner_user_id,
            receipt.amount_micros,
            receipt.action,
            receipt.reason_code,
            receipt.reason_detail,
            receipt.external_evidence_kind,
            receipt.external_evidence_ref,
            receipt.external_evidence_digest,
            receipt.balance_returned_micros,
            receipt.terminal_posting_id,
            receipt.terminal_posting_digest,
            serde_json::to_string(request)?,
            receipt.request_digest,
            serde_json::to_string(receipt)?,
            receipt.event_digest,
            idempotency_scope,
            request.idempotency_key,
            receipt.actor_user_id,
            receipt.actor_role,
            receipt.terminal_at,
        ],
    )?;
    Ok(())
}

pub(super) fn terminal_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredWithdrawalTerminal>> {
    terminal_query(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn terminal_by_withdrawal_on(
    conn: &Connection,
    withdrawal_id: &str,
) -> Result<Option<StoredWithdrawalTerminal>> {
    terminal_query(conn, "WHERE withdrawal_id=?1", params![withdrawal_id])
}

const SELECT_TERMINAL: &str = "SELECT terminal_id, withdrawal_id, withdrawal_event_digest,
            request_posting_id, request_posting_digest, provider_id,
            provider_account_id, owner_user_id, amount_micros, action,
            reason_code, reason_detail, external_evidence_kind,
            external_evidence_ref, external_evidence_digest,
            balance_returned_micros, terminal_posting_id,
            terminal_posting_digest, request_json, request_digest,
            receipt_json, event_digest, idempotency_scope, idempotency_key,
            actor_user_id, actor_role, terminal_at
       FROM compute_settlement_withdrawal_terminals";

fn terminal_query<P>(
    conn: &Connection,
    where_clause: &str,
    values: P,
) -> Result<Option<StoredWithdrawalTerminal>>
where
    P: rusqlite::Params,
{
    conn.query_row(
        &format!("{SELECT_TERMINAL} {where_clause}"),
        values,
        stored_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_from_row(row: &Row<'_>) -> rusqlite::Result<StoredWithdrawalTerminal> {
    Ok(StoredWithdrawalTerminal {
        terminal_id: row.get(0)?,
        withdrawal_id: row.get(1)?,
        withdrawal_event_digest: row.get(2)?,
        request_posting_id: row.get(3)?,
        request_posting_digest: row.get(4)?,
        provider_id: row.get(5)?,
        provider_account_id: row.get(6)?,
        owner_user_id: row.get(7)?,
        amount_micros: row.get(8)?,
        action: row.get(9)?,
        reason_code: row.get(10)?,
        reason_detail: row.get(11)?,
        external_evidence_kind: row.get(12)?,
        external_evidence_ref: row.get(13)?,
        external_evidence_digest: row.get(14)?,
        balance_returned_micros: row.get(15)?,
        terminal_posting_id: row.get(16)?,
        terminal_posting_digest: row.get(17)?,
        request_json: row.get(18)?,
        request_digest: row.get(19)?,
        receipt_json: row.get(20)?,
        event_digest: row.get(21)?,
        idempotency_scope: row.get(22)?,
        idempotency_key: row.get(23)?,
        actor_user_id: row.get(24)?,
        actor_role: row.get(25)?,
        terminal_at: row.get(26)?,
    })
}

fn clean_optional(value: Option<String>, label: &str, max_len: usize) -> Result<Option<String>> {
    value
        .map(|value| {
            let value = value.trim().to_string();
            validate_exact(label, &value, max_len)?;
            Ok(value)
        })
        .transpose()
}

pub(super) fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty() || value != value.trim() {
        bail!("{label}不能为空或包含首尾空白");
    }
    if value.chars().count() > max_len {
        bail!("{label}长度不能超过 {max_len}");
    }
    Ok(())
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label}必须是 64 位十六进制摘要");
    }
    Ok(())
}
