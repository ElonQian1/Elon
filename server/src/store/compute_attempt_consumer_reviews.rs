use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use rusqlite::{params, TransactionBehavior};
use serde::Serialize;

use super::{compute_attempt_terminals::compute_attempt_terminal_candidate_on, new_id, Store};

mod support;

use support::{
    consumer_review_by_candidate_on, consumer_review_by_idempotency_on,
    consumer_review_by_lease_on, consumer_review_event_digest, consumer_review_request_digest,
    ensure_candidate_binding, evidence_refs_digest, normalize_consumer_review_request,
    StoredConsumerReview,
};

pub(crate) const COMPUTE_ATTEMPT_CONSUMER_REVIEW_SCHEMA: &str =
    "compute_federation.attempt_consumer_review.v1";
pub(crate) const CONSUMER_REVIEW_ACCEPTED: &str = "accepted";
pub(crate) const CONSUMER_REVIEW_REJECTED: &str = "rejected";
pub(crate) const CONSUMER_REVIEW_DISPUTED: &str = "disputed";

#[derive(Debug, Clone)]
pub(crate) struct ReviewComputeAttemptTerminalCandidateRequest {
    pub lease_id: String,
    pub expected_terminal_candidate_id: String,
    pub expected_terminal_candidate_event_digest: String,
    pub decision: String,
    pub reason_code: String,
    pub consumer_review_ref: String,
    pub evidence_refs: Vec<String>,
    pub idempotency_key: String,
    pub reviewed_by_user_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptConsumerReviewReceipt {
    pub schema: &'static str,
    pub consumer_review_id: String,
    pub terminal_candidate_id: String,
    pub terminal_candidate_event_digest: String,
    pub lease_id: String,
    pub provider_id: String,
    pub consumer_account_id: String,
    pub source_lease_revision: i64,
    pub source_lease_digest: String,
    pub fencing_generation: i64,
    pub job_id: String,
    pub job_revision: i64,
    pub job_digest: String,
    pub reservation_id: String,
    pub reservation_revision: i64,
    pub reservation_digest: String,
    pub capacity_claim_id: String,
    pub capacity_claim_revision: i64,
    pub capacity_claim_digest: String,
    pub final_usage_snapshot_id: String,
    pub final_usage_sequence_no: i64,
    pub final_cumulative_usage_digest: String,
    pub candidate_outcome: String,
    pub decision: String,
    pub reason_code: String,
    pub consumer_review_ref: String,
    pub evidence_refs: Vec<String>,
    pub evidence_refs_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub reviewed_by_user_id: String,
    pub reviewed_at: String,
    pub evidence_status: &'static str,
    pub review_effect: &'static str,
    pub verification_effect: &'static str,
    pub lease_effect: &'static str,
    pub job_effect: &'static str,
    pub capacity_effect: &'static str,
    pub reservation_effect: &'static str,
    pub money_effect: &'static str,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn review_compute_attempt_terminal_candidate(
        &self,
        input: &ReviewComputeAttemptTerminalCandidateRequest,
    ) -> Result<ComputeAttemptConsumerReviewReceipt> {
        let input = normalize_consumer_review_request(input)?;
        let request_digest = consumer_review_request_digest(&input)?;
        let idempotency_scope = format!(
            "compute_attempt_consumer_review:{}",
            input.reviewed_by_user_id
        );
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            consumer_review_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同消费者终态审核幂等键不能用于不同请求");
            }
            let receipt = consumer_review_receipt_on(&tx, stored, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let candidate = compute_attempt_terminal_candidate_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无 Provider 终态候选"))?;
        if candidate.terminal_candidate_id != input.expected_terminal_candidate_id
            || candidate.event_digest != input.expected_terminal_candidate_event_digest
        {
            bail!("消费者审核必须绑定精确的 Provider 终态候选 ID 与事件摘要");
        }
        if candidate.consumer_account_id != input.reviewed_by_user_id {
            bail!("只有当前 Job 消费者可以提交终态审核证据");
        }

        if let Some(stored) =
            consumer_review_by_candidate_on(&tx, &candidate.terminal_candidate_id)?
        {
            if stored.request_digest != request_digest {
                bail!("同一 Provider 终态候选已绑定另一份消费者审核证据");
            }
            let receipt = consumer_review_receipt_on(&tx, stored, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let evidence_refs_digest = evidence_refs_digest(&input.evidence_refs)?;
        let reviewed_at = Utc::now().to_rfc3339();
        let consumer_review_id = new_id("compute_attempt_consumer_review");
        let event_digest = consumer_review_event_digest(
            &consumer_review_id,
            &input,
            &candidate,
            &evidence_refs_digest,
            &request_digest,
            &reviewed_at,
        )?;

        tx.execute(
            "INSERT INTO compute_attempt_consumer_reviews (
                consumer_review_id, terminal_candidate_id,
                terminal_candidate_event_digest, lease_id, provider_id,
                consumer_account_id, source_lease_revision, source_lease_digest,
                fencing_generation, job_id, job_revision, job_digest,
                reservation_id, reservation_revision, reservation_digest,
                capacity_claim_id, capacity_claim_revision, capacity_claim_digest,
                final_usage_snapshot_id, final_usage_sequence_no,
                final_cumulative_usage_digest, candidate_outcome, decision,
                reason_code, consumer_review_ref, evidence_refs_json,
                evidence_refs_digest, request_digest, event_digest,
                idempotency_scope, idempotency_key, reviewed_by_user_id,
                reviewed_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23,
                       ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?33)",
            params![
                consumer_review_id,
                candidate.terminal_candidate_id,
                candidate.event_digest,
                candidate.lease_id,
                candidate.provider_id,
                candidate.consumer_account_id,
                candidate.source_lease_revision,
                candidate.source_lease_digest,
                candidate.fencing_generation,
                candidate.job_id,
                candidate.job_revision,
                candidate.job_digest,
                candidate.reservation_id,
                candidate.reservation_revision,
                candidate.reservation_digest,
                candidate.capacity_claim_id,
                candidate.capacity_claim_revision,
                candidate.capacity_claim_digest,
                candidate.final_usage_snapshot_id,
                candidate.final_usage_sequence_no,
                candidate.final_cumulative_usage_digest,
                candidate.outcome,
                input.decision,
                input.reason_code,
                input.consumer_review_ref,
                serde_json::to_string(&input.evidence_refs)?,
                evidence_refs_digest,
                request_digest,
                event_digest,
                idempotency_scope,
                input.idempotency_key,
                input.reviewed_by_user_id,
                reviewed_at,
            ],
        )?;

        let stored =
            consumer_review_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
                .ok_or_else(|| anyhow!("消费者终态审核写入后不可见"))?;
        let receipt = consumer_review_receipt_on(&tx, stored, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_attempt_consumer_review(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptConsumerReviewReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        let stored = consumer_review_by_lease_on(&conn, lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无消费者终态审核证据"))?;
        consumer_review_receipt_on(&*conn, stored, false)
    }
}

fn consumer_review_receipt_on(
    conn: &rusqlite::Connection,
    stored: StoredConsumerReview,
    replayed: bool,
) -> Result<ComputeAttemptConsumerReviewReceipt> {
    let candidate = compute_attempt_terminal_candidate_on(conn, &stored.lease_id)?
        .ok_or_else(|| anyhow!("消费者终态审核引用的 Provider 候选不存在"))?;
    ensure_candidate_binding(&stored, &candidate)?;
    stored.into_receipt(replayed)
}
