//! Administrator orchestration for verifier-signed exact-artifact sandbox conformance.

use anyhow::Error as AnyError;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use thiserror::Error;

use crate::store::{
    CreateExternalPoolAdapterSandboxConformance, ExternalPoolAdapterSandboxConformanceCurrentness,
    ExternalPoolAdapterSandboxConformanceWriteReceipt,
    GetExternalPoolAdapterSandboxConformanceChallenge, Store,
};

use super::external_pool_adapter_artifact_sandbox_conformance::{
    validate_sandbox_conformance_draft, ExternalPoolAdapterSandboxCapabilityObservation,
    ExternalPoolAdapterSandboxConformanceChallenge, ExternalPoolAdapterSandboxConformanceDraft,
    SANDBOX_CONFORMANCE_CONFIRMATION,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SandboxConformanceChallengeBody {
    pub expected_vulnerability_report_receipt_digest: String,
    pub sandbox_verifier_key_record_id: String,
    pub expected_sandbox_verifier_key_record_digest: String,
    pub expected_sandbox_verifier_key_id: String,
    pub verifier_report_id: String,
    pub sandbox_runtime_id: String,
    pub runtime_image_digest: String,
    pub isolation_profile_id: String,
    pub run_started_at: String,
    pub run_completed_at: String,
    pub report_generated_at: String,
    pub report_expires_at: String,
    pub external_network_attempt_count: u64,
    pub write_outside_ephemeral_count: u64,
    pub child_process_attempt_count: u64,
    pub peak_memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub observations: Vec<ExternalPoolAdapterSandboxCapabilityObservation>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordSandboxConformanceBody {
    pub expected_vulnerability_report_receipt_digest: String,
    pub sandbox_verifier_key_record_id: String,
    pub expected_sandbox_verifier_key_record_digest: String,
    pub expected_sandbox_verifier_key_id: String,
    pub verifier_report_id: String,
    pub sandbox_runtime_id: String,
    pub runtime_image_digest: String,
    pub isolation_profile_id: String,
    pub run_started_at: String,
    pub run_completed_at: String,
    pub report_generated_at: String,
    pub report_expires_at: String,
    pub external_network_attempt_count: u64,
    pub write_outside_ephemeral_count: u64,
    pub child_process_attempt_count: u64,
    pub peak_memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub observations: Vec<ExternalPoolAdapterSandboxCapabilityObservation>,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub idempotency_key: String,
    pub confirm_conformance: bool,
}

#[derive(Debug, Error)]
pub(crate) enum SandboxConformanceServiceError {
    #[error("external-pool Adapter sandbox conformance was not found")]
    NotFound,
    #[error("external-pool Adapter sandbox-conformance request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter sandbox-conformance lineage conflicts")]
    Conflict(#[source] AnyError),
}

pub(crate) fn challenge_for_admin(
    store: &Store,
    admission_id: &str,
    body: SandboxConformanceChallengeBody,
) -> Result<ExternalPoolAdapterSandboxConformanceChallenge, SandboxConformanceServiceError> {
    let draft = draft_from_challenge(body.clone());
    let input = challenge_input(
        admission_id,
        &body.expected_vulnerability_report_receipt_digest,
        &body.sandbox_verifier_key_record_id,
        &body.expected_sandbox_verifier_key_record_digest,
        &body.expected_sandbox_verifier_key_id,
        draft,
    );
    validate_challenge_request(&input)?;
    require_admission(store, admission_id)?;
    store
        .external_pool_adapter_sandbox_conformance_challenge(input)
        .map_err(SandboxConformanceServiceError::Conflict)
}

pub(crate) fn record_for_admin(
    store: &Store,
    admin_user_id: &str,
    admission_id: &str,
    body: RecordSandboxConformanceBody,
) -> Result<ExternalPoolAdapterSandboxConformanceWriteReceipt, SandboxConformanceServiceError> {
    if !body.confirm_conformance {
        return Err(SandboxConformanceServiceError::Invalid(anyhow::anyhow!(
            "记录沙箱符合性前必须显式确认"
        )));
    }
    let challenge = challenge_input(
        admission_id,
        &body.expected_vulnerability_report_receipt_digest,
        &body.sandbox_verifier_key_record_id,
        &body.expected_sandbox_verifier_key_record_digest,
        &body.expected_sandbox_verifier_key_id,
        draft_from_record(&body),
    );
    let idempotency_scope = format!("external-pool-adapter-sandbox-conformance:{admin_user_id}");
    validate_challenge_request(&challenge)?;
    validate_identifier(admin_user_id, 200, "administrator user ID")?;
    validate_identifier(&idempotency_scope, 240, "idempotency scope")?;
    validate_identifier(&body.idempotency_key, 240, "idempotency key")?;
    validate_digest(
        &body.expected_signature_message_digest,
        "signature message digest",
    )?;
    validate_signature(&body.signature_base64)?;
    require_admission(store, admission_id)?;
    store
        .create_external_pool_adapter_sandbox_conformance(
            CreateExternalPoolAdapterSandboxConformance {
                challenge,
                expected_signature_message_digest: body.expected_signature_message_digest,
                signature_base64: body.signature_base64,
                verified_by_admin_user_id: admin_user_id.to_string(),
                confirmation: SANDBOX_CONFORMANCE_CONFIRMATION.to_string(),
                idempotency_scope,
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(SandboxConformanceServiceError::Conflict)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    admission_id: &str,
) -> Result<ExternalPoolAdapterSandboxConformanceCurrentness, SandboxConformanceServiceError> {
    validate_identifier(admission_id, 160, "admission ID")?;
    store
        .external_pool_adapter_sandbox_conformance_currentness(admission_id)
        .map_err(SandboxConformanceServiceError::Conflict)?
        .ok_or(SandboxConformanceServiceError::NotFound)
}

fn challenge_input(
    admission_id: &str,
    vulnerability_digest: &str,
    verifier_key_record_id: &str,
    verifier_key_record_digest: &str,
    verifier_key_id: &str,
    draft: ExternalPoolAdapterSandboxConformanceDraft,
) -> GetExternalPoolAdapterSandboxConformanceChallenge {
    GetExternalPoolAdapterSandboxConformanceChallenge {
        admission_id: admission_id.to_string(),
        expected_vulnerability_report_receipt_digest: vulnerability_digest.to_string(),
        sandbox_verifier_key_record_id: verifier_key_record_id.to_string(),
        expected_sandbox_verifier_key_record_digest: verifier_key_record_digest.to_string(),
        expected_sandbox_verifier_key_id: verifier_key_id.to_string(),
        draft,
    }
}

fn draft_from_challenge(
    body: SandboxConformanceChallengeBody,
) -> ExternalPoolAdapterSandboxConformanceDraft {
    ExternalPoolAdapterSandboxConformanceDraft {
        verifier_report_id: body.verifier_report_id,
        sandbox_runtime_id: body.sandbox_runtime_id,
        runtime_image_digest: body.runtime_image_digest,
        isolation_profile_id: body.isolation_profile_id,
        run_started_at: body.run_started_at,
        run_completed_at: body.run_completed_at,
        report_generated_at: body.report_generated_at,
        report_expires_at: body.report_expires_at,
        external_network_attempt_count: body.external_network_attempt_count,
        write_outside_ephemeral_count: body.write_outside_ephemeral_count,
        child_process_attempt_count: body.child_process_attempt_count,
        peak_memory_bytes: body.peak_memory_bytes,
        cpu_time_ms: body.cpu_time_ms,
        observations: body.observations,
    }
}

fn draft_from_record(
    body: &RecordSandboxConformanceBody,
) -> ExternalPoolAdapterSandboxConformanceDraft {
    ExternalPoolAdapterSandboxConformanceDraft {
        verifier_report_id: body.verifier_report_id.clone(),
        sandbox_runtime_id: body.sandbox_runtime_id.clone(),
        runtime_image_digest: body.runtime_image_digest.clone(),
        isolation_profile_id: body.isolation_profile_id.clone(),
        run_started_at: body.run_started_at.clone(),
        run_completed_at: body.run_completed_at.clone(),
        report_generated_at: body.report_generated_at.clone(),
        report_expires_at: body.report_expires_at.clone(),
        external_network_attempt_count: body.external_network_attempt_count,
        write_outside_ephemeral_count: body.write_outside_ephemeral_count,
        child_process_attempt_count: body.child_process_attempt_count,
        peak_memory_bytes: body.peak_memory_bytes,
        cpu_time_ms: body.cpu_time_ms,
        observations: body.observations.clone(),
    }
}

fn validate_challenge_request(
    input: &GetExternalPoolAdapterSandboxConformanceChallenge,
) -> Result<(), SandboxConformanceServiceError> {
    validate_identifier(&input.admission_id, 160, "admission ID")?;
    validate_identifier(
        &input.sandbox_verifier_key_record_id,
        160,
        "sandbox verifier key record ID",
    )?;
    for (value, label) in [
        (
            input.expected_vulnerability_report_receipt_digest.as_str(),
            "vulnerability report receipt digest",
        ),
        (
            input.expected_sandbox_verifier_key_record_digest.as_str(),
            "sandbox verifier key record digest",
        ),
        (
            input.expected_sandbox_verifier_key_id.as_str(),
            "sandbox verifier key ID",
        ),
    ] {
        validate_digest(value, label)?;
    }
    validate_sandbox_conformance_draft(&input.draft)
        .map_err(SandboxConformanceServiceError::Invalid)
}

fn validate_identifier(
    value: &str,
    max: usize,
    label: &'static str,
) -> Result<(), SandboxConformanceServiceError> {
    if value.is_empty()
        || value.chars().count() > max
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(SandboxConformanceServiceError::Invalid(anyhow::anyhow!(
            "sandbox conformance {label} is invalid"
        )));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &'static str) -> Result<(), SandboxConformanceServiceError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(SandboxConformanceServiceError::Invalid(anyhow::anyhow!(
            "sandbox conformance {label} is invalid"
        )));
    }
    Ok(())
}

fn validate_signature(value: &str) -> Result<(), SandboxConformanceServiceError> {
    if value.is_empty() || value.len() > 1_368 {
        return Err(invalid("sandbox verifier signature Base64 is invalid"));
    }
    let signature = STANDARD
        .decode(value)
        .map_err(|error| SandboxConformanceServiceError::Invalid(AnyError::new(error)))?;
    if signature.is_empty() || signature.len() > 1024 || STANDARD.encode(&signature) != value {
        return Err(invalid("sandbox verifier signature Base64 is invalid"));
    }
    Ok(())
}

fn require_admission(
    store: &Store,
    admission_id: &str,
) -> Result<(), SandboxConformanceServiceError> {
    store
        .external_pool_adapter_release_admission_currentness(admission_id)
        .map_err(SandboxConformanceServiceError::Conflict)?
        .map(|_| ())
        .ok_or(SandboxConformanceServiceError::NotFound)
}

fn invalid(message: &'static str) -> SandboxConformanceServiceError {
    SandboxConformanceServiceError::Invalid(anyhow::anyhow!(message))
}
