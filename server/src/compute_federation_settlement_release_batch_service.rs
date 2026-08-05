use anyhow::{bail, Result};
use serde::Deserialize;
use uuid::Uuid;

use crate::store::{
    ComputeSettlementReleaseBatchFailure, ComputeSettlementReleaseBatchHistoryPage,
    ComputeSettlementReleaseBatchReport, ComputeSettlementReleaseCandidatePage,
    ReleaseComputeAttemptSettlementRequest, StartComputeSettlementReleaseBatch, Store,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleaseDueComputeSettlementsBody {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    pub confirm_each_item_uses_v198_internal_release_only: bool,
}

pub(crate) fn list_due_for_platform_admin(
    store: &Store,
    limit: usize,
    cursor: Option<&str>,
) -> Result<ComputeSettlementReleaseCandidatePage> {
    store.list_due_compute_settlement_release_candidates(limit, cursor)
}

pub(crate) fn list_batch_history_for_platform_admin(
    store: &Store,
    limit: usize,
    cursor: Option<&str>,
) -> Result<ComputeSettlementReleaseBatchHistoryPage> {
    store.list_compute_settlement_release_batch_history(limit, cursor)
}

pub(crate) fn release_due_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    body: ReleaseDueComputeSettlementsBody,
) -> Result<ComputeSettlementReleaseBatchReport> {
    if !body.confirm_each_item_uses_v198_internal_release_only {
        bail!("批量释放前必须确认每笔只执行 v198 pending 到 available 内部转账");
    }
    let current_page =
        store.list_due_compute_settlement_release_candidates(body.limit, body.cursor.as_deref())?;
    let start = store.start_or_resume_compute_settlement_release_batch(
        &StartComputeSettlementReleaseBatch {
            requested_by_user_id: admin_user_id.to_string(),
            requested_limit: current_page.limit,
            requested_cursor: body.cursor,
            idempotency_key: body
                .idempotency_key
                .unwrap_or_else(|| format!("release-batch:{}", Uuid::new_v4())),
        },
        &current_page,
    )?;
    if let Some(mut completed) = start.completed_report {
        completed.replayed = true;
        return Ok(completed);
    }
    let page = start.candidate_page;
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
                error: bounded_failure_error(error),
            }),
        }
    }
    let report = ComputeSettlementReleaseBatchReport {
        schema: "compute_federation.settlement_release_batch_report.v2".to_string(),
        batch_run_id: start.batch_run_id.clone(),
        replayed: false,
        scanned,
        eligible,
        total_due_candidates: page.total_due_candidates,
        has_more: page.has_more,
        next_cursor: page.next_cursor,
        released,
        skipped,
        failed,
        transaction_scope: "one_independent_begin_immediate_transaction_per_settlement".to_string(),
        money_effect: "eligible_provider_and_platform_pending_moved_to_available".to_string(),
        external_transfer_effect: "none".to_string(),
    };
    let (mut completed, replayed) =
        store.complete_compute_settlement_release_batch(&start.batch_run_id, &report)?;
    completed.replayed = replayed;
    Ok(completed)
}

fn default_limit() -> usize {
    50
}

fn bounded_failure_error(error: anyhow::Error) -> String {
    format!("{error:#}").chars().take(2_000).collect()
}
