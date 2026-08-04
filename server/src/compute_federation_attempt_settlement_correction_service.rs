use anyhow::{bail, Result};
use serde::Deserialize;

use crate::{
    compute_federation_attempt_service::get_for_participant,
    store::{
        ComputePendingSettlementCorrectionCandidate, ComputeSettlementCorrectionReceipt,
        CorrectComputeAttemptSettlementRequest, Store,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CorrectComputeAttemptSettlementBody {
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
    pub confirm_consumer_refund_and_pending_reversal: bool,
}

pub(crate) fn correct_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    lease_id: &str,
    body: CorrectComputeAttemptSettlementBody,
) -> Result<ComputeSettlementCorrectionReceipt> {
    if !body.confirm_consumer_refund_and_pending_reversal {
        bail!("纠正前必须确认消费者会收到退款且 Provider/平台 pending 会被冲减");
    }
    store.correct_compute_attempt_settlement(&CorrectComputeAttemptSettlementRequest {
        lease_id: lease_id.to_string(),
        expected_challenge_id: body.expected_challenge_id,
        expected_challenge_event_digest: body.expected_challenge_event_digest,
        expected_resolution_id: body.expected_resolution_id,
        expected_resolution_event_digest: body.expected_resolution_event_digest,
        expected_settlement_receipt_id: body.expected_settlement_receipt_id,
        expected_settlement_event_digest: body.expected_settlement_event_digest,
        corrected_consumer_charge_fen: body.corrected_consumer_charge_fen,
        corrected_provider_payable_micros: body.corrected_provider_payable_micros,
        corrected_platform_margin_micros: body.corrected_platform_margin_micros,
        statement: body.statement,
        evidence_refs: body.evidence_refs,
        idempotency_key: body.idempotency_key,
        corrected_by_user_id: admin_user_id.to_string(),
    })
}

pub(crate) fn get_for_attempt_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeSettlementCorrectionReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_settlement_correction(lease_id)
}

pub(crate) fn get_for_platform_admin(
    store: &Store,
    lease_id: &str,
) -> Result<ComputeSettlementCorrectionReceipt> {
    store.compute_settlement_correction(lease_id)
}

pub(crate) fn list_pending_for_platform_admin(
    store: &Store,
    limit: usize,
) -> Result<Vec<ComputePendingSettlementCorrectionCandidate>> {
    store.list_pending_compute_settlement_corrections(limit)
}
