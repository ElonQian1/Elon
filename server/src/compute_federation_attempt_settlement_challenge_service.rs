use anyhow::{bail, Result};
use serde::Deserialize;

use crate::{
    compute_federation_attempt_service::get_for_participant,
    store::{
        ComputePendingSettlementChallengeCandidate, ComputeSettlementChallengeReceipt,
        OpenComputeSettlementChallengeRequest, Store,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OpenComputeSettlementChallengeBody {
    pub expected_settlement_receipt_id: String,
    pub expected_settlement_event_digest: String,
    pub expected_posting_id: String,
    pub expected_posting_digest: String,
    pub reason_code: String,
    pub summary: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub idempotency_key: String,
    pub confirm_pending_release_block: bool,
}

pub(crate) fn open_for_consumer(
    store: &Store,
    consumer_user_id: &str,
    lease_id: &str,
    body: OpenComputeSettlementChallengeBody,
) -> Result<ComputeSettlementChallengeReceipt> {
    if !body.confirm_pending_release_block {
        bail!("提出结算挑战前必须确认该挑战会阻断 pending 收益释放");
    }
    get_for_participant(store, consumer_user_id, lease_id)?;
    store.open_compute_settlement_challenge(&OpenComputeSettlementChallengeRequest {
        lease_id: lease_id.to_string(),
        expected_settlement_receipt_id: body.expected_settlement_receipt_id,
        expected_settlement_event_digest: body.expected_settlement_event_digest,
        expected_posting_id: body.expected_posting_id,
        expected_posting_digest: body.expected_posting_digest,
        reason_code: body.reason_code,
        summary: body.summary,
        evidence_refs: body.evidence_refs,
        idempotency_key: body.idempotency_key,
        opened_by_user_id: consumer_user_id.to_string(),
    })
}

pub(crate) fn get_for_attempt_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeSettlementChallengeReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_settlement_challenge(lease_id)
}

pub(crate) fn get_for_platform_admin(
    store: &Store,
    lease_id: &str,
) -> Result<ComputeSettlementChallengeReceipt> {
    store.compute_settlement_challenge(lease_id)
}

pub(crate) fn list_pending_for_consumer(
    store: &Store,
    consumer_user_id: &str,
    limit: usize,
) -> Result<Vec<ComputePendingSettlementChallengeCandidate>> {
    store.list_pending_compute_settlement_challenges(consumer_user_id, limit)
}
