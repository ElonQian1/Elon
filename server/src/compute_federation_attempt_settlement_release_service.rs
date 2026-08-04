use anyhow::{bail, Result};
use serde::Deserialize;

use crate::{
    compute_federation_attempt_service::get_for_participant,
    store::{ComputeSettlementReleaseReceipt, ReleaseComputeAttemptSettlementRequest, Store},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleaseComputeAttemptSettlementBody {
    pub expected_settlement_receipt_id: String,
    pub expected_settlement_event_digest: String,
    pub expected_posting_id: String,
    pub expected_posting_digest: String,
    pub idempotency_key: String,
    pub confirm_pending_to_available_only: bool,
}

pub(crate) fn release_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    lease_id: &str,
    body: ReleaseComputeAttemptSettlementBody,
) -> Result<ComputeSettlementReleaseReceipt> {
    if !body.confirm_pending_to_available_only {
        bail!("释放前必须确认资金仅从 pending 转入 available，不发生提现或外部转账");
    }
    store.release_compute_attempt_settlement(&ReleaseComputeAttemptSettlementRequest {
        lease_id: lease_id.to_string(),
        expected_settlement_receipt_id: body.expected_settlement_receipt_id,
        expected_settlement_event_digest: body.expected_settlement_event_digest,
        expected_posting_id: body.expected_posting_id,
        expected_posting_digest: body.expected_posting_digest,
        idempotency_key: body.idempotency_key,
        released_by_user_id: admin_user_id.to_string(),
    })
}

pub(crate) fn get_for_attempt_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeSettlementReleaseReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_settlement_release(lease_id)
}

pub(crate) fn get_for_platform_admin(
    store: &Store,
    lease_id: &str,
) -> Result<ComputeSettlementReleaseReceipt> {
    store.compute_settlement_release(lease_id)
}
