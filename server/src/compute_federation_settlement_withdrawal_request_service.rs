use anyhow::{bail, Result};
use serde::Deserialize;

use crate::store::{
    ComputeSettlementWithdrawalRequestReceipt, CreateComputeSettlementWithdrawalRequest, Store,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateMyComputeSettlementWithdrawalBody {
    pub amount_micros: i64,
    pub destination_kind: String,
    pub destination_ref: String,
    pub idempotency_key: String,
    pub confirm_internal_reserve_only: bool,
    pub confirm_destination_ref_contains_no_secret: bool,
}

pub(crate) fn create_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    body: CreateMyComputeSettlementWithdrawalBody,
) -> Result<ComputeSettlementWithdrawalRequestReceipt> {
    if !body.confirm_internal_reserve_only {
        bail!("提交前必须确认本接口只冻结内部余额，不执行外部付款");
    }
    if !body.confirm_destination_ref_contains_no_secret {
        bail!("提交前必须确认目标引用不包含密码、私钥或助记词");
    }
    let provider = owned_provider(store, user_id, provider_id)?;
    let account_id = provider
        .provider
        .settlement_account_id
        .clone()
        .unwrap_or_else(|| provider.provider.owner_account_id.clone());
    store.create_compute_settlement_withdrawal_request(&CreateComputeSettlementWithdrawalRequest {
        provider_id: provider.provider.provider_id,
        expected_provider_policy_revision: provider.provider.policy_revision,
        expected_provider_digest: provider.provider_digest,
        provider_account_id: account_id,
        owner_user_id: user_id.to_string(),
        amount_micros: body.amount_micros,
        destination_kind: body.destination_kind,
        destination_ref: body.destination_ref,
        idempotency_key: body.idempotency_key,
    })
}

pub(crate) fn get_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    withdrawal_id: &str,
) -> Result<ComputeSettlementWithdrawalRequestReceipt> {
    owned_provider(store, user_id, provider_id)?;
    let receipt = store.compute_settlement_withdrawal_request(withdrawal_id)?;
    ensure_receipt_owned(&receipt, user_id, provider_id)?;
    Ok(receipt)
}

pub(crate) fn list_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    limit: usize,
) -> Result<Vec<ComputeSettlementWithdrawalRequestReceipt>> {
    owned_provider(store, user_id, provider_id)?;
    let receipts = store.list_compute_settlement_withdrawal_requests(provider_id, limit)?;
    for receipt in &receipts {
        ensure_receipt_owned(receipt, user_id, provider_id)?;
    }
    Ok(receipts)
}

fn owned_provider(
    store: &Store,
    user_id: &str,
    provider_id: &str,
) -> Result<crate::store::ComputeProviderRegistrationReceipt> {
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    Ok(provider)
}

fn ensure_receipt_owned(
    receipt: &ComputeSettlementWithdrawalRequestReceipt,
    user_id: &str,
    provider_id: &str,
) -> Result<()> {
    if receipt.owner_user_id != user_id || receipt.provider_id != provider_id {
        bail!("提现申请不属于当前登录用户或指定 Provider");
    }
    Ok(())
}
