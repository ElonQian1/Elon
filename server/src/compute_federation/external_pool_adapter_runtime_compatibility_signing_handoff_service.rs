//! Authenticated V269 courier handoff for server-owned runtime compatibility evidence.

use std::sync::Arc;

use anyhow::Error as AnyError;
use serde::Serialize;
use thiserror::Error;

use crate::{
    compute_federation::{
        external_pool_adapter_runtime_compatibility_signing_handoff_runtime::{
            external_pool_adapter_runtime_compatibility_signing_handoff_runtime,
            ExternalPoolAdapterRuntimeCompatibilitySigningHandoffUnavailable,
        },
        external_pool_adapter_runtime_compatibility_verification::{
            ExternalPoolAdapterRuntimeCompatibilitySignerPayload,
            ExternalPoolAdapterRuntimeCompatibilitySigningHandoff,
            ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRecordBinding,
        },
    },
    types::AppState,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::{
    compute_federation::external_pool_adapter_installation::{
        audit_external_pool_adapter_installation, ExternalPoolAdapterInstallationFsError,
    },
    store::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError,
};

pub(crate) use super::external_pool_adapter_runtime_compatibility_signing_handoff_service_validation::RuntimeCompatibilitySigningHandoffBody;
use super::external_pool_adapter_runtime_compatibility_signing_handoff_service_validation::validate_signing_handoff;

#[derive(Serialize)]
pub(crate) struct RuntimeCompatibilitySigningHandoffResponse {
    pub schema: &'static str,
    pub record_binding: ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRecordBinding,
    pub signer_payload: ExternalPoolAdapterRuntimeCompatibilitySignerPayload,
    pub replayed: bool,
}

impl From<ExternalPoolAdapterRuntimeCompatibilitySigningHandoff>
    for RuntimeCompatibilitySigningHandoffResponse
{
    fn from(value: ExternalPoolAdapterRuntimeCompatibilitySigningHandoff) -> Self {
        Self {
            schema: value.schema,
            record_binding: value.record_binding,
            signer_payload: value.signer_payload,
            replayed: value.replayed,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum RuntimeCompatibilitySigningHandoffServiceError {
    #[error("external-pool Adapter runtime compatibility signing handoff was not found")]
    NotFound,
    #[error("external-pool Adapter runtime compatibility signing handoff request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter runtime compatibility signing handoff authority conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool Adapter runtime compatibility signing handoff is unavailable")]
    Unavailable(#[source] ExternalPoolAdapterRuntimeCompatibilitySigningHandoffUnavailable),
    #[error("external-pool Adapter runtime compatibility signing handoff failed internally")]
    Internal(#[source] AnyError),
}

pub(crate) async fn signing_handoff_for_admin(
    state: Arc<AppState>,
    admin_user_id: &str,
    registry_release_id: &str,
    challenge_id: &str,
    body: RuntimeCompatibilitySigningHandoffBody,
) -> Result<
    RuntimeCompatibilitySigningHandoffResponse,
    RuntimeCompatibilitySigningHandoffServiceError,
> {
    validate_signing_handoff(admin_user_id, registry_release_id, challenge_id, &body)?;
    let runtime = external_pool_adapter_runtime_compatibility_signing_handoff_runtime()
        .map_err(RuntimeCompatibilitySigningHandoffServiceError::Unavailable)?;

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        run_signing_handoff(state, runtime, registry_release_id, challenge_id, body).await
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = (state, runtime, registry_release_id, challenge_id, body);
        Err(RuntimeCompatibilitySigningHandoffServiceError::Internal(
            anyhow::anyhow!("signing handoff runtime was unexpectedly available"),
        ))
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
async fn run_signing_handoff(
    state: Arc<AppState>,
    runtime: Arc<
        super::external_pool_adapter_runtime_compatibility_signing_handoff_runtime::ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRuntime,
    >,
    registry_release_id: &str,
    challenge_id: &str,
    body: RuntimeCompatibilitySigningHandoffBody,
) -> Result<
    RuntimeCompatibilitySigningHandoffResponse,
    RuntimeCompatibilitySigningHandoffServiceError,
> {
    if !state
        .store
        .external_pool_adapter_registry_release_exists(registry_release_id)
        .map_err(RuntimeCompatibilitySigningHandoffServiceError::Internal)?
    {
        return Err(RuntimeCompatibilitySigningHandoffServiceError::NotFound);
    }
    if !state
        .store
        .external_pool_adapter_runtime_compatibility_verification_challenge_exists(
            challenge_id,
            registry_release_id,
        )
        .map_err(classify_store_error)?
    {
        return Err(RuntimeCompatibilitySigningHandoffServiceError::NotFound);
    }
    let target = state
        .store
        .external_pool_adapter_registry_provider_binding_audit_target(&body.provider_binding_id)
        .map_err(RuntimeCompatibilitySigningHandoffServiceError::Internal)?
        .ok_or(RuntimeCompatibilitySigningHandoffServiceError::NotFound)?;
    if target.installation_receipt_id != body.expected_installation_receipt_id {
        return Err(RuntimeCompatibilitySigningHandoffServiceError::NotFound);
    }
    if target.installation_receipt_digest != body.expected_installation_receipt_digest {
        return Err(RuntimeCompatibilitySigningHandoffServiceError::Conflict(
            anyhow::anyhow!("installation receipt digest is not exact"),
        ));
    }

    let data_dir = state.data_dir.clone();
    let expected_registry_release_id = registry_release_id.to_string();
    let challenge_id = challenge_id.to_string();
    let output = tokio::task::spawn_blocking(move || {
        let prepared =
            audit_external_pool_adapter_installation(&data_dir, target.installation_binding)
                .map_err(classify_filesystem_error)?;
        state
            .store
            .run_external_pool_adapter_runtime_compatibility_signing_handoff(
                &expected_registry_release_id,
                &challenge_id,
                &body.expected_challenge_digest,
                &body.provider_binding_id,
                &body.expected_provider_binding_digest,
                &body.expected_installation_receipt_id,
                &body.expected_installation_receipt_digest,
                prepared,
                runtime.cgroup_parent(),
            )
            .map_err(classify_store_error)
    })
    .await
    .map_err(|error| {
        RuntimeCompatibilitySigningHandoffServiceError::Internal(AnyError::new(error))
    })??;
    Ok(output.into())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn classify_filesystem_error(
    error: ExternalPoolAdapterInstallationFsError,
) -> RuntimeCompatibilitySigningHandoffServiceError {
    match error {
        ExternalPoolAdapterInstallationFsError::Authority(_)
        | ExternalPoolAdapterInstallationFsError::InvalidContentAddress
        | ExternalPoolAdapterInstallationFsError::Package(_)
        | ExternalPoolAdapterInstallationFsError::Missing
        | ExternalPoolAdapterInstallationFsError::UnsafeTarget
        | ExternalPoolAdapterInstallationFsError::ContentDrift => {
            RuntimeCompatibilitySigningHandoffServiceError::Conflict(AnyError::new(error))
        }
        ExternalPoolAdapterInstallationFsError::Storage(_) => {
            RuntimeCompatibilitySigningHandoffServiceError::Internal(AnyError::new(error))
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn classify_store_error(
    error: ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError,
) -> RuntimeCompatibilitySigningHandoffServiceError {
    match error {
        ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError::Conflict(error) => {
            RuntimeCompatibilitySigningHandoffServiceError::Conflict(error)
        }
        ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError::Storage(error) => {
            RuntimeCompatibilitySigningHandoffServiceError::Internal(error)
        }
    }
}
