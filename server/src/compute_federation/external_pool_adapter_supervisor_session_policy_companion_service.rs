//! Owner/admin orchestration for inert V259 supervisor/session policy companions.

use anyhow::Error as AnyError;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::{
    compute_federation::{
        external_pool_adapter_runtime_launch_profile::RUNTIME_LAUNCH_PROFILE_STATUS,
        external_pool_adapter_supervisor_session_policy_companion::{
            SUPERVISOR_SESSION_COMPANION_ACTOR_PLATFORM_ADMIN,
            SUPERVISOR_SESSION_COMPANION_ACTOR_PROVIDER_OWNER,
            SUPERVISOR_SESSION_COMPANION_CONFIRMATION,
            SUPERVISOR_SESSION_COMPANION_REVOCATION_CONFIRMATION,
            SUPERVISOR_SESSION_COMPANION_STATUS,
        },
        external_pool_adapter_upstream_transport_target::UPSTREAM_TRANSPORT_TARGET_STATUS,
        provider::PROVIDER_STATUS_REGISTERING,
    },
    store::{
        CreateExternalPoolAdapterSupervisorSessionPolicyCompanion,
        RevokeExternalPoolAdapterSupervisorSessionPolicyCompanion,
    },
    types::AppState,
};

use super::external_pool_adapter_installation::{
    audit_external_pool_adapter_installation, ExternalPoolAdapterInstallationBinding,
    ExternalPoolAdapterInstallationFsError, PreparedExternalPoolAdapterInstallation,
};
use super::external_pool_adapter_supervisor_session_policy_companion_service_redaction::redacted_json;
use super::external_pool_adapter_supervisor_session_policy_companion_service_validation::*;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpectedSupervisorSessionPolicyCompanionPredecessor {
    pub companion_id: String,
    pub companion_digest: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateSupervisorSessionPolicyCompanionBody {
    pub expected_target_digest: String,
    pub expected_profile_digest: String,
    pub expected_candidate_digest: String,
    pub expected_provider_binding_digest: String,
    pub expected_supervisor_session_policy_digest: String,
    pub expected_predecessor: Option<ExpectedSupervisorSessionPolicyCompanionPredecessor>,
    pub idempotency_key: String,
    pub confirm_supervisor_session_policy_companion: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeSupervisorSessionPolicyCompanionBody {
    pub expected_companion_digest: String,
    pub expected_target_digest: String,
    pub expected_profile_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}

#[derive(Clone)]
pub(crate) enum SupervisorSessionPolicyCompanionActor {
    ProviderOwner(String),
    PlatformAdmin(String),
}

impl SupervisorSessionPolicyCompanionActor {
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::ProviderOwner(_) => SUPERVISOR_SESSION_COMPANION_ACTOR_PROVIDER_OWNER,
            Self::PlatformAdmin(_) => SUPERVISOR_SESSION_COMPANION_ACTOR_PLATFORM_ADMIN,
        }
    }

    pub(super) fn user_id(&self) -> &str {
        match self {
            Self::ProviderOwner(id) | Self::PlatformAdmin(id) => id,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum SupervisorSessionPolicyCompanionServiceError {
    #[error("external-pool supervisor/session policy-companion authority was not found")]
    NotFound,
    #[error("authenticated user cannot manage this external-pool Provider binding")]
    Forbidden,
    #[error("external-pool supervisor/session policy-companion request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool supervisor/session policy-companion authority conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool supervisor/session policy-companion filesystem audit task failed")]
    Task(#[source] tokio::task::JoinError),
    #[error("external-pool supervisor/session policy-companion filesystem audit failed")]
    Storage(#[source] ExternalPoolAdapterInstallationFsError),
}

pub(crate) fn policy_summary(
    state: &AppState,
    actor: SupervisorSessionPolicyCompanionActor,
    path: [&str; 4],
) -> Result<Value, SupervisorSessionPolicyCompanionServiceError> {
    validate_path(path, None)?;
    let target = target_target(state, path)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let output = state
        .store
        .external_pool_adapter_supervisor_session_policy_summary()
        .map_err(SupervisorSessionPolicyCompanionServiceError::Conflict)?;
    require_policy_inert(&output)?;
    redacted_json(output)
}

pub(crate) async fn create(
    state: &AppState,
    actor: SupervisorSessionPolicyCompanionActor,
    path: [&str; 4],
    body: CreateSupervisorSessionPolicyCompanionBody,
) -> Result<Value, SupervisorSessionPolicyCompanionServiceError> {
    validate_create(path, &body)?;
    let target = target_target(state, path)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let prepared = audit_prepared(state, target.installation_binding).await?;
    let (predecessor_companion_id, expected_predecessor_companion_digest) = body
        .expected_predecessor
        .map(|value| (Some(value.companion_id), Some(value.companion_digest)))
        .unwrap_or((None, None));
    let output = state
        .store
        .create_external_pool_adapter_supervisor_session_policy_companion(
            CreateExternalPoolAdapterSupervisorSessionPolicyCompanion {
                prepared,
                target_id: path[3].to_string(),
                expected_target_digest: body.expected_target_digest,
                expected_profile_digest: body.expected_profile_digest,
                expected_candidate_digest: body.expected_candidate_digest,
                expected_provider_binding_digest: body.expected_provider_binding_digest,
                expected_supervisor_session_policy_digest: body
                    .expected_supervisor_session_policy_digest,
                predecessor_companion_id,
                expected_predecessor_companion_digest,
                recorded_by_actor_kind: actor.kind().to_string(),
                recorded_by_actor_user_id: actor.user_id().to_string(),
                idempotency_scope: idempotency_scope("create", &actor),
                idempotency_key: body.idempotency_key,
                confirmation: SUPERVISOR_SESSION_COMPANION_CONFIRMATION.to_string(),
            },
        )
        .map_err(SupervisorSessionPolicyCompanionServiceError::Conflict)?;
    require_companion_inert(&output.companion)?;
    redacted_json(output)
}

pub(crate) async fn currentness(
    state: &AppState,
    actor: SupervisorSessionPolicyCompanionActor,
    path: [&str; 4],
    companion_id: &str,
) -> Result<Value, SupervisorSessionPolicyCompanionServiceError> {
    validate_path(path, Some(companion_id))?;
    let target = companion_target(state, path, companion_id)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let prepared = audit_prepared(state, target.installation_binding).await?;
    let output = state
        .store
        .external_pool_adapter_supervisor_session_policy_companion_currentness(
            companion_id,
            &target.companion_digest,
            prepared,
        )
        .map_err(SupervisorSessionPolicyCompanionServiceError::Conflict)?
        .ok_or(SupervisorSessionPolicyCompanionServiceError::NotFound)?;
    if output.current_status != SUPERVISOR_SESSION_COMPANION_STATUS
        || output.provider_status != PROVIDER_STATUS_REGISTERING
        || output.profile_status != RUNTIME_LAUNCH_PROFILE_STATUS
        || output.target_status != UPSTREAM_TRANSPORT_TARGET_STATUS
        || output.policy_status != "server_policy_current"
        || output.revocation_status != "unrevoked"
    {
        return Err(conflict(
            "supervisor/session policy-companion currentness is not inert",
        ));
    }
    require_companion_inert(&output.companion)?;
    require_currentness_inert(&output)?;
    redacted_json(output)
}

pub(crate) fn revoke(
    state: &AppState,
    actor: SupervisorSessionPolicyCompanionActor,
    path: [&str; 4],
    companion_id: &str,
    body: RevokeSupervisorSessionPolicyCompanionBody,
) -> Result<Value, SupervisorSessionPolicyCompanionServiceError> {
    validate_revoke(path, companion_id, &body)?;
    let target = companion_target(state, path, companion_id)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let output = state
        .store
        .revoke_external_pool_adapter_supervisor_session_policy_companion(
            RevokeExternalPoolAdapterSupervisorSessionPolicyCompanion {
                companion_id: companion_id.to_string(),
                expected_companion_digest: body.expected_companion_digest,
                expected_target_digest: body.expected_target_digest,
                expected_profile_digest: body.expected_profile_digest,
                revoked_by_actor_kind: actor.kind().to_string(),
                revoked_by_actor_user_id: actor.user_id().to_string(),
                reason: body.reason,
                idempotency_scope: idempotency_scope("revoke", &actor),
                idempotency_key: body.idempotency_key,
                confirmation: SUPERVISOR_SESSION_COMPANION_REVOCATION_CONFIRMATION.to_string(),
            },
        )
        .map_err(SupervisorSessionPolicyCompanionServiceError::Conflict)?;
    require_companion_inert(&output.companion)?;
    require_revocation_inert(&output.revocation)?;
    redacted_json(output)
}

fn target_target(
    state: &AppState,
    path: [&str; 4],
) -> Result<
    crate::store::ExternalPoolAdapterUpstreamTransportTargetAuditTarget,
    SupervisorSessionPolicyCompanionServiceError,
> {
    let target = state
        .store
        .external_pool_adapter_upstream_transport_target_audit_target(path[3])
        .map_err(SupervisorSessionPolicyCompanionServiceError::Conflict)?
        .ok_or(SupervisorSessionPolicyCompanionServiceError::NotFound)?;
    for (actual, expected) in [
        (&target.provider_binding_id, path[0]),
        (&target.candidate_id, path[1]),
        (&target.profile_id, path[2]),
        (&target.target_id, path[3]),
    ] {
        require_exact(actual, expected)?;
    }
    Ok(target)
}

fn companion_target(
    state: &AppState,
    path: [&str; 4],
    companion_id: &str,
) -> Result<
    crate::store::ExternalPoolAdapterSupervisorSessionPolicyCompanionAuditTarget,
    SupervisorSessionPolicyCompanionServiceError,
> {
    let target = state
        .store
        .external_pool_adapter_supervisor_session_policy_companion_audit_target(companion_id)
        .map_err(SupervisorSessionPolicyCompanionServiceError::Conflict)?
        .ok_or(SupervisorSessionPolicyCompanionServiceError::NotFound)?;
    for (actual, expected) in [
        (&target.provider_binding_id, path[0]),
        (&target.candidate_id, path[1]),
        (&target.profile_id, path[2]),
        (&target.target_id, path[3]),
        (&target.companion_id, companion_id),
    ] {
        require_exact(actual, expected)?;
    }
    Ok(target)
}

async fn audit_prepared(
    state: &AppState,
    binding: ExternalPoolAdapterInstallationBinding,
) -> Result<PreparedExternalPoolAdapterInstallation, SupervisorSessionPolicyCompanionServiceError> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        audit_external_pool_adapter_installation(&data_dir, binding)
    })
    .await
    .map_err(SupervisorSessionPolicyCompanionServiceError::Task)?
    .map_err(classify_filesystem_error)
}

fn authorize(
    actor: &SupervisorSessionPolicyCompanionActor,
    expected_owner: &str,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    if matches!(actor, SupervisorSessionPolicyCompanionActor::ProviderOwner(id) if id != expected_owner)
    {
        Err(SupervisorSessionPolicyCompanionServiceError::Forbidden)
    } else {
        Ok(())
    }
}
