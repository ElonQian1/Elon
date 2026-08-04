use anyhow::{bail, Result};
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::Store;

mod support;

use support::{
    normalize_request, persist_request_on, request_by_id_on, request_by_idempotency_on,
    request_digest, request_withdrawal_on,
};

pub(crate) const COMPUTE_SETTLEMENT_WITHDRAWAL_REQUEST_SCHEMA: &str =
    "compute_federation.settlement_withdrawal_request.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateComputeSettlementWithdrawalRequest {
    pub provider_id: String,
    pub expected_provider_policy_revision: i64,
    pub expected_provider_digest: String,
    pub provider_account_id: String,
    pub owner_user_id: String,
    pub amount_micros: i64,
    pub destination_kind: String,
    pub destination_ref: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeSettlementWithdrawalRequestReceipt {
    pub schema: String,
    pub withdrawal_id: String,
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_account_id: String,
    pub owner_user_id: String,
    pub currency: String,
    pub amount_micros: i64,
    pub destination_kind: String,
    pub destination_ref: String,
    pub available_balance_after_micros: i64,
    pub withdrawn_balance_after_micros: i64,
    pub account_revision_after: i64,
    pub request_posting_id: String,
    pub request_posting_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub requested_at: String,
    pub fund_effect: String,
    pub external_transfer_effect: String,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn create_compute_settlement_withdrawal_request(
        &self,
        input: &CreateComputeSettlementWithdrawalRequest,
    ) -> Result<ComputeSettlementWithdrawalRequestReceipt> {
        let input = normalize_request(input)?;
        let digest = request_digest(&input)?;
        let idempotency_scope = format!(
            "compute_settlement_withdrawal_request:{}:{}",
            input.owner_user_id, input.provider_id
        );
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            request_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != digest {
                bail!("相同提现申请幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let receipt = request_withdrawal_on(&tx, &input, &digest)?;
        persist_request_on(&tx, &input, &receipt, &idempotency_scope)?;
        let stored = request_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
            .ok_or_else(|| anyhow::anyhow!("提现申请回执写入后不可见"))?;
        let receipt = stored.into_receipt(&tx, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_settlement_withdrawal_request(
        &self,
        withdrawal_id: &str,
    ) -> Result<ComputeSettlementWithdrawalRequestReceipt> {
        support::validate_exact("Withdrawal ID", withdrawal_id, 240)?;
        let conn = self.conn()?;
        compute_settlement_withdrawal_request_on(&conn, withdrawal_id)
    }

    pub(crate) fn list_compute_settlement_withdrawal_requests(
        &self,
        provider_id: &str,
        limit: usize,
    ) -> Result<Vec<ComputeSettlementWithdrawalRequestReceipt>> {
        support::validate_exact("Provider ID", provider_id, 160)?;
        let conn = self.conn()?;
        support::list_requests_on(&conn, provider_id, limit.clamp(1, 100))?
            .into_iter()
            .map(|stored| stored.into_receipt(&conn, false))
            .collect()
    }
}

pub(super) fn compute_settlement_withdrawal_request_on(
    conn: &Connection,
    withdrawal_id: &str,
) -> Result<ComputeSettlementWithdrawalRequestReceipt> {
    support::validate_exact("Withdrawal ID", withdrawal_id, 240)?;
    let stored = request_by_id_on(conn, withdrawal_id)?
        .ok_or_else(|| anyhow::anyhow!("算力结算提现申请不存在"))?;
    stored.into_receipt(conn, false)
}
