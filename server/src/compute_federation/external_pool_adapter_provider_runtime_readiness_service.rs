//! Owner/admin orchestration for V270 Provider runtime-readiness receipts.

use std::sync::Arc;

use anyhow::Error as AnyError;
use thiserror::Error;

use crate::{
    compute_federation::external_pool_adapter_provider_runtime_readiness::{
        CreateProviderRuntimeReadinessReceiptBody, RevokeProviderRuntimeReadinessReceiptBody,
        PROVIDER_RUNTIME_READINESS_ACTOR_PLATFORM_ADMIN,
        PROVIDER_RUNTIME_READINESS_ACTOR_PROVIDER_OWNER, PROVIDER_RUNTIME_READINESS_CONFIRMATION,
        PROVIDER_RUNTIME_READINESS_REVOCATION_CONFIRMATION,
    },
    store::{
        external_pool_adapter_provider_runtime_readiness_runtime,
        CreateExternalPoolAdapterProviderRuntimeReadiness,
        ExternalPoolAdapterProviderRuntimeReadinessStoreError,
        ExternalPoolAdapterProviderRuntimeReadinessUnavailable,
        RevokeExternalPoolAdapterProviderRuntimeReadiness,
    },
    types::AppState,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::compute_federation::external_pool_adapter_installation::audit_external_pool_adapter_installation;

use super::{
    external_pool_adapter_provider_runtime_readiness_service_redaction::redacted_json,
    external_pool_adapter_provider_runtime_readiness_service_validation::{
        idempotency_scope, require_exact, validate_create, validate_currentness, validate_revoke,
    },
};

#[derive(Clone)]
pub(crate) enum ProviderRuntimeReadinessActor {
    ProviderOwner(String),
    PlatformAdmin(String),
}

impl ProviderRuntimeReadinessActor {
    fn kind(&self) -> &'static str {
        match self {
            Self::ProviderOwner(_) => PROVIDER_RUNTIME_READINESS_ACTOR_PROVIDER_OWNER,
            Self::PlatformAdmin(_) => PROVIDER_RUNTIME_READINESS_ACTOR_PLATFORM_ADMIN,
        }
    }

    fn user_id(&self) -> &str {
        match self {
            Self::ProviderOwner(id) | Self::PlatformAdmin(id) => id,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum ProviderRuntimeReadinessServiceError {
    #[error("external-pool Provider runtime-readiness authority was not found")]
    NotFound,
    #[error("authenticated user cannot manage this external-pool Provider binding")]
    Forbidden,
    #[error("external-pool Provider runtime-readiness request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Provider runtime-readiness authority conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool Provider runtime-readiness trigger is unavailable")]
    Unavailable(#[source] ExternalPoolAdapterProviderRuntimeReadinessUnavailable),
    #[error("external-pool Provider runtime-readiness task failed")]
    Task(#[source] tokio::task::JoinError),
    #[error("external-pool Provider runtime-readiness failed internally")]
    Internal(#[source] AnyError),
}

pub(crate) async fn create(
    state: Arc<AppState>,
    actor: ProviderRuntimeReadinessActor,
    path: [&str; 5],
    body: CreateProviderRuntimeReadinessReceiptBody,
) -> Result<serde_json::Value, ProviderRuntimeReadinessServiceError> {
    validate_create(path, &body).map_err(ProviderRuntimeReadinessServiceError::Invalid)?;
    if !matches!(&actor, ProviderRuntimeReadinessActor::PlatformAdmin(_)) {
        return Err(ProviderRuntimeReadinessServiceError::Forbidden);
    }
    let runtime = external_pool_adapter_provider_runtime_readiness_runtime()
        .map_err(ProviderRuntimeReadinessServiceError::Unavailable)?;
    authorize_binding(&state, &actor, path)?;
    let target = exact_companion_target(&state, &actor, path)?;
    let (predecessor_readiness_receipt_id, expected_predecessor_readiness_receipt_digest) = body
        .expected_predecessor
        .map(|value| {
            (
                Some(value.readiness_receipt_id),
                Some(value.readiness_receipt_digest),
            )
        })
        .unwrap_or((None, None));
    let input = CreateExternalPoolAdapterProviderRuntimeReadiness {
        provider_binding_id: path[0].into(),
        expected_provider_binding_digest: body.expected_provider_binding_digest,
        expected_installation_receipt_id: body.expected_installation_receipt_id,
        expected_installation_receipt_digest: body.expected_installation_receipt_digest,
        candidate_id: path[1].into(),
        expected_candidate_digest: body.expected_candidate_digest,
        profile_id: path[2].into(),
        expected_profile_digest: body.expected_profile_digest,
        target_id: path[3].into(),
        expected_target_digest: body.expected_target_digest,
        companion_id: path[4].into(),
        expected_companion_digest: body.expected_companion_digest,
        runtime_compatibility_verification_receipt_id: body
            .runtime_compatibility_verification_receipt_id,
        expected_runtime_compatibility_verification_receipt_digest: body
            .expected_runtime_compatibility_verification_receipt_digest,
        predecessor_readiness_receipt_id,
        expected_predecessor_readiness_receipt_digest,
        recorded_by_actor_kind: actor.kind().into(),
        recorded_by_actor_user_id: actor.user_id().into(),
        idempotency_scope: idempotency_scope("create", actor.kind(), actor.user_id()),
        idempotency_key: body.idempotency_key,
        confirmation: PROVIDER_RUNTIME_READINESS_CONFIRMATION.into(),
    };

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let data_dir = state.data_dir.clone();
        let installation_binding = target.installation_binding;
        let handle = tokio::runtime::Handle::current();
        let output = tokio::task::spawn_blocking(move || {
            let mut reopen_prepared = || {
                audit_external_pool_adapter_installation(&data_dir, installation_binding.clone())
            };
            handle.block_on(
                state
                    .store
                    .create_external_pool_adapter_provider_runtime_readiness(
                        input,
                        &mut reopen_prepared,
                        &runtime,
                    ),
            )
        })
        .await
        .map_err(ProviderRuntimeReadinessServiceError::Task)?
        .map_err(classify_store_error)?;
        return redacted_json(output).map_err(ProviderRuntimeReadinessServiceError::Internal);
    }

    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = (state, target, input, runtime);
        Err(ProviderRuntimeReadinessServiceError::Internal(
            anyhow::anyhow!(
                "Provider runtime-readiness custody was unexpectedly available on this platform"
            ),
        ))
    }
}

pub(crate) fn currentness(
    state: &AppState,
    actor: ProviderRuntimeReadinessActor,
    path: [&str; 5],
    readiness_receipt_id: &str,
) -> Result<serde_json::Value, ProviderRuntimeReadinessServiceError> {
    validate_currentness(path, readiness_receipt_id)
        .map_err(ProviderRuntimeReadinessServiceError::Invalid)?;
    authorize_binding(state, &actor, path)?;
    exact_companion_target(state, &actor, path)?;
    let runtime = external_pool_adapter_provider_runtime_readiness_runtime().ok();
    let output = state
        .store
        .external_pool_adapter_provider_runtime_readiness_currentness(
            path[0],
            path[1],
            path[2],
            path[3],
            path[4],
            readiness_receipt_id,
            runtime.as_deref(),
        )
        .map_err(classify_store_error)?
        .ok_or(ProviderRuntimeReadinessServiceError::NotFound)?;
    redacted_json(output).map_err(ProviderRuntimeReadinessServiceError::Internal)
}

pub(crate) fn revoke(
    state: &AppState,
    actor: ProviderRuntimeReadinessActor,
    path: [&str; 5],
    readiness_receipt_id: &str,
    body: RevokeProviderRuntimeReadinessReceiptBody,
) -> Result<serde_json::Value, ProviderRuntimeReadinessServiceError> {
    validate_revoke(path, readiness_receipt_id, &body)
        .map_err(ProviderRuntimeReadinessServiceError::Invalid)?;
    authorize_binding(state, &actor, path)?;
    exact_companion_target(state, &actor, path)?;
    if state
        .store
        .external_pool_adapter_provider_runtime_readiness_currentness(
            path[0],
            path[1],
            path[2],
            path[3],
            path[4],
            readiness_receipt_id,
            None,
        )
        .map_err(classify_store_error)?
        .is_none()
    {
        return Err(ProviderRuntimeReadinessServiceError::NotFound);
    }
    let output = state
        .store
        .revoke_external_pool_adapter_provider_runtime_readiness(
            RevokeExternalPoolAdapterProviderRuntimeReadiness {
                provider_binding_id: path[0].into(),
                candidate_id: path[1].into(),
                profile_id: path[2].into(),
                target_id: path[3].into(),
                companion_id: path[4].into(),
                readiness_receipt_id: readiness_receipt_id.into(),
                expected_readiness_receipt_digest: body.expected_readiness_receipt_digest,
                revoked_by_actor_kind: actor.kind().into(),
                revoked_by_actor_user_id: actor.user_id().into(),
                reason: body.reason,
                idempotency_scope: idempotency_scope("revoke", actor.kind(), actor.user_id()),
                idempotency_key: body.idempotency_key,
                confirmation: PROVIDER_RUNTIME_READINESS_REVOCATION_CONFIRMATION.into(),
            },
        )
        .map_err(classify_store_error)?;
    redacted_json(output).map_err(ProviderRuntimeReadinessServiceError::Internal)
}

fn authorize_binding(
    state: &AppState,
    actor: &ProviderRuntimeReadinessActor,
    path: [&str; 5],
) -> Result<(), ProviderRuntimeReadinessServiceError> {
    let target = state
        .store
        .external_pool_provider_activation_candidate_audit_target(path[1])
        .map_err(ProviderRuntimeReadinessServiceError::Internal)?
        .ok_or(ProviderRuntimeReadinessServiceError::NotFound)?;
    if matches!(actor, ProviderRuntimeReadinessActor::ProviderOwner(id) if id != &target.provider_owner_account_id)
    {
        return Err(ProviderRuntimeReadinessServiceError::NotFound);
    }
    require_exact(&target.candidate_id, path[1], "activation candidate")
        .and_then(|_| require_exact(&target.provider_binding_id, path[0], "Provider binding"))
        .map_err(ProviderRuntimeReadinessServiceError::Conflict)?;
    Ok(())
}

fn exact_companion_target(
    state: &AppState,
    actor: &ProviderRuntimeReadinessActor,
    path: [&str; 5],
) -> Result<
    crate::store::ExternalPoolAdapterSupervisorSessionPolicyCompanionAuditTarget,
    ProviderRuntimeReadinessServiceError,
> {
    let target = state
        .store
        .external_pool_adapter_supervisor_session_policy_companion_audit_target(path[4])
        .map_err(ProviderRuntimeReadinessServiceError::Internal)?
        .ok_or(ProviderRuntimeReadinessServiceError::NotFound)?;
    for (actual, expected, authority) in [
        (&target.provider_binding_id, path[0], "Provider binding"),
        (&target.candidate_id, path[1], "activation candidate"),
        (&target.profile_id, path[2], "runtime launch profile"),
        (&target.target_id, path[3], "upstream transport target"),
        (
            &target.companion_id,
            path[4],
            "supervisor/session policy companion",
        ),
    ] {
        if let Err(error) = require_exact(actual, expected, authority) {
            return Err(match actor {
                ProviderRuntimeReadinessActor::ProviderOwner(_) => {
                    ProviderRuntimeReadinessServiceError::NotFound
                }
                ProviderRuntimeReadinessActor::PlatformAdmin(_) => {
                    ProviderRuntimeReadinessServiceError::Conflict(error)
                }
            });
        }
    }
    Ok(target)
}

fn classify_store_error(
    error: ExternalPoolAdapterProviderRuntimeReadinessStoreError,
) -> ProviderRuntimeReadinessServiceError {
    match error {
        ExternalPoolAdapterProviderRuntimeReadinessStoreError::Conflict(error) => {
            ProviderRuntimeReadinessServiceError::Conflict(error)
        }
        ExternalPoolAdapterProviderRuntimeReadinessStoreError::Storage(error) => {
            ProviderRuntimeReadinessServiceError::Internal(error)
        }
    }
}
