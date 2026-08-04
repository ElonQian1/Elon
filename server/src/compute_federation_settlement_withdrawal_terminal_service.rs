use anyhow::{bail, Result};
use serde::Deserialize;

use crate::store::{
    ComputeSettlementWithdrawalRequestReceipt, ComputeSettlementWithdrawalTerminalReceipt, Store,
    TerminalizeComputeSettlementWithdrawalRequest,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CancelMyComputeSettlementWithdrawalBody {
    pub expected_withdrawal_event_digest: String,
    pub expected_request_posting_id: String,
    pub expected_request_posting_digest: String,
    pub reason_code: String,
    pub reason_detail: Option<String>,
    pub idempotency_key: String,
    pub confirm_internal_refund_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdminTerminalizeComputeSettlementWithdrawalBody {
    pub expected_withdrawal_event_digest: String,
    pub expected_request_posting_id: String,
    pub expected_request_posting_digest: String,
    pub action: String,
    pub reason_code: String,
    pub reason_detail: Option<String>,
    pub external_evidence_kind: Option<String>,
    pub external_evidence_ref: Option<String>,
    pub external_evidence_digest: Option<String>,
    pub idempotency_key: String,
    pub confirm_refund_or_attestation_only: bool,
    pub confirm_external_payment_already_completed: bool,
    pub confirm_evidence_ref_contains_no_secret: bool,
}

pub(crate) fn cancel_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    withdrawal_id: &str,
    body: CancelMyComputeSettlementWithdrawalBody,
) -> Result<ComputeSettlementWithdrawalTerminalReceipt> {
    if !body.confirm_internal_refund_only {
        bail!("取消前必须确认只执行 withdrawn 到 available 的内部返还");
    }
    let withdrawal = owned_withdrawal(store, user_id, provider_id, withdrawal_id)?;
    store.terminalize_compute_settlement_withdrawal(
        &TerminalizeComputeSettlementWithdrawalRequest {
            withdrawal_id: withdrawal.withdrawal_id,
            expected_withdrawal_event_digest: body.expected_withdrawal_event_digest,
            expected_request_posting_id: body.expected_request_posting_id,
            expected_request_posting_digest: body.expected_request_posting_digest,
            action: "cancelled".to_string(),
            reason_code: body.reason_code,
            reason_detail: body.reason_detail,
            external_evidence_kind: None,
            external_evidence_ref: None,
            external_evidence_digest: None,
            actor_user_id: user_id.to_string(),
            actor_role: "provider_owner".to_string(),
            idempotency_key: body.idempotency_key,
            confirm_refund_or_attestation_only: true,
            confirm_external_payment_already_completed: false,
        },
    )
}

pub(crate) fn terminalize_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    withdrawal_id: &str,
    body: AdminTerminalizeComputeSettlementWithdrawalBody,
) -> Result<ComputeSettlementWithdrawalTerminalReceipt> {
    if !matches!(body.action.as_str(), "rejected" | "external_paid_attested") {
        bail!("管理员终态只能是 rejected 或 external_paid_attested");
    }
    if !body.confirm_refund_or_attestation_only {
        bail!("终态前必须确认只执行内部返还或登记外部付款声明");
    }
    if body.action == "external_paid_attested" && !body.confirm_evidence_ref_contains_no_secret {
        bail!("登记外部付款声明前必须确认其证据引用不含密码、私钥或助记词");
    }
    store.compute_settlement_withdrawal_request(withdrawal_id)?;
    store.terminalize_compute_settlement_withdrawal(
        &TerminalizeComputeSettlementWithdrawalRequest {
            withdrawal_id: withdrawal_id.to_string(),
            expected_withdrawal_event_digest: body.expected_withdrawal_event_digest,
            expected_request_posting_id: body.expected_request_posting_id,
            expected_request_posting_digest: body.expected_request_posting_digest,
            action: body.action,
            reason_code: body.reason_code,
            reason_detail: body.reason_detail,
            external_evidence_kind: body.external_evidence_kind,
            external_evidence_ref: body.external_evidence_ref,
            external_evidence_digest: body.external_evidence_digest,
            actor_user_id: admin_user_id.to_string(),
            actor_role: "platform_admin".to_string(),
            idempotency_key: body.idempotency_key,
            confirm_refund_or_attestation_only: true,
            confirm_external_payment_already_completed: body
                .confirm_external_payment_already_completed,
        },
    )
}

pub(crate) fn get_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    withdrawal_id: &str,
) -> Result<ComputeSettlementWithdrawalTerminalReceipt> {
    owned_withdrawal(store, user_id, provider_id, withdrawal_id)?;
    store.compute_settlement_withdrawal_terminal(withdrawal_id)
}

pub(crate) fn get_request_for_platform_admin(
    store: &Store,
    withdrawal_id: &str,
) -> Result<ComputeSettlementWithdrawalRequestReceipt> {
    store.compute_settlement_withdrawal_request(withdrawal_id)
}

pub(crate) fn get_terminal_for_platform_admin(
    store: &Store,
    withdrawal_id: &str,
) -> Result<ComputeSettlementWithdrawalTerminalReceipt> {
    store.compute_settlement_withdrawal_terminal(withdrawal_id)
}

fn owned_withdrawal(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    withdrawal_id: &str,
) -> Result<ComputeSettlementWithdrawalRequestReceipt> {
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    let withdrawal = store.compute_settlement_withdrawal_request(withdrawal_id)?;
    if withdrawal.provider_id != provider_id || withdrawal.owner_user_id != user_id {
        bail!("提现申请不属于当前登录用户或指定 Provider");
    }
    Ok(withdrawal)
}
