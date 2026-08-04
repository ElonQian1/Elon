use anyhow::{bail, Result};
use rusqlite::TransactionBehavior;
use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    capacity::ComputeCapacityClaimBinding,
    execution::{ComputeJobVersionBinding, ComputeReservedCapacity},
};

use super::Store;

mod capacity;
mod orchestrate;
mod support;

use support::{
    finalization_by_idempotency_on, finalization_by_lease_on, finalization_request_digest,
    normalize_finalization_request, persist_finalization_on,
};

pub(crate) const COMPUTE_ATTEMPT_FINALIZATION_SCHEMA: &str =
    "compute_federation.attempt_finalization.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FinalizeComputeAttemptRequest {
    pub lease_id: String,
    pub expected_execution_receipt_id: String,
    pub expected_execution_receipt_digest: String,
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
    pub expected_claim_revision: i64,
    pub expected_claim_digest: String,
    pub idempotency_key: String,
    pub finalized_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeAttemptRevisionBinding {
    pub revision: i64,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeAttemptCapacityTransactionRef {
    pub transaction_id: String,
    pub transaction_digest: String,
    pub ledger_sequence: i64,
    pub event_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeAttemptFinalizationReceipt {
    pub schema: String,
    pub finalization_id: String,
    pub execution_receipt_id: String,
    pub execution_receipt_digest: String,
    pub lease_id: String,
    pub provider_id: String,
    pub consumer_account_id: String,
    pub outcome: String,
    pub reason_code: String,
    pub source_lease: ComputeAttemptRevisionBinding,
    pub terminal_lease: ComputeAttemptRevisionBinding,
    pub source_job: ComputeJobVersionBinding,
    pub terminal_job: ComputeJobVersionBinding,
    pub source_reservation: ComputeAttemptRevisionBinding,
    pub terminal_reservation: ComputeAttemptRevisionBinding,
    pub source_claim: ComputeCapacityClaimBinding,
    pub terminal_claim: ComputeCapacityClaimBinding,
    pub compensable_usage: Vec<ComputeReservedCapacity>,
    pub capacity_consumed: Vec<ComputeReservedCapacity>,
    pub capacity_returned: Vec<ComputeReservedCapacity>,
    pub capacity_transactions: Vec<ComputeAttemptCapacityTransactionRef>,
    pub request_digest: String,
    pub event_digest: String,
    pub finalized_by_user_id: String,
    pub effective_at: String,
    pub finalized_at: String,
    pub execution_effect: String,
    pub lease_effect: String,
    pub job_effect: String,
    pub capacity_effect: String,
    pub reservation_effect: String,
    pub money_effect: String,
    pub settlement_effect: String,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn finalize_compute_attempt(
        &self,
        input: &FinalizeComputeAttemptRequest,
    ) -> Result<ComputeAttemptFinalizationReceipt> {
        let input = normalize_finalization_request(input)?;
        let request_digest = finalization_request_digest(&input)?;
        let idempotency_scope = format!(
            "compute_attempt_finalization:{}",
            input.finalized_by_user_id
        );
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            finalization_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同 Attempt 终态幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }
        if let Some(stored) = finalization_by_lease_on(&tx, &input.lease_id)? {
            if stored.request_digest != request_digest {
                bail!("同一 Attempt Lease 已绑定另一份可信终态回执");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let receipt =
            orchestrate::finalize_attempt_on(&tx, &input, &request_digest, &idempotency_scope)?;
        persist_finalization_on(&tx, &input, &receipt, &idempotency_scope)?;
        let stored =
            finalization_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
                .ok_or_else(|| anyhow::anyhow!("Attempt 可信终态回执写入后不可见"))?;
        let receipt = stored.into_receipt(&tx, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_attempt_finalization(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptFinalizationReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        let stored = finalization_by_lease_on(&*conn, lease_id)?
            .ok_or_else(|| anyhow::anyhow!("Attempt 尚无可信终态回执"))?;
        stored.into_receipt(&*conn, false)
    }
}
