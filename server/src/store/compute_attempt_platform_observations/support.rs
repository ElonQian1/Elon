use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::receipts::ComputeMeterReading,
    store::{ComputeAttemptTerminalCandidateReceipt, ComputeAttemptUsageDeclarationReceipt},
};

use super::{
    ComputeAttemptPlatformObservationReceipt, ComputeObservedUsageInput,
    ObserveComputeAttemptTerminalCandidateRequest, COMPUTE_ATTEMPT_PLATFORM_OBSERVATION_SCHEMA,
    OBSERVATION_SOURCE_CONTROL_PLANE, OBSERVATION_SOURCE_SERVER_METERING,
    OBSERVATION_SOURCE_TRANSPORT_GATEWAY, OBSERVED_OUTCOME_CANCELED, OBSERVED_OUTCOME_FAILED,
    OBSERVED_OUTCOME_INDETERMINATE, OBSERVED_OUTCOME_SUCCEEDED,
};

mod audit;
mod persistence;

pub(super) use persistence::{
    platform_observation_by_candidate_on, platform_observation_by_idempotency_on,
    platform_observation_by_lease_on,
};

const MAX_EVIDENCE_REFS: usize = 16;

#[derive(Debug, Clone)]
pub(super) struct StoredPlatformObservation {
    pub platform_observation_id: String,
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
    pub final_provider_usage_digest: String,
    pub candidate_outcome: String,
    pub observation_source: String,
    pub observer_ref: String,
    pub observed_outcome: String,
    pub cumulative_observed_usage: Vec<ComputeMeterReading>,
    pub cumulative_observed_usage_digest: String,
    pub variance_meters: Vec<String>,
    pub variance_meters_digest: String,
    pub evidence_refs: Vec<String>,
    pub evidence_refs_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub observed_by_user_id: String,
    pub observed_at: String,
    pub created_at: String,
}

impl StoredPlatformObservation {
    pub(super) fn into_receipt(
        self,
        replayed: bool,
    ) -> Result<ComputeAttemptPlatformObservationReceipt> {
        audit::audit_platform_observation(&self)?;
        Ok(ComputeAttemptPlatformObservationReceipt {
            schema: COMPUTE_ATTEMPT_PLATFORM_OBSERVATION_SCHEMA,
            platform_observation_id: self.platform_observation_id,
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
            final_provider_usage_digest: self.final_provider_usage_digest,
            candidate_outcome: self.candidate_outcome,
            observation_source: self.observation_source,
            observer_ref: self.observer_ref,
            observed_outcome: self.observed_outcome,
            cumulative_observed_usage: self.cumulative_observed_usage,
            cumulative_observed_usage_digest: self.cumulative_observed_usage_digest,
            variance_meters: self.variance_meters,
            variance_meters_digest: self.variance_meters_digest,
            evidence_refs: self.evidence_refs,
            evidence_refs_digest: self.evidence_refs_digest,
            request_digest: self.request_digest,
            event_digest: self.event_digest,
            observed_by_user_id: self.observed_by_user_id,
            observed_at: self.observed_at,
            evidence_status: "unverified_platform_observation",
            observation_effect: "platform_evidence_recorded",
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

pub(super) fn normalize_platform_observation_request(
    input: &ObserveComputeAttemptTerminalCandidateRequest,
) -> Result<ObserveComputeAttemptTerminalCandidateRequest> {
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
        ("观测来源", input.observation_source.as_str(), 40),
        ("平台观测引用", input.observer_ref.as_str(), 1000),
        ("平台观测结果", input.observed_outcome.as_str(), 40),
        ("幂等键", input.idempotency_key.as_str(), 200),
        ("平台观测用户 ID", input.observed_by_user_id.as_str(), 200),
    ] {
        validate_exact(label, value, max_len)?;
    }
    validate_digest(
        "终态候选事件摘要",
        &input.expected_terminal_candidate_event_digest,
    )?;
    if !matches!(
        input.observation_source.as_str(),
        OBSERVATION_SOURCE_CONTROL_PLANE
            | OBSERVATION_SOURCE_TRANSPORT_GATEWAY
            | OBSERVATION_SOURCE_SERVER_METERING
    ) {
        bail!("平台观测来源只允许 control_plane、transport_gateway 或 server_metering");
    }
    if !matches!(
        input.observed_outcome.as_str(),
        OBSERVED_OUTCOME_SUCCEEDED
            | OBSERVED_OUTCOME_FAILED
            | OBSERVED_OUTCOME_CANCELED
            | OBSERVED_OUTCOME_INDETERMINATE
    ) {
        bail!("平台观测 outcome 只允许 succeeded、failed、canceled 或 indeterminate");
    }
    if input.cumulative_observed_usage.is_empty() || input.cumulative_observed_usage.len() > 64 {
        bail!("平台累计观测必须包含 1 至 64 个 meter");
    }
    if input.evidence_refs.len() > MAX_EVIDENCE_REFS {
        bail!("平台观测证据引用数量超过上限");
    }

    let mut normalized = input.clone();
    normalized
        .cumulative_observed_usage
        .sort_by(|left, right| left.meter.cmp(&right.meter));
    let mut meters = BTreeSet::new();
    for reading in &normalized.cumulative_observed_usage {
        validate_exact("平台观测 meter", &reading.meter, 120)?;
        if reading.cumulative_quantity < 0 || !meters.insert(reading.meter.as_str()) {
            bail!("平台累计观测 meter 重复或数量为负数");
        }
    }
    normalized.evidence_refs.sort();
    let mut evidence = BTreeSet::new();
    for evidence_ref in &normalized.evidence_refs {
        validate_exact("平台观测证据引用", evidence_ref, 1000)?;
        if !evidence.insert(evidence_ref.as_str()) {
            bail!("平台观测证据引用重复");
        }
    }
    if normalized.evidence_refs.is_empty() {
        bail!("平台观测必须提供至少一个外部证据引用");
    }
    Ok(normalized)
}

pub(super) fn ensure_exact_meter_set(
    observed: &[ComputeObservedUsageInput],
    declared: &[ComputeMeterReading],
) -> Result<()> {
    if observed.len() != declared.len()
        || observed
            .iter()
            .zip(declared)
            .any(|(observed, declared)| observed.meter != declared.meter)
    {
        bail!("平台累计观测必须精确覆盖最终 Provider 快照的全部 meter");
    }
    Ok(())
}

pub(super) fn build_observed_readings(
    input: &ObserveComputeAttemptTerminalCandidateRequest,
    observed_at: &str,
) -> Result<Vec<ComputeMeterReading>> {
    input
        .cumulative_observed_usage
        .iter()
        .map(|reading| {
            let reading_digest = digest_json(&serde_json::json!({
                "purpose":"compute_attempt_platform_observed_meter_reading",
                "lease_id":input.lease_id,
                "terminal_candidate_id":input.expected_terminal_candidate_id,
                "observation_source":input.observation_source,
                "observer_ref":input.observer_ref,
                "meter":reading.meter,
                "quantity":reading.cumulative_quantity,
                "source_kind":"platform_observed",
                "observed_at":observed_at,
            }))?;
            Ok(ComputeMeterReading {
                meter: reading.meter.clone(),
                quantity: reading.cumulative_quantity,
                source_kind: "platform_observed".to_string(),
                source_id: input.observer_ref.clone(),
                reading_digest,
                observed_at: observed_at.to_string(),
            })
        })
        .collect()
}

pub(super) fn variance_meters(
    observed: &[ComputeObservedUsageInput],
    declared: &[ComputeMeterReading],
) -> Result<Vec<String>> {
    ensure_exact_meter_set(observed, declared)?;
    Ok(observed
        .iter()
        .zip(declared)
        .filter(|(observed, declared)| observed.cumulative_quantity != declared.quantity)
        .map(|(observed, _)| observed.meter.clone())
        .collect())
}

pub(super) fn observed_usage_digest(readings: &[ComputeMeterReading]) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_cumulative_platform_observed_usage",
        "readings":readings,
    }))
}

pub(super) fn variance_meters_digest(meters: &[String]) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_platform_observation_variance_meters",
        "meters":meters,
    }))
}

pub(super) fn evidence_refs_digest(evidence_refs: &[String]) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_platform_observation_evidence_refs",
        "evidence_refs":evidence_refs,
    }))
}

pub(super) fn observation_request_digest(
    input: &ObserveComputeAttemptTerminalCandidateRequest,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_platform_observation_request",
        "lease_id":input.lease_id,
        "expected_terminal_candidate_id":input.expected_terminal_candidate_id,
        "expected_terminal_candidate_event_digest":input.expected_terminal_candidate_event_digest,
        "observation_source":input.observation_source,
        "observer_ref":input.observer_ref,
        "observed_outcome":input.observed_outcome,
        "cumulative_observed_usage":input.cumulative_observed_usage,
        "evidence_refs":input.evidence_refs,
        "idempotency_key":input.idempotency_key,
        "observed_by_user_id":input.observed_by_user_id,
    }))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn observation_event_digest(
    platform_observation_id: &str,
    input: &ObserveComputeAttemptTerminalCandidateRequest,
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    provider_usage: &ComputeAttemptUsageDeclarationReceipt,
    observed_usage_digest: &str,
    variance_meters_digest: &str,
    evidence_refs_digest: &str,
    request_digest: &str,
    observed_at: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema":COMPUTE_ATTEMPT_PLATFORM_OBSERVATION_SCHEMA,
        "platform_observation_id":platform_observation_id,
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
        "final_usage_snapshot_id":provider_usage.snapshot_id,
        "final_usage_sequence_no":provider_usage.sequence_no,
        "final_provider_usage_digest":provider_usage.cumulative_usage_digest,
        "candidate_outcome":candidate.outcome,
        "observation_source":input.observation_source,
        "observer_ref":input.observer_ref,
        "observed_outcome":input.observed_outcome,
        "cumulative_observed_usage_digest":observed_usage_digest,
        "variance_meters_digest":variance_meters_digest,
        "evidence_refs_digest":evidence_refs_digest,
        "request_digest":request_digest,
        "observed_by_user_id":input.observed_by_user_id,
        "observed_at":observed_at,
    }))
}

pub(super) fn ensure_candidate_binding(
    stored: &StoredPlatformObservation,
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
        || stored.final_provider_usage_digest != candidate.final_cumulative_usage_digest
        || stored.candidate_outcome != candidate.outcome
    {
        bail!("平台观测与 Provider 终态候选绑定审计失败");
    }
    Ok(())
}

pub(super) fn ensure_provider_usage_binding(
    candidate: &ComputeAttemptTerminalCandidateReceipt,
    usage: &ComputeAttemptUsageDeclarationReceipt,
) -> Result<()> {
    if candidate.final_usage_snapshot_id != usage.snapshot_id
        || candidate.final_usage_sequence_no != usage.sequence_no
        || candidate.final_cumulative_usage_digest != usage.cumulative_usage_digest
        || candidate.lease_id != usage.lease_id
        || candidate.provider_id != usage.provider_id
        || candidate.consumer_account_id != usage.consumer_account_id
        || candidate.source_lease_revision != usage.source_lease_revision
        || candidate.source_lease_digest != usage.source_lease_digest
        || candidate.fencing_generation != usage.fencing_generation
        || candidate.job_id != usage.job_id
        || candidate.job_revision != usage.job_revision
        || candidate.job_digest != usage.job_digest
        || candidate.reservation_id != usage.reservation_id
        || candidate.reservation_revision != usage.reservation_revision
        || candidate.reservation_digest != usage.reservation_digest
        || candidate.capacity_claim_id != usage.capacity_claim_id
        || candidate.capacity_claim_revision != usage.capacity_claim_revision
        || candidate.capacity_claim_digest != usage.capacity_claim_digest
    {
        bail!("平台观测引用的最终 Provider 用量快照已漂移");
    }
    Ok(())
}

pub(super) fn ensure_observed_usage_binding(
    stored: &StoredPlatformObservation,
    provider_usage: &ComputeAttemptUsageDeclarationReceipt,
) -> Result<()> {
    let observed: Vec<ComputeObservedUsageInput> = stored
        .cumulative_observed_usage
        .iter()
        .map(|reading| ComputeObservedUsageInput {
            meter: reading.meter.clone(),
            cumulative_quantity: reading.quantity,
        })
        .collect();
    ensure_exact_meter_set(&observed, &provider_usage.cumulative_declared_usage)?;
    if stored.variance_meters
        != variance_meters(&observed, &provider_usage.cumulative_declared_usage)?
    {
        bail!("平台观测与 Provider 声明的差异 meter 审计失败");
    }
    Ok(())
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

pub(super) fn quantities_by_meter(readings: &[ComputeMeterReading]) -> BTreeMap<&str, i64> {
    readings
        .iter()
        .map(|reading| (reading.meter.as_str(), reading.quantity))
        .collect()
}
