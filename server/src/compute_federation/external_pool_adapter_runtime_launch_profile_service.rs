//! Owner/admin orchestration for inert V255 runtime launch profiles.

use anyhow::Error as AnyError;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::{
    compute_federation::{
        external_pool_adapter_runtime_launch_profile::{
            RUNTIME_LAUNCH_PROFILE_ACTOR_PLATFORM_ADMIN,
            RUNTIME_LAUNCH_PROFILE_ACTOR_PROVIDER_OWNER, RUNTIME_LAUNCH_PROFILE_CONFIRMATION,
            RUNTIME_LAUNCH_PROFILE_EFFECT, RUNTIME_LAUNCH_PROFILE_NO_EFFECT,
            RUNTIME_LAUNCH_PROFILE_REVOCATION_CONFIRMATION,
            RUNTIME_LAUNCH_PROFILE_REVOCATION_EFFECT, RUNTIME_LAUNCH_PROFILE_STATUS,
        },
        provider::PROVIDER_STATUS_REGISTERING,
    },
    store::{
        CreateExternalPoolAdapterRuntimeLaunchProfile,
        RevokeExternalPoolAdapterRuntimeLaunchProfile,
    },
    types::AppState,
};

use super::external_pool_adapter_installation::{
    audit_external_pool_adapter_installation, ExternalPoolAdapterInstallationBinding,
    ExternalPoolAdapterInstallationFsError, PreparedExternalPoolAdapterInstallation,
};
use super::external_pool_adapter_runtime_launch_profile_service_redaction::redacted_json;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpectedRuntimeLaunchProfilePredecessor {
    pub profile_id: String,
    pub profile_digest: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateRuntimeLaunchProfileBody {
    pub expected_candidate_digest: String,
    pub expected_provider_binding_digest: String,
    pub expected_launch_policy_digest: String,
    pub expected_predecessor: Option<ExpectedRuntimeLaunchProfilePredecessor>,
    pub idempotency_key: String,
    pub confirm_runtime_launch_profile: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeRuntimeLaunchProfileBody {
    pub expected_profile_digest: String,
    pub expected_candidate_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}

#[derive(Clone)]
pub(crate) enum RuntimeLaunchProfileActor {
    ProviderOwner(String),
    PlatformAdmin(String),
}

impl RuntimeLaunchProfileActor {
    fn kind(&self) -> &'static str {
        match self {
            Self::ProviderOwner(_) => RUNTIME_LAUNCH_PROFILE_ACTOR_PROVIDER_OWNER,
            Self::PlatformAdmin(_) => RUNTIME_LAUNCH_PROFILE_ACTOR_PLATFORM_ADMIN,
        }
    }

    fn user_id(&self) -> &str {
        match self {
            Self::ProviderOwner(id) | Self::PlatformAdmin(id) => id,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum RuntimeLaunchProfileServiceError {
    #[error("external-pool runtime launch-profile authority was not found")]
    NotFound,
    #[error("authenticated user cannot manage this external-pool Provider binding")]
    Forbidden,
    #[error("external-pool runtime launch-profile request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool runtime launch-profile authority conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool runtime launch-profile filesystem audit task failed")]
    Task(#[source] tokio::task::JoinError),
    #[error("external-pool runtime launch-profile filesystem audit failed")]
    Storage(#[source] ExternalPoolAdapterInstallationFsError),
}

pub(crate) fn policy_summary(
    state: &AppState,
    actor: RuntimeLaunchProfileActor,
    provider_binding_id: &str,
    candidate_id: &str,
) -> Result<Value, RuntimeLaunchProfileServiceError> {
    validate_path(provider_binding_id, candidate_id, None)?;
    let target = candidate_target(state, provider_binding_id, candidate_id)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let output = state
        .store
        .external_pool_adapter_runtime_launch_policy_summary()
        .map_err(RuntimeLaunchProfileServiceError::Conflict)?;
    if output.profile_effect != RUNTIME_LAUNCH_PROFILE_EFFECT
        || output.adapter_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
        || output.runtime_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
        || output.usage_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
    {
        return Err(conflict("runtime launch policy changed inert effect"));
    }
    redacted_json(output)
}

pub(crate) async fn create(
    state: &AppState,
    actor: RuntimeLaunchProfileActor,
    provider_binding_id: &str,
    candidate_id: &str,
    body: CreateRuntimeLaunchProfileBody,
) -> Result<Value, RuntimeLaunchProfileServiceError> {
    validate_create(provider_binding_id, candidate_id, &body)?;
    let target = candidate_target(state, provider_binding_id, candidate_id)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let prepared = audit_prepared(state, target.installation_binding).await?;
    let (predecessor_profile_id, expected_predecessor_profile_digest) = body
        .expected_predecessor
        .map(|value| (Some(value.profile_id), Some(value.profile_digest)))
        .unwrap_or((None, None));
    let output = state
        .store
        .create_external_pool_adapter_runtime_launch_profile(
            CreateExternalPoolAdapterRuntimeLaunchProfile {
                prepared,
                candidate_id: candidate_id.to_string(),
                expected_candidate_digest: body.expected_candidate_digest,
                expected_provider_binding_digest: body.expected_provider_binding_digest,
                expected_launch_policy_digest: body.expected_launch_policy_digest,
                predecessor_profile_id,
                expected_predecessor_profile_digest,
                recorded_by_actor_kind: actor.kind().to_string(),
                recorded_by_actor_user_id: actor.user_id().to_string(),
                idempotency_scope: idempotency_scope("create", &actor),
                idempotency_key: body.idempotency_key,
                confirmation: RUNTIME_LAUNCH_PROFILE_CONFIRMATION.to_string(),
            },
        )
        .map_err(RuntimeLaunchProfileServiceError::Conflict)?;
    if output.profile.profile_status != RUNTIME_LAUNCH_PROFILE_STATUS
        || output.profile.adapter_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
        || output.profile.runtime_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
        || output.profile.usage_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
    {
        return Err(conflict("runtime launch profile changed its inert effect"));
    }
    redacted_json(output)
}

pub(crate) async fn currentness(
    state: &AppState,
    actor: RuntimeLaunchProfileActor,
    provider_binding_id: &str,
    candidate_id: &str,
    profile_id: &str,
) -> Result<Value, RuntimeLaunchProfileServiceError> {
    validate_path(provider_binding_id, candidate_id, Some(profile_id))?;
    let target = profile_target(state, provider_binding_id, candidate_id, profile_id)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let prepared = audit_prepared(state, target.installation_binding).await?;
    let output = state
        .store
        .external_pool_adapter_runtime_launch_profile_currentness(profile_id, prepared)
        .map_err(RuntimeLaunchProfileServiceError::Conflict)?
        .ok_or(RuntimeLaunchProfileServiceError::NotFound)?;
    if output.current_status != RUNTIME_LAUNCH_PROFILE_STATUS
        || output.provider_status != PROVIDER_STATUS_REGISTERING
        || output.runtime_launch_ready
        || output.profile.adapter_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
        || output.profile.runtime_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
        || output.profile.usage_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
    {
        return Err(conflict("runtime launch-profile currentness is not inert"));
    }
    redacted_json(output)
}

pub(crate) fn revoke(
    state: &AppState,
    actor: RuntimeLaunchProfileActor,
    provider_binding_id: &str,
    candidate_id: &str,
    profile_id: &str,
    body: RevokeRuntimeLaunchProfileBody,
) -> Result<Value, RuntimeLaunchProfileServiceError> {
    validate_revoke(provider_binding_id, candidate_id, profile_id, &body)?;
    let target = profile_target(state, provider_binding_id, candidate_id, profile_id)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let output = state
        .store
        .revoke_external_pool_adapter_runtime_launch_profile(
            RevokeExternalPoolAdapterRuntimeLaunchProfile {
                profile_id: profile_id.to_string(),
                expected_profile_digest: body.expected_profile_digest,
                expected_candidate_digest: body.expected_candidate_digest,
                revoked_by_actor_kind: actor.kind().to_string(),
                revoked_by_actor_user_id: actor.user_id().to_string(),
                reason: body.reason,
                idempotency_scope: idempotency_scope("revoke", &actor),
                idempotency_key: body.idempotency_key,
                confirmation: RUNTIME_LAUNCH_PROFILE_REVOCATION_CONFIRMATION.to_string(),
            },
        )
        .map_err(RuntimeLaunchProfileServiceError::Conflict)?;
    if output.revocation.revocation_effect != RUNTIME_LAUNCH_PROFILE_REVOCATION_EFFECT
        || output.revocation.adapter_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
        || output.revocation.runtime_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
        || output.revocation.usage_effect != RUNTIME_LAUNCH_PROFILE_NO_EFFECT
    {
        return Err(conflict(
            "runtime launch-profile revocation changed its inert effect",
        ));
    }
    redacted_json(output)
}

fn candidate_target(
    state: &AppState,
    provider_binding_id: &str,
    candidate_id: &str,
) -> Result<
    crate::store::ExternalPoolProviderActivationCandidateAuditTarget,
    RuntimeLaunchProfileServiceError,
> {
    let target = state
        .store
        .external_pool_provider_activation_candidate_audit_target(candidate_id)
        .map_err(RuntimeLaunchProfileServiceError::Conflict)?
        .ok_or(RuntimeLaunchProfileServiceError::NotFound)?;
    require_exact(provider_binding_id, &target.provider_binding_id)?;
    Ok(target)
}

fn profile_target(
    state: &AppState,
    provider_binding_id: &str,
    candidate_id: &str,
    profile_id: &str,
) -> Result<
    crate::store::ExternalPoolAdapterRuntimeLaunchProfileAuditTarget,
    RuntimeLaunchProfileServiceError,
> {
    let target = state
        .store
        .external_pool_adapter_runtime_launch_profile_audit_target(profile_id)
        .map_err(RuntimeLaunchProfileServiceError::Conflict)?
        .ok_or(RuntimeLaunchProfileServiceError::NotFound)?;
    require_exact(provider_binding_id, &target.provider_binding_id)?;
    require_exact(candidate_id, &target.candidate_id)?;
    require_exact(profile_id, &target.profile_id)?;
    Ok(target)
}

async fn audit_prepared(
    state: &AppState,
    binding: ExternalPoolAdapterInstallationBinding,
) -> Result<PreparedExternalPoolAdapterInstallation, RuntimeLaunchProfileServiceError> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        audit_external_pool_adapter_installation(&data_dir, binding)
    })
    .await
    .map_err(RuntimeLaunchProfileServiceError::Task)?
    .map_err(classify_filesystem_error)
}

fn authorize(
    actor: &RuntimeLaunchProfileActor,
    expected_owner: &str,
) -> Result<(), RuntimeLaunchProfileServiceError> {
    if matches!(actor, RuntimeLaunchProfileActor::ProviderOwner(id) if id != expected_owner) {
        Err(RuntimeLaunchProfileServiceError::Forbidden)
    } else {
        Ok(())
    }
}

fn validate_create(
    binding: &str,
    candidate: &str,
    body: &CreateRuntimeLaunchProfileBody,
) -> Result<(), RuntimeLaunchProfileServiceError> {
    validate_path(binding, candidate, None)?;
    if !body.confirm_runtime_launch_profile {
        return Err(invalid(
            "runtime launch profile requires explicit confirmation",
        ));
    }
    validate_digest(&body.expected_candidate_digest, "candidate digest")?;
    validate_digest(
        &body.expected_provider_binding_digest,
        "Provider binding digest",
    )?;
    validate_digest(&body.expected_launch_policy_digest, "launch policy digest")?;
    validate_identifier(&body.idempotency_key, 240, "idempotency key")?;
    if let Some(predecessor) = &body.expected_predecessor {
        validate_identifier(&predecessor.profile_id, 240, "predecessor profile ID")?;
        validate_digest(&predecessor.profile_digest, "predecessor profile digest")?;
    }
    Ok(())
}

fn validate_revoke(
    binding: &str,
    candidate: &str,
    profile: &str,
    body: &RevokeRuntimeLaunchProfileBody,
) -> Result<(), RuntimeLaunchProfileServiceError> {
    validate_path(binding, candidate, Some(profile))?;
    if !body.confirm_revocation {
        return Err(invalid(
            "runtime launch-profile revocation requires confirmation",
        ));
    }
    validate_digest(&body.expected_profile_digest, "profile digest")?;
    validate_digest(&body.expected_candidate_digest, "candidate digest")?;
    validate_identifier(&body.idempotency_key, 240, "idempotency key")?;
    if body.reason.trim() != body.reason
        || !(12..=500).contains(&body.reason.chars().count())
        || body.reason.chars().any(char::is_control)
    {
        return Err(invalid(
            "runtime launch-profile revocation reason is invalid",
        ));
    }
    Ok(())
}

fn validate_path(
    binding: &str,
    candidate: &str,
    profile: Option<&str>,
) -> Result<(), RuntimeLaunchProfileServiceError> {
    validate_identifier(binding, 240, "Provider binding ID")?;
    validate_identifier(candidate, 240, "activation candidate ID")?;
    if let Some(profile) = profile {
        validate_identifier(profile, 240, "runtime launch-profile ID")?;
    }
    Ok(())
}

fn validate_identifier(
    value: &str,
    maximum: usize,
    label: &'static str,
) -> Result<(), RuntimeLaunchProfileServiceError> {
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
) -> Result<(), RuntimeLaunchProfileServiceError> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if valid {
        Ok(())
    } else {
        Err(invalid(format!("{label} is invalid")))
    }
}

fn require_exact(actual: &str, expected: &str) -> Result<(), RuntimeLaunchProfileServiceError> {
    if actual == expected {
        Ok(())
    } else {
        Err(conflict(
            "runtime launch authority does not belong to the requested path",
        ))
    }
}

fn idempotency_scope(operation: &str, actor: &RuntimeLaunchProfileActor) -> String {
    format!(
        "v255:runtime-launch-profile:{operation}:{}:{}",
        actor.kind(),
        actor.user_id()
    )
}

fn classify_filesystem_error(
    error: ExternalPoolAdapterInstallationFsError,
) -> RuntimeLaunchProfileServiceError {
    match error {
        ExternalPoolAdapterInstallationFsError::Authority(_)
        | ExternalPoolAdapterInstallationFsError::InvalidContentAddress
        | ExternalPoolAdapterInstallationFsError::Package(_)
        | ExternalPoolAdapterInstallationFsError::Missing
        | ExternalPoolAdapterInstallationFsError::UnsafeTarget
        | ExternalPoolAdapterInstallationFsError::ContentDrift => {
            RuntimeLaunchProfileServiceError::Conflict(AnyError::new(error))
        }
        ExternalPoolAdapterInstallationFsError::Storage(_) => {
            RuntimeLaunchProfileServiceError::Storage(error)
        }
    }
}

fn invalid(message: impl Into<String>) -> RuntimeLaunchProfileServiceError {
    RuntimeLaunchProfileServiceError::Invalid(anyhow::anyhow!(message.into()))
}

fn conflict(message: impl Into<String>) -> RuntimeLaunchProfileServiceError {
    RuntimeLaunchProfileServiceError::Conflict(anyhow::anyhow!(message.into()))
}
