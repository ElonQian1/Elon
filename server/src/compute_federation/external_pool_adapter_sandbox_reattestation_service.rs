//! Administrator orchestration and public redaction for V252 sandbox re-attestation.

use anyhow::Error as AnyError;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::store::{
    CreateExternalPoolAdapterSandboxReattestation,
    GetExternalPoolAdapterSandboxReattestationChallenge,
    RevokeExternalPoolAdapterSandboxReattestation, Store,
};

use super::{
    external_pool_adapter_artifact_sandbox_conformance::{
        validate_sandbox_conformance_draft, ExternalPoolAdapterSandboxCapabilityObservation,
        ExternalPoolAdapterSandboxConformanceDraft,
    },
    external_pool_adapter_sandbox_reattestation::{
        ExternalPoolAdapterSandboxReattestationChallenge, SANDBOX_REATTESTATION_CONFIRMATION,
        SANDBOX_REATTESTATION_REVOCATION_CONFIRMATION,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SandboxReattestationChallengeBody {
    pub expected_registry_release_digest: String,
    pub vulnerability_reattestation_receipt_id: String,
    pub expected_vulnerability_reattestation_receipt_digest: String,
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
pub(crate) struct RecordSandboxReattestationBody {
    pub challenge_id: String,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub idempotency_key: String,
    pub confirm_reattestation: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeSandboxReattestationBody {
    pub expected_reattestation_receipt_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct SandboxReattestationChallengeResponse {
    pub schema: &'static str,
    pub canonicalization: &'static str,
    pub digest_algorithm: &'static str,
    pub signature_algorithm: &'static str,
    pub signature_message_base64: String,
    pub signature_message_digest: String,
    pub binding: Value,
}

#[derive(Debug, Error)]
pub(crate) enum SandboxReattestationServiceError {
    #[error("external-pool Adapter sandbox re-attestation was not found")]
    NotFound,
    #[error("external-pool Adapter sandbox re-attestation request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter sandbox re-attestation authority conflicts")]
    Conflict(#[source] AnyError),
}

pub(crate) fn challenge_for_admin(
    store: &Store,
    registry_release_id: &str,
    body: SandboxReattestationChallengeBody,
) -> Result<SandboxReattestationChallengeResponse, SandboxReattestationServiceError> {
    validate_identifier(registry_release_id, 200, "registry release ID")?;
    for (value, label) in [
        (
            body.vulnerability_reattestation_receipt_id.as_str(),
            "vulnerability re-attestation receipt ID",
        ),
        (
            body.sandbox_verifier_key_record_id.as_str(),
            "sandbox verifier key record ID",
        ),
    ] {
        validate_identifier(value, 200, label)?;
    }
    for (value, label) in [
        (
            body.expected_registry_release_digest.as_str(),
            "registry release digest",
        ),
        (
            body.expected_vulnerability_reattestation_receipt_digest
                .as_str(),
            "vulnerability re-attestation receipt digest",
        ),
        (
            body.expected_sandbox_verifier_key_record_digest.as_str(),
            "sandbox verifier key record digest",
        ),
        (
            body.expected_sandbox_verifier_key_id.as_str(),
            "sandbox verifier key ID",
        ),
    ] {
        validate_digest(value, label)?;
    }
    let draft = draft_from_body(&body);
    validate_sandbox_conformance_draft(&draft)
        .map_err(SandboxReattestationServiceError::Invalid)?;
    let input = GetExternalPoolAdapterSandboxReattestationChallenge {
        registry_release_id: registry_release_id.to_string(),
        expected_registry_release_digest: body.expected_registry_release_digest,
        vulnerability_reattestation_receipt_id: body.vulnerability_reattestation_receipt_id,
        expected_vulnerability_reattestation_receipt_digest: body
            .expected_vulnerability_reattestation_receipt_digest,
        sandbox_verifier_key_record_id: body.sandbox_verifier_key_record_id,
        expected_sandbox_verifier_key_record_digest: body
            .expected_sandbox_verifier_key_record_digest,
        expected_sandbox_verifier_key_id: body.expected_sandbox_verifier_key_id,
        draft,
    };
    require_registry_release(store, registry_release_id)?;
    if !store
        .external_pool_adapter_vulnerability_reattestation_exists(
            &input.vulnerability_reattestation_receipt_id,
            registry_release_id,
        )
        .map_err(SandboxReattestationServiceError::Conflict)?
    {
        return Err(SandboxReattestationServiceError::NotFound);
    }
    let challenge = store
        .issue_external_pool_adapter_sandbox_reattestation_challenge(input)
        .map_err(SandboxReattestationServiceError::Conflict)?;
    public_challenge(challenge)
}

pub(crate) fn record_for_admin(
    store: &Store,
    admin_user_id: &str,
    registry_release_id: &str,
    body: RecordSandboxReattestationBody,
) -> Result<Value, SandboxReattestationServiceError> {
    if !body.confirm_reattestation {
        return Err(invalid(
            "sandbox re-attestation requires explicit confirmation",
        ));
    }
    for (value, maximum, label) in [
        (admin_user_id, 200, "administrator user ID"),
        (registry_release_id, 200, "registry release ID"),
        (body.challenge_id.as_str(), 200, "challenge ID"),
        (body.idempotency_key.as_str(), 240, "idempotency key"),
    ] {
        validate_identifier(value, maximum, label)?;
    }
    validate_digest(
        &body.expected_signature_message_digest,
        "signature message digest",
    )?;
    validate_signature(&body.signature_base64)?;
    require_registry_release(store, registry_release_id)?;
    if !store
        .external_pool_adapter_sandbox_reattestation_challenge_exists(
            &body.challenge_id,
            registry_release_id,
        )
        .map_err(SandboxReattestationServiceError::Conflict)?
    {
        return Err(SandboxReattestationServiceError::NotFound);
    }
    let receipt = store
        .create_external_pool_adapter_sandbox_reattestation(
            CreateExternalPoolAdapterSandboxReattestation {
                challenge_id: body.challenge_id,
                expected_signature_message_digest: body.expected_signature_message_digest,
                signature_base64: body.signature_base64,
                recorded_by_admin_user_id: admin_user_id.to_string(),
                confirmation: SANDBOX_REATTESTATION_CONFIRMATION.to_string(),
                idempotency_scope: format!("v252:sandbox-reattest:{admin_user_id}"),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(SandboxReattestationServiceError::Conflict)?;
    redacted_json(receipt)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    registry_release_id: &str,
) -> Result<Value, SandboxReattestationServiceError> {
    validate_identifier(registry_release_id, 200, "registry release ID")?;
    require_registry_release(store, registry_release_id)?;
    let current = store
        .external_pool_adapter_sandbox_reattestation_currentness(registry_release_id)
        .map_err(SandboxReattestationServiceError::Conflict)?
        .ok_or(SandboxReattestationServiceError::NotFound)?;
    if current.current_status != "verified_current" {
        return Err(SandboxReattestationServiceError::Conflict(anyhow::anyhow!(
            "sandbox re-attestation is not current"
        )));
    }
    redacted_json(current)
}

pub(crate) fn revoke_for_admin(
    store: &Store,
    admin_user_id: &str,
    registry_release_id: &str,
    reattestation_receipt_id: &str,
    body: RevokeSandboxReattestationBody,
) -> Result<Value, SandboxReattestationServiceError> {
    if !body.confirm_revocation {
        return Err(invalid(
            "sandbox re-attestation revocation requires confirmation",
        ));
    }
    for (value, maximum, label) in [
        (admin_user_id, 200, "administrator user ID"),
        (registry_release_id, 200, "registry release ID"),
        (reattestation_receipt_id, 200, "re-attestation receipt ID"),
        (body.idempotency_key.as_str(), 240, "idempotency key"),
    ] {
        validate_identifier(value, maximum, label)?;
    }
    validate_digest(
        &body.expected_reattestation_receipt_digest,
        "re-attestation receipt digest",
    )?;
    if body.reason.trim() != body.reason
        || !(12..=500).contains(&body.reason.chars().count())
        || body.reason.chars().any(char::is_control)
    {
        return Err(invalid(
            "sandbox re-attestation revocation reason is invalid",
        ));
    }
    require_registry_release(store, registry_release_id)?;
    if !store
        .external_pool_adapter_sandbox_reattestation_exists(
            reattestation_receipt_id,
            registry_release_id,
        )
        .map_err(SandboxReattestationServiceError::Conflict)?
    {
        return Err(SandboxReattestationServiceError::NotFound);
    }
    let receipt = store
        .revoke_external_pool_adapter_sandbox_reattestation(
            RevokeExternalPoolAdapterSandboxReattestation {
                reattestation_receipt_id: reattestation_receipt_id.to_string(),
                expected_reattestation_receipt_digest: body.expected_reattestation_receipt_digest,
                revoked_by_admin_user_id: admin_user_id.to_string(),
                reason: body.reason,
                confirmation: SANDBOX_REATTESTATION_REVOCATION_CONFIRMATION.to_string(),
                idempotency_scope: format!("v252:sandbox-reattest-revoke:{admin_user_id}"),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(SandboxReattestationServiceError::Conflict)?;
    redacted_json(receipt)
}

fn draft_from_body(
    body: &SandboxReattestationChallengeBody,
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

fn public_challenge(
    challenge: ExternalPoolAdapterSandboxReattestationChallenge,
) -> Result<SandboxReattestationChallengeResponse, SandboxReattestationServiceError> {
    let mut binding = serde_json::to_value(challenge.binding)
        .map_err(|error| SandboxReattestationServiceError::Conflict(AnyError::new(error)))?;
    let nonce = binding.get("challenge_nonce_base64").cloned();
    let nonce_digest = binding.get("challenge_nonce_digest").cloned();
    redact(&mut binding);
    if let Value::Object(map) = &mut binding {
        if let Some(value) = nonce {
            map.insert("nonce_base64".to_string(), value);
        }
        if let Some(value) = nonce_digest {
            map.insert("nonce_digest".to_string(), value);
        }
    }
    Ok(SandboxReattestationChallengeResponse {
        schema: challenge.schema,
        canonicalization: challenge.canonicalization,
        digest_algorithm: challenge.digest_algorithm,
        signature_algorithm: challenge.signature_algorithm,
        signature_message_base64: challenge.signature_message_base64,
        signature_message_digest: challenge.signature_message_digest,
        binding,
    })
}

fn redacted_json<T: Serialize>(value: T) -> Result<Value, SandboxReattestationServiceError> {
    let mut value = serde_json::to_value(value)
        .map_err(|error| SandboxReattestationServiceError::Conflict(AnyError::new(error)))?;
    redact(&mut value);
    Ok(value)
}

fn redact(value: &mut Value) {
    const ALWAYS: &[&str] = &[
        "public_key_pem",
        "test_plan",
        "observations",
        "output_transcript_digest",
        "sandbox_verifier_operator",
        "sandbox_verifier_product",
        "recorded_by_admin_user_id",
        "revoked_by_admin_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "receipt_json",
        "installation_path",
        "installation_root",
        "install_root",
        "installed_path",
        "entrypoint_path",
        "filesystem_path",
        "source_path",
        "archive_path",
        "path",
    ];
    const SIGNATURE_MATERIAL: &[&str] = &[
        "challenge_nonce_base64",
        "challenge_nonce_digest",
        "nonce_base64",
        "nonce_digest",
        "signature_message_base64",
        "signature_message_digest",
        "signature_base64",
        "signature_digest",
    ];
    match value {
        Value::Object(map) => {
            for key in ALWAYS.iter().chain(SIGNATURE_MATERIAL) {
                map.remove(*key);
            }
            for child in map.values_mut() {
                redact(child);
            }
        }
        Value::Array(values) => values.iter_mut().for_each(redact),
        _ => {}
    }
}

fn validate_identifier(
    value: &str,
    maximum: usize,
    label: &'static str,
) -> Result<(), SandboxReattestationServiceError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        return Err(invalid(format!("{label} is invalid")));
    }
    Ok(())
}

fn validate_digest(
    value: &str,
    label: &'static str,
) -> Result<(), SandboxReattestationServiceError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid(format!("{label} is invalid")));
    }
    Ok(())
}

fn validate_signature(value: &str) -> Result<(), SandboxReattestationServiceError> {
    if value.is_empty() || value.len() > 1_368 {
        return Err(invalid("sandbox verifier signature Base64 is invalid"));
    }
    let signature = STANDARD
        .decode(value)
        .map_err(|error| SandboxReattestationServiceError::Invalid(AnyError::new(error)))?;
    if signature.is_empty() || signature.len() > 1024 || STANDARD.encode(&signature) != value {
        return Err(invalid("sandbox verifier signature Base64 is invalid"));
    }
    Ok(())
}

fn require_registry_release(
    store: &Store,
    registry_release_id: &str,
) -> Result<(), SandboxReattestationServiceError> {
    if store
        .external_pool_adapter_registry_release_exists(registry_release_id)
        .map_err(SandboxReattestationServiceError::Conflict)?
    {
        Ok(())
    } else {
        Err(SandboxReattestationServiceError::NotFound)
    }
}

fn invalid(message: impl Into<String>) -> SandboxReattestationServiceError {
    SandboxReattestationServiceError::Invalid(anyhow::anyhow!(message.into()))
}
