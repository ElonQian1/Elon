use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::store::{
    ComputeSettlementReleaseCandidate, ComputeSettlementReleaseCandidatePage,
    ComputeSettlementReleaseReceipt, ReleaseComputeAttemptSettlementRequest, Store,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleaseDueComputeSettlementsBody {
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub confirm_each_item_uses_v198_internal_release_only: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeSettlementReleaseBatchFailure {
    pub lease_id: String,
    pub settlement_receipt_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeSettlementReleaseBatchReport {
    pub schema: String,
    pub scanned: usize,
    pub eligible: usize,
    pub released: Vec<ComputeSettlementReleaseReceipt>,
    pub skipped: Vec<ComputeSettlementReleaseCandidate>,
    pub failed: Vec<ComputeSettlementReleaseBatchFailure>,
    pub transaction_scope: String,
    pub money_effect: String,
    pub external_transfer_effect: String,
}

pub(crate) fn list_due_for_platform_admin(
    store: &Store,
    limit: usize,
) -> Result<ComputeSettlementReleaseCandidatePage> {
    store.list_due_compute_settlement_release_candidates(limit)
}

pub(crate) fn release_due_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    body: ReleaseDueComputeSettlementsBody,
) -> Result<ComputeSettlementReleaseBatchReport> {
    if !body.confirm_each_item_uses_v198_internal_release_only {
        bail!("批量释放前必须确认每笔只执行 v198 pending 到 available 内部转账");
    }
    let page = store.list_due_compute_settlement_release_candidates(body.limit)?;
    let scanned = page.candidates.len();
    let eligible = page
        .candidates
        .iter()
        .filter(|candidate| candidate.eligible)
        .count();
    let mut released = Vec::with_capacity(eligible);
    let mut skipped = Vec::new();
    let mut failed = Vec::new();
    for candidate in page.candidates {
        if !candidate.eligible {
            skipped.push(candidate);
            continue;
        }
        let request = ReleaseComputeAttemptSettlementRequest {
            lease_id: candidate.lease_id.clone(),
            expected_settlement_receipt_id: candidate.settlement_receipt_id.clone(),
            expected_settlement_event_digest: candidate.settlement_event_digest.clone(),
            expected_posting_id: candidate.settlement_posting_id.clone(),
            expected_posting_digest: candidate.settlement_posting_digest.clone(),
            idempotency_key: format!("due-release:{}", candidate.settlement_receipt_id),
            released_by_user_id: admin_user_id.to_string(),
        };
        match store.release_compute_attempt_settlement(&request) {
            Ok(receipt) => released.push(receipt),
            Err(error) => failed.push(ComputeSettlementReleaseBatchFailure {
                lease_id: candidate.lease_id,
                settlement_receipt_id: candidate.settlement_receipt_id,
                error: format!("{error:#}"),
            }),
        }
    }
    Ok(ComputeSettlementReleaseBatchReport {
        schema: "compute_federation.settlement_release_batch_report.v1".to_string(),
        scanned,
        eligible,
        released,
        skipped,
        failed,
        transaction_scope: "one_independent_begin_immediate_transaction_per_settlement".to_string(),
        money_effect: "eligible_provider_and_platform_pending_moved_to_available".to_string(),
        external_transfer_effect: "none".to_string(),
    })
}

fn default_limit() -> usize {
    50
}
