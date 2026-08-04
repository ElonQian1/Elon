use anyhow::{bail, Result};
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::Store;

mod support;

use support::{
    normalize_request, persist_terminal_on, terminal_by_idempotency_on, terminal_by_withdrawal_on,
    terminal_digest, terminalize_on,
};

pub(crate) const COMPUTE_SETTLEMENT_WITHDRAWAL_TERMINAL_SCHEMA: &str =
    "compute_federation.settlement_withdrawal_terminal.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TerminalizeComputeSettlementWithdrawalRequest {
    pub withdrawal_id: String,
    pub expected_withdrawal_event_digest: String,
    pub expected_request_posting_id: String,
    pub expected_request_posting_digest: String,
    pub action: String,
    pub reason_code: String,
    pub reason_detail: Option<String>,
    pub external_evidence_kind: Option<String>,
    pub external_evidence_ref: Option<String>,
    pub external_evidence_digest: Option<String>,
    pub actor_user_id: String,
    pub actor_role: String,
    pub idempotency_key: String,
    pub confirm_refund_or_attestation_only: bool,
    pub confirm_external_payment_already_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeSettlementWithdrawalTerminalReceipt {
    pub schema: String,
    pub terminal_id: String,
    pub withdrawal_id: String,
    pub withdrawal_event_digest: String,
    pub request_posting_id: String,
    pub request_posting_digest: String,
    pub provider_id: String,
    pub provider_account_id: String,
    pub owner_user_id: String,
    pub currency: String,
    pub amount_micros: i64,
    pub action: String,
    pub reason_code: String,
    pub reason_detail: Option<String>,
    pub external_evidence_kind: Option<String>,
    pub external_evidence_ref: Option<String>,
    pub external_evidence_digest: Option<String>,
    pub balance_returned_micros: i64,
    pub available_balance_after_micros: i64,
    pub withdrawn_balance_after_micros: i64,
    pub account_revision_after: i64,
    pub terminal_posting_id: String,
    pub terminal_posting_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub actor_user_id: String,
    pub actor_role: String,
    pub terminal_at: String,
    pub fund_effect: String,
    pub external_transfer_effect: String,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn terminalize_compute_settlement_withdrawal(
        &self,
        input: &TerminalizeComputeSettlementWithdrawalRequest,
    ) -> Result<ComputeSettlementWithdrawalTerminalReceipt> {
        let input = normalize_request(input)?;
        let digest = terminal_digest(&input)?;
        let idempotency_scope = format!(
            "compute_settlement_withdrawal_terminal:{}:{}",
            input.actor_role, input.actor_user_id
        );
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            terminal_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != digest {
                bail!("相同提现终态幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }
        if let Some(stored) = terminal_by_withdrawal_on(&tx, &input.withdrawal_id)? {
            if stored.request_digest != digest {
                bail!("同一提现申请已经绑定另一份唯一终态");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let receipt = terminalize_on(&tx, &input, &digest)?;
        persist_terminal_on(&tx, &input, &receipt, &idempotency_scope)?;
        let stored = terminal_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
            .ok_or_else(|| anyhow::anyhow!("提现终态回执写入后不可见"))?;
        let receipt = stored.into_receipt(&tx, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_settlement_withdrawal_terminal(
        &self,
        withdrawal_id: &str,
    ) -> Result<ComputeSettlementWithdrawalTerminalReceipt> {
        support::validate_exact("Withdrawal ID", withdrawal_id, 240)?;
        let conn = self.conn()?;
        compute_settlement_withdrawal_terminal_on(&conn, withdrawal_id)
    }
}

pub(super) fn compute_settlement_withdrawal_terminal_on(
    conn: &Connection,
    withdrawal_id: &str,
) -> Result<ComputeSettlementWithdrawalTerminalReceipt> {
    support::validate_exact("Withdrawal ID", withdrawal_id, 240)?;
    let stored = terminal_by_withdrawal_on(conn, withdrawal_id)?
        .ok_or_else(|| anyhow::anyhow!("算力结算提现申请尚无终态"))?;
    stored.into_receipt(conn, false)
}
