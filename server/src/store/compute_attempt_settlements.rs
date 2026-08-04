use anyhow::{bail, Result};
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    execution::ComputeJobVersionBinding,
    market::ComputePriceSnapshot,
    receipts::{ComputeSettlementAmounts, ComputeSettlementReceipt},
};

use super::{
    compute_attempt_execution_receipts::ComputeAttemptExecutionReceiptEnvelope,
    compute_attempt_finalizations::ComputeAttemptFinalizationReceipt, Store,
};

pub(super) mod calculation;
mod money;
mod orchestrate;
mod pending_candidate;
mod pending_queue;
mod support;

use pending_candidate::build_pending_settlement_candidate_on;
use pending_queue::list_pending_settlement_lease_ids_on;
use support::{
    normalize_settlement_request, persist_settlement_on, settlement_by_idempotency_on,
    settlement_by_lease_on, settlement_request_digest,
};

pub(crate) const COMPUTE_ATTEMPT_SETTLEMENT_SCHEMA: &str =
    "compute_federation.attempt_settlement.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SettleComputeAttemptRequest {
    pub lease_id: String,
    pub expected_finalization_id: String,
    pub expected_finalization_event_digest: String,
    pub expected_execution_receipt_id: String,
    pub expected_execution_receipt_digest: String,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub expected_budget_reservation_id: String,
    pub expected_price_snapshot_id: String,
    pub expected_price_snapshot_digest: String,
    pub idempotency_key: String,
    pub settled_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeAttemptSettlementReceipt {
    pub schema: String,
    pub settlement: ComputeSettlementReceipt,
    pub lease_id: String,
    pub finalization_id: String,
    pub finalization_event_digest: String,
    pub budget_reservation_id: String,
    pub budget_reserved_fen: i64,
    pub consumer_charged_fen: i64,
    pub consumer_refunded_fen: i64,
    pub consumer_balance_after_fen: i64,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub source_job: ComputeJobVersionBinding,
    pub terminal_job: ComputeJobVersionBinding,
    pub posting_id: String,
    pub posting_digest: String,
    pub provider_pending_balance_micros: i64,
    pub platform_pending_balance_micros: i64,
    pub request_digest: String,
    pub event_digest: String,
    pub settled_by_user_id: String,
    pub settled_at: String,
    pub money_effect: String,
    pub provider_balance_effect: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputePendingAttemptSettlementPreview {
    pub currency: &'static str,
    pub budget_reserved_fen: i64,
    pub consumer_charge_fen: i64,
    pub consumer_refund_fen: i64,
    pub amounts: ComputeSettlementAmounts,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputePendingAttemptSettlementCandidate {
    pub finalization: ComputeAttemptFinalizationReceipt,
    pub execution_receipt: ComputeAttemptExecutionReceiptEnvelope,
    pub expected_job: ComputeJobVersionBinding,
    pub expected_budget_reservation_id: String,
    pub price_snapshot: ComputePriceSnapshot,
    pub provider_account_id: String,
    pub preview: ComputePendingAttemptSettlementPreview,
    pub money_effect: &'static str,
    pub provider_balance_effect: &'static str,
    pub external_payment_effect: &'static str,
}

impl Store {
    pub(crate) fn settle_compute_attempt(
        &self,
        input: &SettleComputeAttemptRequest,
    ) -> Result<ComputeAttemptSettlementReceipt> {
        let input = normalize_settlement_request(input)?;
        let request_digest = settlement_request_digest(&input)?;
        let idempotency_scope = format!("compute_attempt_settlement:{}", input.settled_by_user_id);
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            settlement_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同 Attempt 结算幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }
        if let Some(stored) = settlement_by_lease_on(&tx, &input.lease_id)? {
            if stored.request_digest != request_digest {
                bail!("同一 Attempt Lease 已绑定另一份结算回执");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let receipt =
            orchestrate::settle_attempt_on(&tx, &input, &request_digest, &idempotency_scope)?;
        persist_settlement_on(&tx, &input, &receipt, &idempotency_scope)?;
        let stored = settlement_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
            .ok_or_else(|| anyhow::anyhow!("Attempt 结算回执写入后不可见"))?;
        let receipt = stored.into_receipt(&tx, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_attempt_settlement(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptSettlementReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        compute_attempt_settlement_on(&*conn, lease_id)
    }

    pub(crate) fn list_pending_compute_attempt_settlements(
        &self,
        limit: usize,
    ) -> Result<Vec<ComputePendingAttemptSettlementCandidate>> {
        let conn = self.conn()?;
        list_pending_settlement_lease_ids_on(&conn, limit.clamp(1, 100))?
            .into_iter()
            .map(|lease_id| build_pending_settlement_candidate_on(&conn, &lease_id))
            .collect()
    }
}

pub(super) fn compute_attempt_settlement_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<ComputeAttemptSettlementReceipt> {
    support::validate_exact("Attempt Lease ID", lease_id, 200)?;
    let stored = settlement_by_lease_on(conn, lease_id)?
        .ok_or_else(|| anyhow::anyhow!("Attempt 尚无结算回执"))?;
    stored.into_receipt(conn, false)
}
