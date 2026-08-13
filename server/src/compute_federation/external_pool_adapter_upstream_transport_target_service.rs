//! Owner/admin orchestration for inert V258 upstream transport targets.

use anyhow::Error as AnyError;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::{
    compute_federation::{
        external_pool_adapter_runtime_launch_profile::RUNTIME_LAUNCH_PROFILE_STATUS,
        external_pool_adapter_upstream_transport_target::{
            UPSTREAM_TRANSPORT_TARGET_ACTOR_PLATFORM_ADMIN,
            UPSTREAM_TRANSPORT_TARGET_ACTOR_PROVIDER_OWNER, UPSTREAM_TRANSPORT_TARGET_CONFIRMATION,
            UPSTREAM_TRANSPORT_TARGET_REVOCATION_CONFIRMATION, UPSTREAM_TRANSPORT_TARGET_STATUS,
        },
        provider::PROVIDER_STATUS_REGISTERING,
    },
    store::{
        CreateExternalPoolAdapterUpstreamTransportTarget,
        ExternalPoolAdapterUpstreamTransportTargetDraft,
        RevokeExternalPoolAdapterUpstreamTransportTarget,
    },
    types::AppState,
};

use super::external_pool_adapter_installation::{
    audit_external_pool_adapter_installation, ExternalPoolAdapterInstallationBinding,
    ExternalPoolAdapterInstallationFsError, PreparedExternalPoolAdapterInstallation,
};
use super::external_pool_adapter_upstream_transport_target_service_redaction::redacted_json;
use super::external_pool_adapter_upstream_transport_target_service_validation::*;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpectedUpstreamTransportTargetPredecessor {
    pub target_id: String,
    pub target_digest: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpstreamTransportTargetDraftBody {
    pub dns_hostname: String,
    pub port: u16,
    pub expected_tls_leaf_spki_sha256: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateUpstreamTransportTargetBody {
    pub expected_profile_digest: String,
    pub expected_candidate_digest: String,
    pub expected_provider_binding_digest: String,
    pub expected_target_policy_digest: String,
    pub draft: UpstreamTransportTargetDraftBody,
    pub expected_predecessor: Option<ExpectedUpstreamTransportTargetPredecessor>,
    pub idempotency_key: String,
    pub confirm_upstream_transport_target: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeUpstreamTransportTargetBody {
    pub expected_target_digest: String,
    pub expected_profile_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}

#[derive(Clone)]
pub(crate) enum UpstreamTransportTargetActor {
    ProviderOwner(String),
    PlatformAdmin(String),
}

impl UpstreamTransportTargetActor {
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::ProviderOwner(_) => UPSTREAM_TRANSPORT_TARGET_ACTOR_PROVIDER_OWNER,
            Self::PlatformAdmin(_) => UPSTREAM_TRANSPORT_TARGET_ACTOR_PLATFORM_ADMIN,
        }
    }

    pub(super) fn user_id(&self) -> &str {
        match self {
            Self::ProviderOwner(id) | Self::PlatformAdmin(id) => id,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum UpstreamTransportTargetServiceError {
    #[error("external-pool upstream transport-target authority was not found")]
    NotFound,
    #[error("authenticated user cannot manage this external-pool Provider binding")]
    Forbidden,
    #[error("external-pool upstream transport-target request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool upstream transport-target authority conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool upstream transport-target filesystem audit task failed")]
    Task(#[source] tokio::task::JoinError),
    #[error("external-pool upstream transport-target filesystem audit failed")]
    Storage(#[source] ExternalPoolAdapterInstallationFsError),
}

pub(crate) fn policy_summary(
    state: &AppState,
    actor: UpstreamTransportTargetActor,
    binding_id: &str,
    candidate_id: &str,
    profile_id: &str,
) -> Result<Value, UpstreamTransportTargetServiceError> {
    validate_path(binding_id, candidate_id, profile_id, None)?;
    let target = profile_target(state, binding_id, candidate_id, profile_id)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let output = state
        .store
        .external_pool_adapter_upstream_transport_target_policy_summary()
        .map_err(UpstreamTransportTargetServiceError::Conflict)?;
    require_policy_inert(&output)?;
    redacted_json(output)
}

pub(crate) async fn create(
    state: &AppState,
    actor: UpstreamTransportTargetActor,
    binding_id: &str,
    candidate_id: &str,
    profile_id: &str,
    body: CreateUpstreamTransportTargetBody,
) -> Result<Value, UpstreamTransportTargetServiceError> {
    validate_create(binding_id, candidate_id, profile_id, &body)?;
    let target = profile_target(state, binding_id, candidate_id, profile_id)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let prepared = audit_prepared(state, target.installation_binding).await?;
    let (predecessor_target_id, expected_predecessor_target_digest) = body
        .expected_predecessor
        .map(|value| (Some(value.target_id), Some(value.target_digest)))
        .unwrap_or((None, None));
    let output = state
        .store
        .create_external_pool_adapter_upstream_transport_target(
            CreateExternalPoolAdapterUpstreamTransportTarget {
                prepared,
                profile_id: profile_id.to_string(),
                expected_profile_digest: body.expected_profile_digest,
                expected_candidate_digest: body.expected_candidate_digest,
                expected_provider_binding_digest: body.expected_provider_binding_digest,
                expected_target_policy_digest: body.expected_target_policy_digest,
                target: ExternalPoolAdapterUpstreamTransportTargetDraft {
                    dns_hostname: body.draft.dns_hostname,
                    port: body.draft.port,
                    expected_tls_leaf_spki_sha256: body.draft.expected_tls_leaf_spki_sha256,
                },
                predecessor_target_id,
                expected_predecessor_target_digest,
                recorded_by_actor_kind: actor.kind().to_string(),
                recorded_by_actor_user_id: actor.user_id().to_string(),
                idempotency_scope: idempotency_scope("create", &actor),
                idempotency_key: body.idempotency_key,
                confirmation: UPSTREAM_TRANSPORT_TARGET_CONFIRMATION.to_string(),
            },
        )
        .map_err(UpstreamTransportTargetServiceError::Conflict)?;
    require_target_inert(&output.target)?;
    redacted_json(output)
}

pub(crate) async fn currentness(
    state: &AppState,
    actor: UpstreamTransportTargetActor,
    binding_id: &str,
    candidate_id: &str,
    profile_id: &str,
    target_id: &str,
) -> Result<Value, UpstreamTransportTargetServiceError> {
    validate_path(binding_id, candidate_id, profile_id, Some(target_id))?;
    let target = target_target(state, binding_id, candidate_id, profile_id, target_id)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let prepared = audit_prepared(state, target.installation_binding).await?;
    let output = state
        .store
        .external_pool_adapter_upstream_transport_target_currentness(
            target_id,
            &target.target_digest,
            prepared,
        )
        .map_err(UpstreamTransportTargetServiceError::Conflict)?
        .ok_or(UpstreamTransportTargetServiceError::NotFound)?;
    if output.current_status != UPSTREAM_TRANSPORT_TARGET_STATUS
        || output.provider_status != PROVIDER_STATUS_REGISTERING
        || output.profile_status != RUNTIME_LAUNCH_PROFILE_STATUS
        || output.target_policy_status != "server_policy_current"
        || output.revocation_status != "unrevoked"
        || output.broker_connect_ready
        || output.upstream_probe_observed
        || output.runtime_launch_ready
        || output.activation_ready
    {
        return Err(conflict(
            "upstream transport-target currentness is not inert",
        ));
    }
    require_target_inert(&output.target)?;
    redacted_json(output)
}

pub(crate) fn revoke(
    state: &AppState,
    actor: UpstreamTransportTargetActor,
    binding_id: &str,
    candidate_id: &str,
    profile_id: &str,
    target_id: &str,
    body: RevokeUpstreamTransportTargetBody,
) -> Result<Value, UpstreamTransportTargetServiceError> {
    validate_revoke(binding_id, candidate_id, profile_id, target_id, &body)?;
    let target = target_target(state, binding_id, candidate_id, profile_id, target_id)?;
    authorize(&actor, &target.provider_owner_account_id)?;
    let output = state
        .store
        .revoke_external_pool_adapter_upstream_transport_target(
            RevokeExternalPoolAdapterUpstreamTransportTarget {
                target_id: target_id.to_string(),
                expected_target_digest: body.expected_target_digest,
                expected_profile_digest: body.expected_profile_digest,
                revoked_by_actor_kind: actor.kind().to_string(),
                revoked_by_actor_user_id: actor.user_id().to_string(),
                reason: body.reason,
                idempotency_scope: idempotency_scope("revoke", &actor),
                idempotency_key: body.idempotency_key,
                confirmation: UPSTREAM_TRANSPORT_TARGET_REVOCATION_CONFIRMATION.to_string(),
            },
        )
        .map_err(UpstreamTransportTargetServiceError::Conflict)?;
    require_target_inert(&output.target)?;
    require_revocation_inert(&output.revocation)?;
    redacted_json(output)
}

fn profile_target(
    state: &AppState,
    binding_id: &str,
    candidate_id: &str,
    profile_id: &str,
) -> Result<
    crate::store::ExternalPoolAdapterRuntimeLaunchProfileAuditTarget,
    UpstreamTransportTargetServiceError,
> {
    let target = state
        .store
        .external_pool_adapter_runtime_launch_profile_audit_target(profile_id)
        .map_err(UpstreamTransportTargetServiceError::Conflict)?
        .ok_or(UpstreamTransportTargetServiceError::NotFound)?;
    require_exact(binding_id, &target.provider_binding_id)?;
    require_exact(candidate_id, &target.candidate_id)?;
    require_exact(profile_id, &target.profile_id)?;
    Ok(target)
}

fn target_target(
    state: &AppState,
    binding_id: &str,
    candidate_id: &str,
    profile_id: &str,
    target_id: &str,
) -> Result<
    crate::store::ExternalPoolAdapterUpstreamTransportTargetAuditTarget,
    UpstreamTransportTargetServiceError,
> {
    let target = state
        .store
        .external_pool_adapter_upstream_transport_target_audit_target(target_id)
        .map_err(UpstreamTransportTargetServiceError::Conflict)?
        .ok_or(UpstreamTransportTargetServiceError::NotFound)?;
    require_exact(binding_id, &target.provider_binding_id)?;
    require_exact(candidate_id, &target.candidate_id)?;
    require_exact(profile_id, &target.profile_id)?;
    require_exact(target_id, &target.target_id)?;
    Ok(target)
}

async fn audit_prepared(
    state: &AppState,
    binding: ExternalPoolAdapterInstallationBinding,
) -> Result<PreparedExternalPoolAdapterInstallation, UpstreamTransportTargetServiceError> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        audit_external_pool_adapter_installation(&data_dir, binding)
    })
    .await
    .map_err(UpstreamTransportTargetServiceError::Task)?
    .map_err(classify_filesystem_error)
}

fn authorize(
    actor: &UpstreamTransportTargetActor,
    expected_owner: &str,
) -> Result<(), UpstreamTransportTargetServiceError> {
    if matches!(actor, UpstreamTransportTargetActor::ProviderOwner(id) if id != expected_owner) {
        Err(UpstreamTransportTargetServiceError::Forbidden)
    } else {
        Ok(())
    }
}
