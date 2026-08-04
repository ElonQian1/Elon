use std::collections::BTreeSet;

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    ComputeAttemptConsumerReviewReceipt, ReviewComputeAttemptTerminalCandidateRequest,
    COMPUTE_ATTEMPT_CONSUMER_REVIEW_SCHEMA, CONSUMER_REVIEW_ACCEPTED, CONSUMER_REVIEW_DISPUTED,
    CONSUMER_REVIEW_REJECTED,
};
use crate::store::ComputeAttemptTerminalCandidateReceipt;

const MAX_EVIDENCE_REFS: usize = 16;

#[derive(Debug, Clone)]
pub(super) struct StoredConsumerReview {
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
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub reviewed_by_user_id: String,
    pub reviewed_at: String,
    pub created_at: String,
}

impl StoredConsumerReview {
    pub(super) fn into_receipt(
        self,
        replayed: bool,
    ) -> Result<ComputeAttemptConsumerReviewReceipt> {
        audit_consumer_review(&self)?;
        Ok(ComputeAttemptConsumerReviewReceipt {
            schema: COMPUTE_ATTEMPT_CONSUMER_REVIEW_SCHEMA,
            consumer_review_id: self.consumer_review_id,
            terminal_candidate_id: self.terminal_candidate_id,
            terminal_candidate_event_digest: self.terminal_candidate_event_digest,
            lease_id: self.lease_id,
            provider_id: self.provider_id,
            consumer_account_id: self.consumer_account_id,
            source_lease_revision: self.source_lease_revision,
            source_lease_digest: self.source_lease_digest,
            fencing_generation: self.fencing_generation,
            job_id: self.job_id,
            job_revision: self.job_revision,
            job_digest: self.job_digest,
            reservation_id: self.reservation_id,
            reservation_revision: self.reservation_revision,
            reservation_digest: self.reservation_digest,
            capacity_claim_id: self.capacity_claim_id,
            capacity_claim_revision: self.capacity_claim_revision,
            capacity_claim_digest: self.capacity_claim_digest,
            final_usage_snapshot_id: self.final_usage_snapshot_id,
            final_usage_sequence_no: self.final_usage_sequence_no,
            final_cumulative_usage_digest: self.final_cumulative_usage_digest,
            candidate_outcome: self.candidate_outcome,
            decision: self.decision,
            reason_code: self.reason_code,
            consumer_review_ref: self.consumer_review_ref,
            evidence_refs: self.evidence_refs,
            evidence_refs_digest: self.evidence_refs_digest,
            request_digest: self.request_digest,
            event_digest: self.event_digest,
            reviewed_by_user_id: self.reviewed_by_user_id,
            reviewed_at: self.reviewed_at,
            evidence_status: "consumer_attestation_only",
            review_effect: "consumer_evidence_recorded",
            verification_effect: "none",
            lease_effect: "unchanged",
            job_effect: "unchanged",
            capacity_effect: "unchanged",
            reservation_effect: "unchanged",
            money_effect: "preauthorization_unchanged",
            replayed,
        })
    }
}

pub(super) fn normalize_consumer_review_request(
    input: &ReviewComputeAttemptTerminalCandidateRequest,
) -> Result<ReviewComputeAttemptTerminalCandidateRequest> {
    for (label, value, max_len) in [
        ("Attempt Lease ID", input.lease_id.as_str(), 200),
        (
            "终态候选 ID",
            input.expected_terminal_candidate_id.as_str(),
            200,
        ),
        (
            "终态候选事件摘要",
            input.expected_terminal_candidate_event_digest.as_str(),
            64,
        ),
        ("审核决定", input.decision.as_str(), 40),
        ("审核原因码", input.reason_code.as_str(), 100),
        ("消费者审核引用", input.consumer_review_ref.as_str(), 1000),
        ("幂等键", input.idempotency_key.as_str(), 200),
        ("审核用户 ID", input.reviewed_by_user_id.as_str(), 200),
    ] {
        validate_exact(label, value, max_len)?;
    }
    validate_digest(
        "终态候选事件摘要",
        &input.expected_terminal_candidate_event_digest,
    )?;
    if !matches!(
        input.decision.as_str(),
        CONSUMER_REVIEW_ACCEPTED | CONSUMER_REVIEW_REJECTED | CONSUMER_REVIEW_DISPUTED
    ) {
        bail!("消费者审核 decision 只允许 accepted、rejected 或 disputed");
    }
    if input.reason_code.bytes().any(|byte| {
        !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-'))
    }) {
        bail!("消费者审核原因码只允许小写字母、数字、点、下划线和连字符");
    }
    if input.evidence_refs.len() > MAX_EVIDENCE_REFS {
        bail!("消费者审核证据引用数量超过上限");
    }

    let mut normalized = input.clone();
    normalized.evidence_refs.sort();
    let mut unique = BTreeSet::new();
    for evidence_ref in &normalized.evidence_refs {
        validate_exact("消费者审核证据引用", evidence_ref, 1000)?;
        if !unique.insert(evidence_ref.as_str()) {
            bail!("消费者审核证据引用重复");
        }
    }
    if matches!(
        normalized.decision.as_str(),
        CONSUMER_REVIEW_REJECTED | CONSUMER_REVIEW_DISPUTED
    ) && normalized.evidence_refs.is_empty()
    {
        bail!("rejected 或 disputed 消费者审核必须提供至少一个证据引用");
    }
    Ok(normalized)
}

pub(super) fn consumer_review_request_digest(
    input: &ReviewComputeAttemptTerminalCandidateRequest,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_consumer_review_request",
        "lease_id":input.lease_id,
        "expected_terminal_candidate_id":input.expected_terminal_candidate_id,
        "expected_terminal_candidate_event_digest":input.expected_terminal_candidate_event_digest,
        "decision":input.decision,
        "reason_code":input.reason_code,
        "consumer_review_ref":input.consumer_review_ref,
        "evidence_refs":input.evidence_refs,
        "idempotency_key":input.idempotency_key,
        "reviewed_by_user_id":input.reviewed_by_user_id,
    }))
}

pub(super) fn evidence_refs_digest(evidence_refs: &[String]) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_consumer_review_evidence_refs",
        "evidence_refs":evidence_refs,
    }))
}

pub(super) fn consumer_review_event_digest(
    consumer_review_id: &str,
    input: &ReviewComputeAttemptTerminalCandidateRequest,
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    evidence_refs_digest: &str,
    request_digest: &str,
    reviewed_at: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema":COMPUTE_ATTEMPT_CONSUMER_REVIEW_SCHEMA,
        "consumer_review_id":consumer_review_id,
        "terminal_candidate_id":candidate.terminal_candidate_id,
        "terminal_candidate_event_digest":candidate.event_digest,
        "lease_id":candidate.lease_id,
        "provider_id":candidate.provider_id,
        "consumer_account_id":candidate.consumer_account_id,
        "source_lease_revision":candidate.source_lease_revision,
        "source_lease_digest":candidate.source_lease_digest,
        "fencing_generation":candidate.fencing_generation,
        "job_id":candidate.job_id,
        "job_revision":candidate.job_revision,
        "job_digest":candidate.job_digest,
        "reservation_id":candidate.reservation_id,
        "reservation_revision":candidate.reservation_revision,
        "reservation_digest":candidate.reservation_digest,
        "capacity_claim_id":candidate.capacity_claim_id,
        "capacity_claim_revision":candidate.capacity_claim_revision,
        "capacity_claim_digest":candidate.capacity_claim_digest,
        "final_usage_snapshot_id":candidate.final_usage_snapshot_id,
        "final_usage_sequence_no":candidate.final_usage_sequence_no,
        "final_cumulative_usage_digest":candidate.final_cumulative_usage_digest,
        "candidate_outcome":candidate.outcome,
        "decision":input.decision,
        "reason_code":input.reason_code,
        "consumer_review_ref":input.consumer_review_ref,
        "evidence_refs_digest":evidence_refs_digest,
        "request_digest":request_digest,
        "reviewed_by_user_id":input.reviewed_by_user_id,
        "reviewed_at":reviewed_at,
    }))
}

pub(super) fn ensure_candidate_binding(
    stored: &StoredConsumerReview,
    candidate: &ComputeAttemptTerminalCandidateReceipt,
) -> Result<()> {
    if stored.terminal_candidate_id != candidate.terminal_candidate_id
        || stored.terminal_candidate_event_digest != candidate.event_digest
        || stored.lease_id != candidate.lease_id
        || stored.provider_id != candidate.provider_id
        || stored.consumer_account_id != candidate.consumer_account_id
        || stored.source_lease_revision != candidate.source_lease_revision
        || stored.source_lease_digest != candidate.source_lease_digest
        || stored.fencing_generation != candidate.fencing_generation
        || stored.job_id != candidate.job_id
        || stored.job_revision != candidate.job_revision
        || stored.job_digest != candidate.job_digest
        || stored.reservation_id != candidate.reservation_id
        || stored.reservation_revision != candidate.reservation_revision
        || stored.reservation_digest != candidate.reservation_digest
        || stored.capacity_claim_id != candidate.capacity_claim_id
        || stored.capacity_claim_revision != candidate.capacity_claim_revision
        || stored.capacity_claim_digest != candidate.capacity_claim_digest
        || stored.final_usage_snapshot_id != candidate.final_usage_snapshot_id
        || stored.final_usage_sequence_no != candidate.final_usage_sequence_no
        || stored.final_cumulative_usage_digest != candidate.final_cumulative_usage_digest
        || stored.candidate_outcome != candidate.outcome
    {
        bail!("消费者终态审核与 Provider 终态候选绑定审计失败");
    }
    Ok(())
}

fn audit_consumer_review(stored: &StoredConsumerReview) -> Result<()> {
    if stored.created_at != stored.reviewed_at
        || stored.reviewed_by_user_id != stored.consumer_account_id
        || stored.evidence_refs_digest != evidence_refs_digest(&stored.evidence_refs)?
    {
        bail!("消费者终态审核基础字段审计失败");
    }
    let request =
        normalize_consumer_review_request(&ReviewComputeAttemptTerminalCandidateRequest {
            lease_id: stored.lease_id.clone(),
            expected_terminal_candidate_id: stored.terminal_candidate_id.clone(),
            expected_terminal_candidate_event_digest: stored
                .terminal_candidate_event_digest
                .clone(),
            decision: stored.decision.clone(),
            reason_code: stored.reason_code.clone(),
            consumer_review_ref: stored.consumer_review_ref.clone(),
            evidence_refs: stored.evidence_refs.clone(),
            idempotency_key: stored.idempotency_key.clone(),
            reviewed_by_user_id: stored.reviewed_by_user_id.clone(),
        })?;
    if stored.idempotency_scope
        != format!(
            "compute_attempt_consumer_review:{}",
            stored.reviewed_by_user_id
        )
        || stored.request_digest != consumer_review_request_digest(&request)?
    {
        bail!("消费者终态审核请求审计失败");
    }
    let event_digest = digest_json(&serde_json::json!({
        "schema":COMPUTE_ATTEMPT_CONSUMER_REVIEW_SCHEMA,
        "consumer_review_id":stored.consumer_review_id,
        "terminal_candidate_id":stored.terminal_candidate_id,
        "terminal_candidate_event_digest":stored.terminal_candidate_event_digest,
        "lease_id":stored.lease_id,
        "provider_id":stored.provider_id,
        "consumer_account_id":stored.consumer_account_id,
        "source_lease_revision":stored.source_lease_revision,
        "source_lease_digest":stored.source_lease_digest,
        "fencing_generation":stored.fencing_generation,
        "job_id":stored.job_id,
        "job_revision":stored.job_revision,
        "job_digest":stored.job_digest,
        "reservation_id":stored.reservation_id,
        "reservation_revision":stored.reservation_revision,
        "reservation_digest":stored.reservation_digest,
        "capacity_claim_id":stored.capacity_claim_id,
        "capacity_claim_revision":stored.capacity_claim_revision,
        "capacity_claim_digest":stored.capacity_claim_digest,
        "final_usage_snapshot_id":stored.final_usage_snapshot_id,
        "final_usage_sequence_no":stored.final_usage_sequence_no,
        "final_cumulative_usage_digest":stored.final_cumulative_usage_digest,
        "candidate_outcome":stored.candidate_outcome,
        "decision":stored.decision,
        "reason_code":stored.reason_code,
        "consumer_review_ref":stored.consumer_review_ref,
        "evidence_refs_digest":stored.evidence_refs_digest,
        "request_digest":stored.request_digest,
        "reviewed_by_user_id":stored.reviewed_by_user_id,
        "reviewed_at":stored.reviewed_at,
    }))?;
    if stored.event_digest != event_digest {
        bail!("消费者终态审核事件摘要审计失败");
    }
    Ok(())
}

pub(super) fn consumer_review_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredConsumerReview>> {
    query_one(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn consumer_review_by_candidate_on(
    conn: &Connection,
    terminal_candidate_id: &str,
) -> Result<Option<StoredConsumerReview>> {
    query_one(
        conn,
        "WHERE terminal_candidate_id=?1",
        params![terminal_candidate_id],
    )
}

pub(super) fn consumer_review_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredConsumerReview>> {
    query_one(conn, "WHERE lease_id=?1", params![lease_id])
}

fn query_one<P: rusqlite::Params>(
    conn: &Connection,
    clause: &str,
    params: P,
) -> Result<Option<StoredConsumerReview>> {
    conn.query_row(
        &format!("{} {clause}", select_sql()),
        params,
        stored_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn select_sql() -> &'static str {
    "SELECT consumer_review_id, terminal_candidate_id,
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
       FROM compute_attempt_consumer_reviews"
}

fn stored_from_row(row: &Row<'_>) -> rusqlite::Result<StoredConsumerReview> {
    Ok(StoredConsumerReview {
        consumer_review_id: row.get(0)?,
        terminal_candidate_id: row.get(1)?,
        terminal_candidate_event_digest: row.get(2)?,
        lease_id: row.get(3)?,
        provider_id: row.get(4)?,
        consumer_account_id: row.get(5)?,
        source_lease_revision: row.get(6)?,
        source_lease_digest: row.get(7)?,
        fencing_generation: row.get(8)?,
        job_id: row.get(9)?,
        job_revision: row.get(10)?,
        job_digest: row.get(11)?,
        reservation_id: row.get(12)?,
        reservation_revision: row.get(13)?,
        reservation_digest: row.get(14)?,
        capacity_claim_id: row.get(15)?,
        capacity_claim_revision: row.get(16)?,
        capacity_claim_digest: row.get(17)?,
        final_usage_snapshot_id: row.get(18)?,
        final_usage_sequence_no: row.get(19)?,
        final_cumulative_usage_digest: row.get(20)?,
        candidate_outcome: row.get(21)?,
        decision: row.get(22)?,
        reason_code: row.get(23)?,
        consumer_review_ref: row.get(24)?,
        evidence_refs: parse_json(row, 25)?,
        evidence_refs_digest: row.get(26)?,
        request_digest: row.get(27)?,
        event_digest: row.get(28)?,
        idempotency_scope: row.get(29)?,
        idempotency_key: row.get(30)?,
        reviewed_by_user_id: row.get(31)?,
        reviewed_at: row.get(32)?,
        created_at: row.get(33)?,
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    let value: String = row.get(index)?;
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

pub(super) fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max_len
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符");
    }
    Ok(())
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || value != value.to_ascii_lowercase()
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("{label}必须是 64 位小写十六进制 SHA-256");
    }
    Ok(())
}

fn digest_json(value: &impl Serialize) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value)?)))
}
