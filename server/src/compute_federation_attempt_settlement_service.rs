use anyhow::{bail, Result};
use serde::Deserialize;

use crate::{
    compute_federation_attempt_service::get_for_participant,
    store::{
        ComputeAttemptSettlementReceipt, ComputePendingAttemptSettlementCandidate,
        SettleComputeAttemptRequest, Store,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SettleComputeAttemptBody {
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
    pub confirm_consumer_capture_and_provider_pending: bool,
}

pub(crate) fn settle_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    lease_id: &str,
    body: SettleComputeAttemptBody,
) -> Result<ComputeAttemptSettlementReceipt> {
    if !body.confirm_consumer_capture_and_provider_pending {
        bail!("结算前必须确认消费者预授权会扣结且 Provider 收益只进入 pending");
    }
    store.settle_compute_attempt(&SettleComputeAttemptRequest {
        lease_id: lease_id.to_string(),
        expected_finalization_id: body.expected_finalization_id,
        expected_finalization_event_digest: body.expected_finalization_event_digest,
        expected_execution_receipt_id: body.expected_execution_receipt_id,
        expected_execution_receipt_digest: body.expected_execution_receipt_digest,
        expected_job_revision: body.expected_job_revision,
        expected_job_digest: body.expected_job_digest,
        expected_budget_reservation_id: body.expected_budget_reservation_id,
        expected_price_snapshot_id: body.expected_price_snapshot_id,
        expected_price_snapshot_digest: body.expected_price_snapshot_digest,
        idempotency_key: body.idempotency_key,
        settled_by_user_id: admin_user_id.to_string(),
    })
}

pub(crate) fn get_for_attempt_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeAttemptSettlementReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_attempt_settlement(lease_id)
}

pub(crate) fn list_pending_for_platform_admin(
    store: &Store,
    limit: usize,
) -> Result<Vec<ComputePendingAttemptSettlementCandidate>> {
    store.list_pending_compute_attempt_settlements(limit)
}
