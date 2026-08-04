use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use rusqlite::{params, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::receipts::ComputeMeterReading;

use super::{
    compute_attempt_consumer_reviews::{
        compute_attempt_consumer_review_on, ComputeAttemptConsumerReviewReceipt,
    },
    compute_attempt_platform_observations::{
        compute_attempt_platform_observation_on, ComputeAttemptPlatformObservationReceipt,
    },
    compute_attempt_terminals::{
        compute_attempt_terminal_candidate_on, ComputeAttemptTerminalCandidateReceipt,
    },
    compute_attempt_usage::{
        compute_attempt_usage_declaration_on, ComputeAttemptUsageDeclarationReceipt,
    },
    compute_reservation_registry::registered_reservation_version_on,
    new_id, Store,
};

mod pending_queue;
mod support;

use pending_queue::list_pending_verification_lease_ids_on;

use support::{
    build_policy_usage, ensure_evidence_binding, ensure_expected_binding, ensure_policy_decision,
    normalize_verification_request, reason_codes_digest, verification_decision_by_candidate_on,
    verification_decision_by_idempotency_on, verification_decision_by_lease_on,
    verification_event_digest, verification_request_digest, verification_usage_digest,
    StoredVerificationDecision,
};

pub(crate) const COMPUTE_ATTEMPT_VERIFICATION_DECISION_SCHEMA: &str =
    "compute_federation.attempt_verification_decision.v1";
pub(crate) const VERIFICATION_POLICY_CONSERVATIVE_MIN_V1: &str = "conservative_min_v1";
pub(crate) const VERIFICATION_DECISION_ACCEPTED: &str = "accepted";
pub(crate) const VERIFICATION_DECISION_REJECTED: &str = "rejected";
pub(crate) const VERIFICATION_DECISION_DISPUTED: &str = "disputed";

#[derive(Debug, Clone)]
pub(crate) struct DecideComputeAttemptVerificationRequest {
    pub lease_id: String,
    pub expected_terminal_candidate_id: String,
    pub expected_terminal_candidate_event_digest: String,
    pub expected_consumer_review_id: String,
    pub expected_consumer_review_event_digest: String,
    pub expected_platform_observation_id: String,
    pub expected_platform_observation_event_digest: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub decision: String,
    pub reason_codes: Vec<String>,
    pub decision_ref: String,
    pub idempotency_key: String,
    pub decided_by_user_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptVerificationDecisionReceipt {
    pub schema: &'static str,
    pub verification_decision_id: String,
    pub terminal_candidate_id: String,
    pub terminal_candidate_event_digest: String,
    pub consumer_review_id: String,
    pub consumer_review_event_digest: String,
    pub platform_observation_id: String,
    pub platform_observation_event_digest: String,
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
    pub final_provider_usage_digest: String,
    pub platform_observed_usage_digest: String,
    pub candidate_outcome: String,
    pub consumer_decision: String,
    pub observed_outcome: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub decision: String,
    pub reason_codes: Vec<String>,
    pub reason_codes_digest: String,
    pub decision_ref: String,
    pub verified_usage: Vec<ComputeMeterReading>,
    pub verified_usage_digest: String,
    pub compensable_usage: Vec<ComputeMeterReading>,
    pub compensable_usage_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub decided_by_user_id: String,
    pub decided_at: String,
    pub verification_effect: &'static str,
    pub execution_receipt_effect: &'static str,
    pub lease_effect: &'static str,
    pub job_effect: &'static str,
    pub capacity_effect: &'static str,
    pub reservation_effect: &'static str,
    pub money_effect: &'static str,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputePendingAttemptVerificationCandidate {
    pub terminal_candidate: ComputeAttemptTerminalCandidateReceipt,
    pub provider_usage: ComputeAttemptUsageDeclarationReceipt,
    pub consumer_review: ComputeAttemptConsumerReviewReceipt,
    pub platform_observation: ComputeAttemptPlatformObservationReceipt,
}

impl Store {
    pub(crate) fn decide_compute_attempt_verification(
        &self,
        input: &DecideComputeAttemptVerificationRequest,
    ) -> Result<ComputeAttemptVerificationDecisionReceipt> {
        let input = normalize_verification_request(input)?;
        let request_digest = verification_request_digest(&input)?;
        let idempotency_scope =
            format!("compute_attempt_verification:{}", input.decided_by_user_id);
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = verification_decision_by_idempotency_on(
            &tx,
            &idempotency_scope,
            &input.idempotency_key,
        )? {
            if stored.request_digest != request_digest {
                bail!("相同 Verification 幂等键不能用于不同请求");
            }
            let receipt = verification_decision_receipt_on(&tx, stored, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let candidate = compute_attempt_terminal_candidate_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无 Provider 终态候选"))?;
        let consumer_review = compute_attempt_consumer_review_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无消费者终态审核证据"))?;
        let platform_observation = compute_attempt_platform_observation_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无平台终态观测证据"))?;
        let provider_usage = compute_attempt_usage_declaration_on(
            &tx,
            &input.lease_id,
            candidate.final_usage_sequence_no,
        )?
        .ok_or_else(|| anyhow!("终态候选绑定的最终 Provider 用量快照不存在"))?;
        let reservation = registered_reservation_version_on(
            &tx,
            &candidate.reservation_id,
            candidate.reservation_revision,
        )?
        .ok_or_else(|| anyhow!("终态候选绑定的 Reservation 历史版本不存在"))?;

        ensure_expected_binding(&input, &candidate, &consumer_review, &platform_observation)?;
        ensure_evidence_binding(
            &candidate,
            &consumer_review,
            &platform_observation,
            &provider_usage,
            &reservation,
        )?;
        ensure_policy_decision(&input, &candidate, &consumer_review, &platform_observation)?;

        if let Some(stored) =
            verification_decision_by_candidate_on(&tx, &candidate.terminal_candidate_id)?
        {
            if stored.request_digest != request_digest {
                bail!("同一 Provider 终态候选已绑定另一份 Verification 决定");
            }
            let receipt = verification_decision_receipt_on(&tx, stored, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let decided_at = Utc::now().to_rfc3339();
        let (verified_usage, compensable_usage) = build_policy_usage(
            &input,
            &provider_usage,
            &platform_observation,
            &reservation.reservation,
            &decided_at,
        )?;
        let verified_usage_digest = verification_usage_digest("verified", &verified_usage)?;
        let compensable_usage_digest =
            verification_usage_digest("compensable", &compensable_usage)?;
        let reason_codes_digest = reason_codes_digest(&input.reason_codes)?;
        let verification_decision_id = new_id("compute_attempt_verification");
        let event_digest = verification_event_digest(
            &verification_decision_id,
            &input,
            &candidate,
            &consumer_review,
            &platform_observation,
            &reason_codes_digest,
            &verified_usage_digest,
            &compensable_usage_digest,
            &request_digest,
            &decided_at,
        )?;

        tx.execute(
            "INSERT INTO compute_attempt_verification_decisions (
                verification_decision_id, terminal_candidate_id,
                terminal_candidate_event_digest, consumer_review_id,
                consumer_review_event_digest, platform_observation_id,
                platform_observation_event_digest, lease_id, policy_id,
                policy_version, decision, reason_codes_json,
                reason_codes_digest, decision_ref, verified_usage_json,
                verified_usage_digest, compensable_usage_json,
                compensable_usage_digest, request_digest, event_digest,
                idempotency_scope, idempotency_key, decided_by_user_id,
                decided_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                       ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21,
                       ?22, ?23, ?24, ?24)",
            params![
                verification_decision_id,
                candidate.terminal_candidate_id,
                candidate.event_digest,
                consumer_review.consumer_review_id,
                consumer_review.event_digest,
                platform_observation.platform_observation_id,
                platform_observation.event_digest,
                candidate.lease_id,
                input.policy_id,
                input.policy_version,
                input.decision,
                serde_json::to_string(&input.reason_codes)?,
                reason_codes_digest,
                input.decision_ref,
                serde_json::to_string(&verified_usage)?,
                verified_usage_digest,
                serde_json::to_string(&compensable_usage)?,
                compensable_usage_digest,
                request_digest,
                event_digest,
                idempotency_scope,
                input.idempotency_key,
                input.decided_by_user_id,
                decided_at,
            ],
        )?;

        let stored = verification_decision_by_idempotency_on(
            &tx,
            &idempotency_scope,
            &input.idempotency_key,
        )?
        .ok_or_else(|| anyhow!("Verification 决定写入后不可见"))?;
        let receipt = verification_decision_receipt_on(&tx, stored, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_attempt_verification_decision(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptVerificationDecisionReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        compute_attempt_verification_decision_on(&*conn, lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无 Verification 决定"))
    }

    pub(crate) fn list_pending_compute_attempt_verifications(
        &self,
        limit: usize,
    ) -> Result<Vec<ComputePendingAttemptVerificationCandidate>> {
        let conn = self.conn()?;
        list_pending_verification_lease_ids_on(&conn, limit.clamp(1, 100))?
            .into_iter()
            .map(|lease_id| {
                let candidate = compute_attempt_terminal_candidate_on(&conn, &lease_id)?
                    .ok_or_else(|| anyhow!("待验证队列引用的 Provider 候选不存在"))?;
                let consumer_review = compute_attempt_consumer_review_on(&conn, &lease_id)?
                    .ok_or_else(|| anyhow!("待验证队列引用的消费者审核不存在"))?;
                let platform_observation =
                    compute_attempt_platform_observation_on(&conn, &lease_id)?
                        .ok_or_else(|| anyhow!("待验证队列引用的平台观测不存在"))?;
                let provider_usage = compute_attempt_usage_declaration_on(
                    &conn,
                    &lease_id,
                    candidate.final_usage_sequence_no,
                )?
                .ok_or_else(|| anyhow!("待验证队列引用的 Provider 用量不存在"))?;
                let reservation = registered_reservation_version_on(
                    &conn,
                    &candidate.reservation_id,
                    candidate.reservation_revision,
                )?
                .ok_or_else(|| anyhow!("待验证队列引用的 Reservation 版本不存在"))?;
                ensure_evidence_binding(
                    &candidate,
                    &consumer_review,
                    &platform_observation,
                    &provider_usage,
                    &reservation,
                )?;
                Ok(ComputePendingAttemptVerificationCandidate {
                    terminal_candidate: candidate,
                    provider_usage,
                    consumer_review,
                    platform_observation,
                })
            })
            .collect()
    }
}

pub(crate) fn compute_attempt_verification_decision_on(
    conn: &rusqlite::Connection,
    lease_id: &str,
) -> Result<Option<ComputeAttemptVerificationDecisionReceipt>> {
    let Some(stored) = verification_decision_by_lease_on(conn, lease_id)? else {
        return Ok(None);
    };
    Ok(Some(verification_decision_receipt_on(conn, stored, false)?))
}

fn verification_decision_receipt_on(
    conn: &rusqlite::Connection,
    stored: StoredVerificationDecision,
    replayed: bool,
) -> Result<ComputeAttemptVerificationDecisionReceipt> {
    let candidate = compute_attempt_terminal_candidate_on(conn, &stored.lease_id)?
        .ok_or_else(|| anyhow!("Verification 决定引用的 Provider 候选不存在"))?;
    let consumer_review = compute_attempt_consumer_review_on(conn, &stored.lease_id)?
        .ok_or_else(|| anyhow!("Verification 决定引用的消费者审核不存在"))?;
    let platform_observation = compute_attempt_platform_observation_on(conn, &stored.lease_id)?
        .ok_or_else(|| anyhow!("Verification 决定引用的平台观测不存在"))?;
    let provider_usage = compute_attempt_usage_declaration_on(
        conn,
        &stored.lease_id,
        candidate.final_usage_sequence_no,
    )?
    .ok_or_else(|| anyhow!("Verification 决定引用的 Provider 用量不存在"))?;
    let reservation = registered_reservation_version_on(
        conn,
        &candidate.reservation_id,
        candidate.reservation_revision,
    )?
    .ok_or_else(|| anyhow!("Verification 决定引用的 Reservation 版本不存在"))?;
    ensure_evidence_binding(
        &candidate,
        &consumer_review,
        &platform_observation,
        &provider_usage,
        &reservation,
    )?;
    stored.into_receipt(
        &candidate,
        &consumer_review,
        &platform_observation,
        &provider_usage,
        &reservation.reservation,
        replayed,
    )
}
