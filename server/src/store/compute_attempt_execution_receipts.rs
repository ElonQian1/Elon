use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use rusqlite::{params, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::receipts::ComputeExecutionReceipt;

use super::{
    compute_attempt_activations::compute_attempt_activation_on,
    compute_attempt_consumer_reviews::compute_attempt_consumer_review_on,
    compute_attempt_platform_observations::compute_attempt_platform_observation_on,
    compute_attempt_terminals::compute_attempt_terminal_candidate_on,
    compute_attempt_usage::compute_attempt_usage_declaration_on,
    compute_attempt_verifications::compute_attempt_verification_decision_on,
    compute_job_registry::registered_job_version_on,
    compute_reservation_registry::registered_reservation_version_on, new_id, Store,
};

mod support;

use support::{
    build_execution_receipt, ensure_expected_verification, ensure_receipt_sources,
    execution_receipt_by_idempotency_on, execution_receipt_by_lease_on,
    execution_receipt_by_verification_on, execution_receipt_request_digest,
    normalize_execution_receipt_request, StoredExecutionReceipt,
};

#[derive(Debug, Clone)]
pub(crate) struct IssueComputeAttemptExecutionReceiptRequest {
    pub lease_id: String,
    pub expected_verification_decision_id: String,
    pub expected_verification_event_digest: String,
    pub idempotency_key: String,
    pub issued_by_user_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptExecutionReceiptEnvelope {
    pub receipt: ComputeExecutionReceipt,
    pub verification_decision_id: String,
    pub verification_event_digest: String,
    pub request_digest: String,
    pub issued_by_user_id: String,
    pub issued_at: String,
    pub execution_effect: &'static str,
    pub lease_effect: &'static str,
    pub job_effect: &'static str,
    pub capacity_effect: &'static str,
    pub reservation_effect: &'static str,
    pub money_effect: &'static str,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn issue_compute_attempt_execution_receipt(
        &self,
        input: &IssueComputeAttemptExecutionReceiptRequest,
    ) -> Result<ComputeAttemptExecutionReceiptEnvelope> {
        let input = normalize_execution_receipt_request(input)?;
        let request_digest = execution_receipt_request_digest(&input)?;
        let idempotency_scope = format!(
            "compute_attempt_execution_receipt:{}",
            input.issued_by_user_id
        );
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            execution_receipt_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同 Execution Receipt 幂等键不能用于不同请求");
            }
            let envelope = execution_receipt_envelope_on(&tx, stored, true)?;
            tx.commit()?;
            return Ok(envelope);
        }

        let verification = compute_attempt_verification_decision_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无 Verification 决定"))?;
        ensure_expected_verification(&input, &verification)?;
        if let Some(stored) =
            execution_receipt_by_verification_on(&tx, &verification.verification_decision_id)?
        {
            if stored.request_digest != request_digest {
                bail!("同一 Verification 决定已绑定另一份 Execution Receipt");
            }
            let envelope = execution_receipt_envelope_on(&tx, stored, true)?;
            tx.commit()?;
            return Ok(envelope);
        }

        let candidate = compute_attempt_terminal_candidate_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Execution Receipt 引用的 Provider 候选不存在"))?;
        let consumer_review = compute_attempt_consumer_review_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Execution Receipt 引用的消费者审核不存在"))?;
        let platform_observation =
            compute_attempt_platform_observation_on(&tx, &input.lease_id)?
                .ok_or_else(|| anyhow!("Execution Receipt 引用的平台观测不存在"))?;
        let provider_usage = compute_attempt_usage_declaration_on(
            &tx,
            &input.lease_id,
            candidate.final_usage_sequence_no,
        )?
        .ok_or_else(|| anyhow!("Execution Receipt 引用的 Provider 用量不存在"))?;
        let activation = compute_attempt_activation_on(&tx, &input.lease_id)?;
        let job = registered_job_version_on(&tx, &candidate.job_id, candidate.job_revision)?
            .ok_or_else(|| anyhow!("Execution Receipt 引用的 Job 历史版本不存在"))?;
        let reservation = registered_reservation_version_on(
            &tx,
            &candidate.reservation_id,
            candidate.reservation_revision,
        )?
        .ok_or_else(|| anyhow!("Execution Receipt 引用的 Reservation 历史版本不存在"))?;
        ensure_receipt_sources(
            &verification,
            &candidate,
            &consumer_review,
            &platform_observation,
            &provider_usage,
            &activation,
            &job,
            &reservation,
        )?;

        let issued_at = Utc::now().to_rfc3339();
        let execution_receipt_id = new_id("compute_execution_receipt");
        let receipt = build_execution_receipt(
            &execution_receipt_id,
            &verification,
            &candidate,
            &consumer_review,
            &platform_observation,
            &provider_usage,
            &activation,
            &job.job,
            &reservation.reservation,
            &issued_at,
        )?;
        tx.execute(
            "INSERT INTO compute_attempt_execution_receipts (
                execution_receipt_id, verification_decision_id,
                verification_event_digest, lease_id, receipt_digest,
                receipt_json, request_digest, idempotency_scope,
                idempotency_key, issued_by_user_id, issued_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![
                receipt.receipt_id,
                verification.verification_decision_id,
                verification.event_digest,
                input.lease_id,
                receipt.receipt_digest,
                serde_json::to_string(&receipt)?,
                request_digest,
                idempotency_scope,
                input.idempotency_key,
                input.issued_by_user_id,
                issued_at,
            ],
        )?;
        let stored =
            execution_receipt_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
                .ok_or_else(|| anyhow!("Execution Receipt 写入后不可见"))?;
        let envelope = execution_receipt_envelope_on(&tx, stored, false)?;
        tx.commit()?;
        Ok(envelope)
    }

    pub(crate) fn compute_attempt_execution_receipt(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptExecutionReceiptEnvelope> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        let stored = execution_receipt_by_lease_on(&*conn, lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无 Execution Receipt"))?;
        execution_receipt_envelope_on(&*conn, stored, false)
    }
}

pub(super) fn compute_attempt_execution_receipt_on(
    conn: &rusqlite::Connection,
    lease_id: &str,
) -> Result<ComputeAttemptExecutionReceiptEnvelope> {
    support::validate_exact("Attempt Lease ID", lease_id, 200)?;
    let stored = execution_receipt_by_lease_on(conn, lease_id)?
        .ok_or_else(|| anyhow!("Attempt 尚无 Execution Receipt"))?;
    execution_receipt_envelope_on(conn, stored, false)
}

fn execution_receipt_envelope_on(
    conn: &rusqlite::Connection,
    stored: StoredExecutionReceipt,
    replayed: bool,
) -> Result<ComputeAttemptExecutionReceiptEnvelope> {
    let verification = compute_attempt_verification_decision_on(conn, &stored.lease_id)?
        .ok_or_else(|| anyhow!("Execution Receipt 引用的 Verification 决定不存在"))?;
    let candidate = compute_attempt_terminal_candidate_on(conn, &stored.lease_id)?
        .ok_or_else(|| anyhow!("Execution Receipt 引用的 Provider 候选不存在"))?;
    let consumer_review = compute_attempt_consumer_review_on(conn, &stored.lease_id)?
        .ok_or_else(|| anyhow!("Execution Receipt 引用的消费者审核不存在"))?;
    let platform_observation = compute_attempt_platform_observation_on(conn, &stored.lease_id)?
        .ok_or_else(|| anyhow!("Execution Receipt 引用的平台观测不存在"))?;
    let provider_usage = compute_attempt_usage_declaration_on(
        conn,
        &stored.lease_id,
        candidate.final_usage_sequence_no,
    )?
    .ok_or_else(|| anyhow!("Execution Receipt 引用的 Provider 用量不存在"))?;
    let activation = compute_attempt_activation_on(conn, &stored.lease_id)?;
    let job = registered_job_version_on(conn, &candidate.job_id, candidate.job_revision)?
        .ok_or_else(|| anyhow!("Execution Receipt 引用的 Job 版本不存在"))?;
    let reservation = registered_reservation_version_on(
        conn,
        &candidate.reservation_id,
        candidate.reservation_revision,
    )?
    .ok_or_else(|| anyhow!("Execution Receipt 引用的 Reservation 版本不存在"))?;
    ensure_receipt_sources(
        &verification,
        &candidate,
        &consumer_review,
        &platform_observation,
        &provider_usage,
        &activation,
        &job,
        &reservation,
    )?;
    stored.into_envelope(
        &verification,
        &candidate,
        &consumer_review,
        &platform_observation,
        &provider_usage,
        &activation,
        &job.job,
        &reservation.reservation,
        replayed,
    )
}
