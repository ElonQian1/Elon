use std::collections::BTreeSet;

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::workload::{ComputeArtifactRef, ComputeOutputContract};

use super::{
    ComputeAttemptTerminalCandidateReceipt, ComputeDeclaredResultArtifactInput,
    DeclareComputeAttemptTerminalCandidateRequest, COMPUTE_ATTEMPT_TERMINAL_CANDIDATE_SCHEMA,
    TERMINAL_OUTCOME_CANCELED, TERMINAL_OUTCOME_FAILED, TERMINAL_OUTCOME_SUCCEEDED,
};

mod audit;
mod consumer_queue;

pub(super) use consumer_queue::list_pending_consumer_review_candidates_on;

const COMPUTE_JSON_SAFE_SEQUENCE_MAX: i64 = 9_007_199_254_740_991;
const MAX_RESULT_ARTIFACTS: usize = 32;

#[derive(Debug, Clone)]
pub(super) struct StoredTerminalCandidate {
    pub terminal_candidate_id: String,
    pub lease_id: String,
    pub provider_id: String,
    pub consumer_account_id: String,
    pub source_lease_revision: i64,
    pub source_lease_digest: String,
    pub source_lease_status: String,
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
    pub executor_terminal_ref: String,
    pub outcome: String,
    pub reason_code: String,
    pub diagnostic_ref: Option<String>,
    pub output_digest: Option<String>,
    pub result_artifacts: Vec<ComputeArtifactRef>,
    pub result_artifacts_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub declared_by_user_id: String,
    pub declared_at: String,
    pub created_at: String,
}

impl StoredTerminalCandidate {
    pub(super) fn into_receipt(
        self,
        replayed: bool,
    ) -> Result<ComputeAttemptTerminalCandidateReceipt> {
        audit::audit_candidate(&self)?;
        Ok(ComputeAttemptTerminalCandidateReceipt {
            schema: COMPUTE_ATTEMPT_TERMINAL_CANDIDATE_SCHEMA,
            terminal_candidate_id: self.terminal_candidate_id,
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
            executor_terminal_ref: self.executor_terminal_ref,
            outcome: self.outcome,
            reason_code: self.reason_code,
            diagnostic_ref: self.diagnostic_ref,
            output_digest: self.output_digest,
            result_artifacts: self.result_artifacts,
            result_artifacts_digest: self.result_artifacts_digest,
            request_digest: self.request_digest,
            event_digest: self.event_digest,
            declared_by_user_id: self.declared_by_user_id,
            declared_at: self.declared_at,
            verification_status: "unverified_provider_declaration",
            execution_effect: "candidate_only",
            lease_effect: "unchanged",
            job_effect: "unchanged",
            capacity_effect: "unchanged",
            reservation_effect: "unchanged",
            money_effect: "preauthorization_unchanged",
            replayed,
        })
    }
}

pub(super) fn normalize_terminal_request(
    input: &DeclareComputeAttemptTerminalCandidateRequest,
) -> Result<DeclareComputeAttemptTerminalCandidateRequest> {
    for (label, value, max_len) in [
        ("Attempt Lease ID", input.lease_id.as_str(), 200),
        ("Provider ID", input.provider_id.as_str(), 200),
        ("Lease 摘要", input.expected_lease_digest.as_str(), 64),
        (
            "最终用量快照 ID",
            input.final_usage_snapshot_id.as_str(),
            200,
        ),
        (
            "最终累计用量摘要",
            input.final_cumulative_usage_digest.as_str(),
            64,
        ),
        ("外部终态引用", input.executor_terminal_ref.as_str(), 1000),
        ("终态结果", input.outcome.as_str(), 40),
        ("终态原因码", input.reason_code.as_str(), 100),
        ("幂等键", input.idempotency_key.as_str(), 200),
        ("声明用户 ID", input.declared_by_user_id.as_str(), 200),
    ] {
        validate_exact(label, value, max_len)?;
    }
    validate_digest("Lease 摘要", &input.expected_lease_digest)?;
    validate_digest("最终累计用量摘要", &input.final_cumulative_usage_digest)?;
    if let Some(value) = input.diagnostic_ref.as_deref() {
        validate_exact("诊断信息引用", value, 1000)?;
    }
    if let Some(value) = input.output_digest.as_deref() {
        validate_digest("输出摘要", value)?;
    }
    if input.expected_lease_revision <= 0
        || input.expected_fencing_generation <= 0
        || input.final_usage_sequence_no <= 0
        || input.final_usage_sequence_no > COMPUTE_JSON_SAFE_SEQUENCE_MAX
    {
        bail!("Attempt 终态候选版本、fencing 或最终用量序号无效");
    }
    if !matches!(
        input.outcome.as_str(),
        TERMINAL_OUTCOME_SUCCEEDED | TERMINAL_OUTCOME_FAILED | TERMINAL_OUTCOME_CANCELED
    ) {
        bail!("Attempt 终态候选 outcome 只允许 succeeded、failed 或 canceled");
    }
    if input.reason_code.bytes().any(|byte| {
        !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-'))
    }) {
        bail!("Attempt 终态原因码只允许小写字母、数字、点、下划线和连字符");
    }
    if input.result_artifacts.len() > MAX_RESULT_ARTIFACTS {
        bail!("Attempt 终态候选结果工件数量超过上限");
    }

    let mut normalized = input.clone();
    normalized
        .result_artifacts
        .sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
    let mut ids = BTreeSet::new();
    for artifact in &normalized.result_artifacts {
        validate_artifact(artifact)?;
        if !ids.insert(artifact.artifact_id.as_str()) {
            bail!("Attempt 终态候选结果工件 ID 重复");
        }
    }
    Ok(normalized)
}

fn validate_artifact(artifact: &ComputeDeclaredResultArtifactInput) -> Result<()> {
    for (label, value, max_len) in [
        ("结果工件 ID", artifact.artifact_id.as_str(), 200),
        ("结果工件摘要算法", artifact.digest_algorithm.as_str(), 40),
        ("结果工件摘要", artifact.digest.as_str(), 64),
        ("结果工件媒体类型", artifact.media_type.as_str(), 200),
        ("结果工件位置引用", artifact.location_ref.as_str(), 1000),
    ] {
        validate_exact(label, value, max_len)?;
    }
    if artifact.digest_algorithm != "sha256" {
        bail!("Attempt 终态候选结果工件只接受 sha256 摘要");
    }
    validate_digest("结果工件摘要", &artifact.digest)?;
    if artifact.size_bytes < 0 || artifact.size_bytes > COMPUTE_JSON_SAFE_SEQUENCE_MAX {
        bail!("Attempt 终态候选结果工件大小无效");
    }
    if let Some(value) = artifact.encryption_profile.as_deref() {
        validate_exact("结果工件加密档案", value, 200)?;
    }
    Ok(())
}

pub(super) fn ensure_output_contract(
    input: &DeclareComputeAttemptTerminalCandidateRequest,
    artifacts: &[ComputeArtifactRef],
    output: &ComputeOutputContract,
) -> Result<()> {
    if input.outcome != TERMINAL_OUTCOME_SUCCEEDED {
        if input.output_digest.is_some() || !artifacts.is_empty() {
            bail!("failed 或 canceled 终态候选不能携带最终输出摘要或结果工件");
        }
        return Ok(());
    }
    if output.deterministic_digest_expected && input.output_digest.is_none() {
        bail!("当前输出合同要求 succeeded 终态候选提供确定性输出摘要");
    }
    if output.result_artifact_required && artifacts.is_empty() {
        bail!("当前输出合同要求 succeeded 终态候选提供结果工件");
    }
    let mut total_size = 0i64;
    for artifact in artifacts {
        if artifact.media_type != output.media_type {
            bail!("Attempt 终态候选结果工件媒体类型不符合输出合同");
        }
        total_size = total_size
            .checked_add(artifact.size_bytes)
            .ok_or_else(|| anyhow::anyhow!("Attempt 终态候选结果工件总大小溢出"))?;
    }
    if total_size > output.max_output_bytes {
        bail!("Attempt 终态候选结果工件总大小超过输出合同上限");
    }
    Ok(())
}

pub(super) fn terminal_request_digest(
    input: &DeclareComputeAttemptTerminalCandidateRequest,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_terminal_candidate_request",
        "lease_id":input.lease_id,
        "provider_id":input.provider_id,
        "expected_lease_revision":input.expected_lease_revision,
        "expected_lease_digest":input.expected_lease_digest,
        "expected_fencing_generation":input.expected_fencing_generation,
        "final_usage_snapshot_id":input.final_usage_snapshot_id,
        "final_usage_sequence_no":input.final_usage_sequence_no,
        "final_cumulative_usage_digest":input.final_cumulative_usage_digest,
        "executor_terminal_ref":input.executor_terminal_ref,
        "outcome":input.outcome,
        "reason_code":input.reason_code,
        "diagnostic_ref":input.diagnostic_ref,
        "output_digest":input.output_digest,
        "result_artifacts":input.result_artifacts,
        "idempotency_key":input.idempotency_key,
        "declared_by_user_id":input.declared_by_user_id,
    }))
}

pub(super) fn artifacts_digest(artifacts: &[ComputeArtifactRef]) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_terminal_candidate_artifacts",
        "artifacts":artifacts,
    }))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn terminal_event_digest(
    terminal_candidate_id: &str,
    input: &DeclareComputeAttemptTerminalCandidateRequest,
    current: &crate::store::compute_attempt_leases::StoredLeaseState,
    job: &crate::store::compute_job_registry::ComputeJobRegistrationReceipt,
    reservation: &crate::store::compute_reservation_registry::ComputeReservationRegistrationReceipt,
    claim: &crate::compute_federation::capacity::ComputeCapacityClaim,
    result_artifacts_digest: &str,
    request_digest: &str,
    declared_at: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema":COMPUTE_ATTEMPT_TERMINAL_CANDIDATE_SCHEMA,
        "terminal_candidate_id":terminal_candidate_id,
        "lease_id":input.lease_id,
        "provider_id":input.provider_id,
        "consumer_account_id":current.consumer_account_id,
        "source_lease_revision":current.lease_revision,
        "source_lease_digest":current.lease_digest,
        "fencing_generation":current.lease.fencing_generation,
        "job_id":job.job.job_id,
        "job_revision":job.revision,
        "job_digest":job.job_digest,
        "reservation_id":reservation.reservation.reservation_id,
        "reservation_revision":reservation.revision,
        "reservation_digest":reservation.reservation_digest,
        "capacity_claim_id":claim.claim_id,
        "capacity_claim_revision":claim.revision,
        "capacity_claim_digest":claim.claim_digest,
        "final_usage_snapshot_id":input.final_usage_snapshot_id,
        "final_usage_sequence_no":input.final_usage_sequence_no,
        "final_cumulative_usage_digest":input.final_cumulative_usage_digest,
        "executor_terminal_ref":input.executor_terminal_ref,
        "outcome":input.outcome,
        "reason_code":input.reason_code,
        "diagnostic_ref":input.diagnostic_ref,
        "output_digest":input.output_digest,
        "result_artifacts_digest":result_artifacts_digest,
        "request_digest":request_digest,
        "declared_by_user_id":input.declared_by_user_id,
        "declared_at":declared_at,
    }))
}

pub(super) fn candidate_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredTerminalCandidate>> {
    conn.query_row(
        &format!(
            "{} WHERE idempotency_scope=?1 AND idempotency_key=?2",
            select_sql()
        ),
        params![scope, key],
        stored_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn candidate_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredTerminalCandidate>> {
    conn.query_row(
        &format!("{} WHERE lease_id=?1", select_sql()),
        params![lease_id],
        stored_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn select_sql() -> &'static str {
    "SELECT terminal_candidate_id, lease_id, provider_id, consumer_account_id,
            source_lease_revision, source_lease_digest, source_lease_status,
            fencing_generation, job_id, job_revision, job_digest,
            reservation_id, reservation_revision, reservation_digest,
            capacity_claim_id, capacity_claim_revision, capacity_claim_digest,
            final_usage_snapshot_id, final_usage_sequence_no,
            final_cumulative_usage_digest, executor_terminal_ref, outcome,
            reason_code, diagnostic_ref, output_digest, result_artifacts_json,
            result_artifacts_digest, request_digest, event_digest,
            idempotency_scope, idempotency_key, declared_by_user_id,
            declared_at, created_at
       FROM compute_attempt_terminal_candidates"
}

fn stored_from_row(row: &Row<'_>) -> rusqlite::Result<StoredTerminalCandidate> {
    Ok(StoredTerminalCandidate {
        terminal_candidate_id: row.get(0)?,
        lease_id: row.get(1)?,
        provider_id: row.get(2)?,
        consumer_account_id: row.get(3)?,
        source_lease_revision: row.get(4)?,
        source_lease_digest: row.get(5)?,
        source_lease_status: row.get(6)?,
        fencing_generation: row.get(7)?,
        job_id: row.get(8)?,
        job_revision: row.get(9)?,
        job_digest: row.get(10)?,
        reservation_id: row.get(11)?,
        reservation_revision: row.get(12)?,
        reservation_digest: row.get(13)?,
        capacity_claim_id: row.get(14)?,
        capacity_claim_revision: row.get(15)?,
        capacity_claim_digest: row.get(16)?,
        final_usage_snapshot_id: row.get(17)?,
        final_usage_sequence_no: row.get(18)?,
        final_cumulative_usage_digest: row.get(19)?,
        executor_terminal_ref: row.get(20)?,
        outcome: row.get(21)?,
        reason_code: row.get(22)?,
        diagnostic_ref: row.get(23)?,
        output_digest: row.get(24)?,
        result_artifacts: parse_json(row, 25)?,
        result_artifacts_digest: row.get(26)?,
        request_digest: row.get(27)?,
        event_digest: row.get(28)?,
        idempotency_scope: row.get(29)?,
        idempotency_key: row.get(30)?,
        declared_by_user_id: row.get(31)?,
        declared_at: row.get(32)?,
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

pub(super) fn digest_json(value: &impl Serialize) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value)?)))
}
