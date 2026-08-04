use anyhow::{bail, Result};
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::Store;

mod support;

use support::{
    correction_by_id_on, correction_by_idempotency_on, correction_by_lease_on,
    correction_by_resolution_on, correction_request_digest, normalize_correction_request,
    persist_correction_on,
};

pub(crate) const COMPUTE_SETTLEMENT_CORRECTION_SCHEMA: &str =
    "compute_federation.settlement_correction.v1";
pub(crate) const COMPUTE_SETTLEMENT_CORRECTION_POLICY_ID: &str =
    "accepted_challenge_downward_correction_v1";
pub(crate) const COMPUTE_SETTLEMENT_CORRECTION_POLICY_VERSION: i64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CorrectComputeAttemptSettlementRequest {
    pub lease_id: String,
    pub expected_challenge_id: String,
    pub expected_challenge_event_digest: String,
    pub expected_resolution_id: String,
    pub expected_resolution_event_digest: String,
    pub expected_settlement_receipt_id: String,
    pub expected_settlement_event_digest: String,
    pub corrected_consumer_charge_fen: i64,
    pub corrected_provider_payable_micros: i64,
    pub corrected_platform_margin_micros: i64,
    pub statement: String,
    pub evidence_refs: Vec<String>,
    pub idempotency_key: String,
    pub corrected_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeSettlementCorrectionReceipt {
    pub schema: String,
    pub correction_id: String,
    pub challenge_id: String,
    pub challenge_event_digest: String,
    pub resolution_id: String,
    pub resolution_event_digest: String,
    pub settlement_receipt_id: String,
    pub settlement_event_digest: String,
    pub lease_id: String,
    pub consumer_account_id: String,
    pub provider_account_id: String,
    pub platform_account_id: String,
    pub currency: String,
    pub original_consumer_charge_fen: i64,
    pub original_consumer_charge_micros: i64,
    pub corrected_consumer_charge_fen: i64,
    pub corrected_consumer_charge_micros: i64,
    pub consumer_refund_fen: i64,
    pub consumer_refund_micros: i64,
    pub original_provider_payable_micros: i64,
    pub corrected_provider_payable_micros: i64,
    pub provider_reversal_micros: i64,
    pub original_platform_margin_micros: i64,
    pub corrected_platform_margin_micros: i64,
    pub platform_reversal_micros: i64,
    pub consumer_balance_after_fen: i64,
    pub provider_pending_balance_after_micros: i64,
    pub provider_account_revision_after: i64,
    pub platform_pending_balance_after_micros: i64,
    pub platform_account_revision_after: i64,
    pub statement: String,
    pub evidence_refs: Vec<String>,
    pub evidence_refs_digest: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub posting_id: String,
    pub posting_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub corrected_by_user_id: String,
    pub corrected_at: String,
    pub balance_effect: String,
    pub settlement_release_effect: String,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn correct_compute_attempt_settlement(
        &self,
        input: &CorrectComputeAttemptSettlementRequest,
    ) -> Result<ComputeSettlementCorrectionReceipt> {
        let input = normalize_correction_request(input)?;
        let request_digest = correction_request_digest(&input)?;
        let idempotency_scope = format!(
            "compute_settlement_correction:{}",
            input.corrected_by_user_id
        );
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            correction_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同结算纠正幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }
        if let Some(stored) = correction_by_lease_on(&tx, &input.lease_id)? {
            if stored.request_digest != request_digest {
                bail!("同一 Attempt Lease 已绑定另一份结算纠正回执");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let receipt = support::correct_settlement_on(&tx, &input, &request_digest)?;
        persist_correction_on(&tx, &input, &receipt, &idempotency_scope)?;
        let stored = correction_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
            .ok_or_else(|| anyhow::anyhow!("结算纠正回执写入后不可见"))?;
        let receipt = stored.into_receipt(&tx, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_settlement_correction(
        &self,
        lease_id: &str,
    ) -> Result<ComputeSettlementCorrectionReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        compute_settlement_correction_on(&*conn, lease_id)
    }
}

pub(super) fn compute_settlement_correction_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<ComputeSettlementCorrectionReceipt> {
    support::validate_exact("Attempt Lease ID", lease_id, 200)?;
    let stored = correction_by_lease_on(conn, lease_id)?
        .ok_or_else(|| anyhow::anyhow!("Attempt 结算尚无 accepted 挑战纠正回执"))?;
    stored.into_receipt(conn, false)
}

pub(super) fn compute_settlement_correction_by_resolution_on(
    conn: &Connection,
    resolution_id: &str,
) -> Result<Option<ComputeSettlementCorrectionReceipt>> {
    support::validate_exact("挑战决议 ID", resolution_id, 240)?;
    correction_by_resolution_on(conn, resolution_id)?
        .map(|stored| stored.into_receipt(conn, false))
        .transpose()
}

pub(super) fn compute_settlement_correction_by_id_on(
    conn: &Connection,
    correction_id: &str,
) -> Result<Option<ComputeSettlementCorrectionReceipt>> {
    support::validate_exact("结算纠正 ID", correction_id, 240)?;
    correction_by_id_on(conn, correction_id)?
        .map(|stored| stored.into_receipt(conn, false))
        .transpose()
}
