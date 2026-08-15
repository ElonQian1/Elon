//! Platform-administrator orchestration for V268 signed runtime compatibility evidence.

use anyhow::Error as AnyError;
use serde_json::Value;
use thiserror::Error;

use crate::{
    compute_federation::external_pool_adapter_runtime_compatibility_verification::{
        server_runtime_compatibility_v2_profile_catalog,
        CreateExternalPoolAdapterRuntimeCompatibilityChallengeInput,
        RecordExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput,
        RevokeExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput,
        RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_CONFIRMATION,
        RUNTIME_COMPATIBILITY_VERIFICATION_CONFIRMATION,
        RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_CONFIRMATION,
    },
    store::{ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError, Store},
};

use super::external_pool_adapter_runtime_compatibility_verification_service_redaction::redacted_json;
pub(crate) use super::external_pool_adapter_runtime_compatibility_verification_service_validation::{
    CreateRuntimeCompatibilityChallengeBody, RecordRuntimeCompatibilityVerificationBody,
    RevokeRuntimeCompatibilityVerificationBody,
};
use super::external_pool_adapter_runtime_compatibility_verification_service_validation::{
    validate_challenge, validate_identifier, validate_record, validate_revoke,
};

#[derive(Debug, Error)]
pub(crate) enum RuntimeCompatibilityVerificationServiceError {
    #[error("external-pool Adapter runtime compatibility authority was not found")]
    NotFound,
    #[error("external-pool Adapter runtime compatibility request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter runtime compatibility authority conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool Adapter runtime compatibility internal operation failed")]
    Internal(#[source] AnyError),
}

pub(crate) fn profile_v2_for_admin() -> Result<Value, RuntimeCompatibilityVerificationServiceError>
{
    let profile = server_runtime_compatibility_v2_profile_catalog()
        .map_err(RuntimeCompatibilityVerificationServiceError::Internal)?;
    redacted_json(profile)
}

pub(crate) fn challenge_for_admin(
    store: &Store,
    admin_user_id: &str,
    registry_release_id: &str,
    body: CreateRuntimeCompatibilityChallengeBody,
) -> Result<Value, RuntimeCompatibilityVerificationServiceError> {
    validate_challenge(admin_user_id, registry_release_id, &body)?;
    require_registry_release(store, registry_release_id)?;
    require_sandbox_verifier_key_record(store, &body.sandbox_verifier_key_record_id)?;
    let (predecessor_verification_receipt_id, predecessor_verification_receipt_digest) = body
        .expected_predecessor
        .map(|value| {
            (
                Some(value.verification_receipt_id),
                Some(value.verification_receipt_digest),
            )
        })
        .unwrap_or((None, None));
    if let Some(predecessor_id) = predecessor_verification_receipt_id.as_deref() {
        require_verification_receipt(store, predecessor_id, registry_release_id)?;
    }
    let output = store
        .issue_external_pool_adapter_runtime_compatibility_verification_challenge(
            admin_user_id,
            CreateExternalPoolAdapterRuntimeCompatibilityChallengeInput {
                registry_release_id: registry_release_id.to_string(),
                expected_registry_release_digest: body.expected_registry_release_digest,
                expected_profile_digest: body.expected_profile_digest,
                expected_runner_policy_digest: body.expected_runner_policy_digest,
                expected_fixture_catalog_digest: body.expected_fixture_catalog_digest,
                sandbox_verifier_key_record_id: body.sandbox_verifier_key_record_id,
                expected_sandbox_verifier_key_record_digest: body
                    .expected_sandbox_verifier_key_record_digest,
                expected_sandbox_verifier_key_id: body.expected_sandbox_verifier_key_id,
                predecessor_verification_receipt_id,
                predecessor_verification_receipt_digest,
                idempotency_key: body.idempotency_key,
                confirmation: RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_CONFIRMATION.into(),
            },
        )
        .map_err(classify_store_error)?;
    redacted_json(output)
}

pub(crate) fn record_for_admin(
    store: &Store,
    admin_user_id: &str,
    registry_release_id: &str,
    body: RecordRuntimeCompatibilityVerificationBody,
) -> Result<Value, RuntimeCompatibilityVerificationServiceError> {
    validate_record(admin_user_id, registry_release_id, &body)?;
    require_registry_release(store, registry_release_id)?;
    require_run_observation(store, &body.run_observation_id, registry_release_id)?;
    let output = store
        .record_external_pool_adapter_runtime_compatibility_verification(
            admin_user_id,
            RecordExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput {
                run_observation_id: body.run_observation_id,
                expected_run_observation_digest: body.expected_run_observation_digest,
                expected_signature_message_digest: body.expected_signature_message_digest,
                signature_base64: body.signature_base64,
                idempotency_key: body.idempotency_key,
                confirmation: RUNTIME_COMPATIBILITY_VERIFICATION_CONFIRMATION.into(),
            },
        )
        .map_err(classify_store_error)?;
    redacted_json(output)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    registry_release_id: &str,
) -> Result<Value, RuntimeCompatibilityVerificationServiceError> {
    validate_identifier(registry_release_id, 200, "registry release ID")?;
    require_registry_release(store, registry_release_id)?;
    let output = store
        .external_pool_adapter_runtime_compatibility_verification_currentness(registry_release_id)
        .map_err(classify_store_error)?
        .ok_or(RuntimeCompatibilityVerificationServiceError::NotFound)?;
    redacted_json(output)
}

pub(crate) fn revoke_for_admin(
    store: &Store,
    admin_user_id: &str,
    registry_release_id: &str,
    verification_receipt_id: &str,
    body: RevokeRuntimeCompatibilityVerificationBody,
) -> Result<Value, RuntimeCompatibilityVerificationServiceError> {
    validate_revoke(
        admin_user_id,
        registry_release_id,
        verification_receipt_id,
        &body,
    )?;
    require_registry_release(store, registry_release_id)?;
    require_verification_receipt(store, verification_receipt_id, registry_release_id)?;
    let output = store
        .revoke_external_pool_adapter_runtime_compatibility_verification(
            admin_user_id,
            RevokeExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput {
                verification_receipt_id: verification_receipt_id.to_string(),
                expected_verification_receipt_digest: body.expected_verification_receipt_digest,
                reason: body.reason,
                idempotency_key: body.idempotency_key,
                confirmation: RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_CONFIRMATION.into(),
            },
        )
        .map_err(classify_store_error)?;
    redacted_json(output)
}

fn require_registry_release(
    store: &Store,
    registry_release_id: &str,
) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    if store
        .external_pool_adapter_registry_release_exists(registry_release_id)
        .map_err(RuntimeCompatibilityVerificationServiceError::Internal)?
    {
        Ok(())
    } else {
        Err(RuntimeCompatibilityVerificationServiceError::NotFound)
    }
}

fn require_run_observation(
    store: &Store,
    run_observation_id: &str,
    registry_release_id: &str,
) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    if store
        .external_pool_adapter_runtime_compatibility_verification_run_observation_exists(
            run_observation_id,
            registry_release_id,
        )
        .map_err(classify_store_error)?
    {
        Ok(())
    } else {
        Err(RuntimeCompatibilityVerificationServiceError::NotFound)
    }
}

fn require_sandbox_verifier_key_record(
    store: &Store,
    sandbox_verifier_key_record_id: &str,
) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    if store
        .external_pool_adapter_sandbox_verifier_key_currentness(sandbox_verifier_key_record_id)
        .map_err(RuntimeCompatibilityVerificationServiceError::Internal)?
        .is_some()
    {
        Ok(())
    } else {
        Err(RuntimeCompatibilityVerificationServiceError::NotFound)
    }
}

fn require_verification_receipt(
    store: &Store,
    verification_receipt_id: &str,
    registry_release_id: &str,
) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    if store
        .external_pool_adapter_runtime_compatibility_verification_exists(
            verification_receipt_id,
            registry_release_id,
        )
        .map_err(classify_store_error)?
    {
        Ok(())
    } else {
        Err(RuntimeCompatibilityVerificationServiceError::NotFound)
    }
}

fn classify_store_error(
    error: ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError,
) -> RuntimeCompatibilityVerificationServiceError {
    match error {
        ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError::Conflict(error) => {
            RuntimeCompatibilityVerificationServiceError::Conflict(error)
        }
        ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError::Storage(error) => {
            RuntimeCompatibilityVerificationServiceError::Internal(error)
        }
    }
}
