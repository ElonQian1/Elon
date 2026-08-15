//! Admin-owned orchestration for V272 task-protocol conformance evidence.

use std::sync::Arc;

use anyhow::Error as AnyError;
use thiserror::Error;

use crate::{
    compute_federation::external_pool_adapter_task_protocol_conformance::{
        TASK_PROTOCOL_CONFORMANCE_CONFIRMATION, TASK_PROTOCOL_CONFORMANCE_REVOCATION_CONFIRMATION,
    },
    store::{
        external_pool_adapter_task_protocol_conformance_runtime,
        CreateExternalPoolAdapterTaskProtocolConformanceRun,
        ExternalPoolAdapterTaskProtocolConformanceStoreError,
        ExternalPoolAdapterTaskProtocolConformanceUnavailable,
        RevokeExternalPoolAdapterTaskProtocolConformanceRun,
    },
    types::AppState,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::compute_federation::external_pool_adapter_installation::audit_external_pool_adapter_installation;

use super::{
    external_pool_adapter_task_protocol_conformance_service_redaction::redacted_json,
    external_pool_adapter_task_protocol_conformance_service_validation::{
        idempotency_scope, validate_create, validate_currentness, validate_revoke,
        CreateTaskProtocolConformanceRunBody, RevokeTaskProtocolConformanceRunBody,
    },
};

#[derive(Debug, Error)]
pub(crate) enum TaskProtocolConformanceServiceError {
    #[error("external-pool Adapter task-protocol conformance authority was not found")]
    NotFound,
    #[error("external-pool Adapter task-protocol conformance request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter task-protocol conformance authority conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool Adapter task-protocol conformance trigger is unavailable")]
    Unavailable(#[source] ExternalPoolAdapterTaskProtocolConformanceUnavailable),
    #[error("external-pool Adapter task-protocol conformance task failed")]
    Task(#[source] tokio::task::JoinError),
    #[error("external-pool Adapter task-protocol conformance failed internally")]
    Internal(#[source] AnyError),
}

pub(crate) async fn create(
    state: Arc<AppState>,
    admin_user_id: String,
    registry_release_id: &str,
    body: CreateTaskProtocolConformanceRunBody,
) -> Result<serde_json::Value, TaskProtocolConformanceServiceError> {
    validate_create(&admin_user_id, registry_release_id, &body)
        .map_err(TaskProtocolConformanceServiceError::Invalid)?;
    let runtime = external_pool_adapter_task_protocol_conformance_runtime()
        .map_err(TaskProtocolConformanceServiceError::Unavailable)?;

    require_release_and_receipts(&state, registry_release_id, &body)?;
    let target = state
        .store
        .external_pool_adapter_registry_provider_binding_audit_target(&body.provider_binding_id)
        .map_err(TaskProtocolConformanceServiceError::Internal)?
        .ok_or(TaskProtocolConformanceServiceError::NotFound)?;
    if target.installation_receipt_id != body.expected_installation_receipt_id {
        return Err(TaskProtocolConformanceServiceError::NotFound);
    }
    if target.installation_receipt_digest != body.expected_installation_receipt_digest {
        return Err(TaskProtocolConformanceServiceError::Conflict(
            anyhow::anyhow!("installation receipt digest is not exact"),
        ));
    }

    let (predecessor_run_receipt_id, expected_predecessor_run_receipt_digest) = body
        .expected_predecessor
        .map(|predecessor| {
            (
                Some(predecessor.run_receipt_id),
                Some(predecessor.run_receipt_digest),
            )
        })
        .unwrap_or((None, None));
    let input = CreateExternalPoolAdapterTaskProtocolConformanceRun {
        registry_release_id: registry_release_id.into(),
        expected_registry_release_digest: body.expected_registry_release_digest,
        sandbox_reattestation_receipt_id: body.sandbox_reattestation_receipt_id,
        expected_sandbox_reattestation_receipt_digest: body
            .expected_sandbox_reattestation_receipt_digest,
        runtime_compatibility_verification_receipt_id: body
            .runtime_compatibility_verification_receipt_id,
        expected_runtime_compatibility_verification_receipt_digest: body
            .expected_runtime_compatibility_verification_receipt_digest,
        expected_task_protocol_profile_digest: body.expected_task_protocol_profile_digest,
        expected_fixture_catalog_digest: body.expected_fixture_catalog_digest,
        provider_binding_id: body.provider_binding_id,
        expected_provider_binding_digest: body.expected_provider_binding_digest,
        expected_installation_receipt_id: body.expected_installation_receipt_id,
        expected_installation_receipt_digest: body.expected_installation_receipt_digest,
        predecessor_run_receipt_id,
        expected_predecessor_run_receipt_digest,
        recorded_by_admin_user_id: admin_user_id.clone(),
        idempotency_scope: idempotency_scope("create", &admin_user_id),
        idempotency_key: body.idempotency_key,
        confirmation: TASK_PROTOCOL_CONFORMANCE_CONFIRMATION.into(),
    };

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let data_dir = state.data_dir.clone();
        let installation_binding = target.installation_binding;
        let output = tokio::task::spawn_blocking(move || {
            let mut reopen_prepared = || {
                audit_external_pool_adapter_installation(&data_dir, installation_binding.clone())
            };
            state
                .store
                .create_external_pool_adapter_task_protocol_conformance_run(
                    input,
                    &mut reopen_prepared,
                    &runtime,
                )
                .map_err(classify_store_error)
        })
        .await
        .map_err(TaskProtocolConformanceServiceError::Task)??;
        return redacted_json(output).map_err(TaskProtocolConformanceServiceError::Internal);
    }

    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = (state, target, input, runtime);
        Err(TaskProtocolConformanceServiceError::Internal(
            anyhow::anyhow!(
                "task-protocol conformance runtime was unexpectedly available on this platform"
            ),
        ))
    }
}

pub(crate) fn currentness(
    state: &AppState,
    admin_user_id: &str,
    registry_release_id: &str,
) -> Result<serde_json::Value, TaskProtocolConformanceServiceError> {
    validate_currentness(admin_user_id, registry_release_id)
        .map_err(TaskProtocolConformanceServiceError::Invalid)?;
    let runtime = external_pool_adapter_task_protocol_conformance_runtime()
        .map_err(TaskProtocolConformanceServiceError::Unavailable)?;
    if !state
        .store
        .external_pool_adapter_registry_release_exists(registry_release_id)
        .map_err(TaskProtocolConformanceServiceError::Internal)?
    {
        return Err(TaskProtocolConformanceServiceError::NotFound);
    }
    let output = state
        .store
        .external_pool_adapter_task_protocol_conformance_currentness(
            registry_release_id,
            Some(runtime.as_ref()),
        )
        .map_err(classify_store_error)?
        .ok_or(TaskProtocolConformanceServiceError::NotFound)?;
    redacted_json(output).map_err(TaskProtocolConformanceServiceError::Internal)
}

pub(crate) fn revoke(
    state: &AppState,
    admin_user_id: &str,
    registry_release_id: &str,
    run_receipt_id: &str,
    body: RevokeTaskProtocolConformanceRunBody,
) -> Result<serde_json::Value, TaskProtocolConformanceServiceError> {
    validate_revoke(admin_user_id, registry_release_id, run_receipt_id, &body)
        .map_err(TaskProtocolConformanceServiceError::Invalid)?;
    let _runtime = external_pool_adapter_task_protocol_conformance_runtime()
        .map_err(TaskProtocolConformanceServiceError::Unavailable)?;
    if !state
        .store
        .external_pool_adapter_registry_release_exists(registry_release_id)
        .map_err(TaskProtocolConformanceServiceError::Internal)?
        || !state
            .store
            .external_pool_adapter_task_protocol_conformance_run_exists(
                registry_release_id,
                run_receipt_id,
            )
            .map_err(classify_store_error)?
    {
        return Err(TaskProtocolConformanceServiceError::NotFound);
    }
    let output = state
        .store
        .revoke_external_pool_adapter_task_protocol_conformance_run(
            RevokeExternalPoolAdapterTaskProtocolConformanceRun {
                registry_release_id: registry_release_id.into(),
                run_receipt_id: run_receipt_id.into(),
                expected_run_receipt_digest: body.expected_run_receipt_digest,
                revoked_by_admin_user_id: admin_user_id.into(),
                reason: body.reason,
                idempotency_scope: idempotency_scope("revoke", admin_user_id),
                idempotency_key: body.idempotency_key,
                confirmation: TASK_PROTOCOL_CONFORMANCE_REVOCATION_CONFIRMATION.into(),
            },
        )
        .map_err(classify_store_error)?;
    redacted_json(output).map_err(TaskProtocolConformanceServiceError::Internal)
}

fn require_release_and_receipts(
    state: &AppState,
    registry_release_id: &str,
    body: &CreateTaskProtocolConformanceRunBody,
) -> Result<(), TaskProtocolConformanceServiceError> {
    let release_exists = state
        .store
        .external_pool_adapter_registry_release_exists(registry_release_id)
        .map_err(TaskProtocolConformanceServiceError::Internal)?;
    let sandbox_exists = state
        .store
        .external_pool_adapter_sandbox_reattestation_exists(
            &body.sandbox_reattestation_receipt_id,
            registry_release_id,
        )
        .map_err(TaskProtocolConformanceServiceError::Internal)?;
    let compatibility_exists = state
        .store
        .external_pool_adapter_runtime_compatibility_verification_exists(
            &body.runtime_compatibility_verification_receipt_id,
            registry_release_id,
        )
        .map_err(classify_runtime_compatibility_error)?;
    if !(release_exists && sandbox_exists && compatibility_exists) {
        return Err(TaskProtocolConformanceServiceError::NotFound);
    }
    Ok(())
}

fn classify_store_error(
    error: ExternalPoolAdapterTaskProtocolConformanceStoreError,
) -> TaskProtocolConformanceServiceError {
    match error {
        ExternalPoolAdapterTaskProtocolConformanceStoreError::Conflict(error) => {
            TaskProtocolConformanceServiceError::Conflict(error)
        }
        ExternalPoolAdapterTaskProtocolConformanceStoreError::Storage(error) => {
            TaskProtocolConformanceServiceError::Internal(error)
        }
    }
}

fn classify_runtime_compatibility_error(
    error: crate::store::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError,
) -> TaskProtocolConformanceServiceError {
    match error {
        crate::store::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError::Conflict(
            error,
        ) => TaskProtocolConformanceServiceError::Conflict(error),
        crate::store::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError::Storage(
            error,
        ) => TaskProtocolConformanceServiceError::Internal(error),
    }
}
