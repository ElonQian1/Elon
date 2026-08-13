//! Owner/admin orchestration for inert V254 external-pool activation candidates.

use anyhow::Error as AnyError;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::{
    compute_federation::external_pool_provider_activation_candidate::{
        ACTIVATION_CANDIDATE_CONFIRMATION, ACTIVATION_DELEGATION_REVOCATION_CONFIRMATION,
    },
    store::{
        CreateExternalPoolProviderActivationCandidate,
        GetCurrentExternalPoolProviderActivationPreflight,
        RevokeExternalPoolProviderActivationDelegation,
    },
    types::AppState,
};

use super::external_pool_adapter_installation::{
    audit_external_pool_adapter_installation, ExternalPoolAdapterInstallationBinding,
    ExternalPoolAdapterInstallationFsError, PreparedExternalPoolAdapterInstallation,
};
use super::external_pool_provider_activation_candidate_service_redaction::redacted_json;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateActivationCandidateBody {
    pub expected_provider_binding_digest: String,
    pub expected_registry_release_digest: String,
    pub idempotency_key: String,
    pub confirm_activation_candidate: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeActivationDelegationBody {
    pub expected_delegation_digest: String,
    pub expected_candidate_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivationPreflightQuery {
    pub expected_candidate_digest: String,
    pub vulnerability_reattestation_receipt_id: String,
    pub expected_vulnerability_reattestation_receipt_digest: String,
    pub sandbox_reattestation_receipt_id: String,
    pub expected_sandbox_reattestation_receipt_digest: String,
    pub credential_reattestation_receipt_id: String,
    pub expected_credential_reattestation_receipt_digest: String,
}

#[derive(Clone, Copy)]
pub(crate) enum ActivationReadActor<'a> {
    ProviderOwner(&'a str),
    PlatformAdmin,
}

#[derive(Debug, Error)]
pub(crate) enum ActivationCandidateServiceError {
    #[error("external-pool activation candidate authority was not found")]
    NotFound,
    #[error("authenticated user does not own this external-pool Provider binding")]
    Forbidden,
    #[error("external-pool activation candidate request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool activation candidate authority conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool activation candidate filesystem audit task failed")]
    Task(#[source] tokio::task::JoinError),
    #[error("external-pool activation candidate filesystem audit failed")]
    Storage(#[source] ExternalPoolAdapterInstallationFsError),
}

pub(crate) async fn create_for_owner(
    state: &AppState,
    owner_user_id: &str,
    provider_binding_id: &str,
    body: CreateActivationCandidateBody,
) -> Result<Value, ActivationCandidateServiceError> {
    validate_create(owner_user_id, provider_binding_id, &body)?;
    let target = state
        .store
        .external_pool_adapter_registry_provider_binding_audit_target(provider_binding_id)
        .map_err(ActivationCandidateServiceError::Conflict)?
        .ok_or(ActivationCandidateServiceError::NotFound)?;
    require_owner(
        owner_user_id,
        &target.installation_binding.provider_owner_account_id,
    )?;
    let prepared = audit_prepared(state, target.installation_binding).await?;
    let output = state
        .store
        .create_external_pool_provider_activation_candidate(
            CreateExternalPoolProviderActivationCandidate {
                prepared,
                provider_binding_id: provider_binding_id.to_string(),
                expected_provider_binding_digest: body.expected_provider_binding_digest,
                expected_registry_release_digest: body.expected_registry_release_digest,
                issued_by_owner_user_id: owner_user_id.to_string(),
                idempotency_scope: format!("v254:activation-candidate:{owner_user_id}"),
                idempotency_key: body.idempotency_key,
                confirmation: ACTIVATION_CANDIDATE_CONFIRMATION.to_string(),
            },
        )
        .map_err(ActivationCandidateServiceError::Conflict)?;
    redacted_json(output)
}

pub(crate) async fn currentness(
    state: &AppState,
    actor: ActivationReadActor<'_>,
    provider_binding_id: &str,
    candidate_id: &str,
) -> Result<Value, ActivationCandidateServiceError> {
    validate_identifier(provider_binding_id, 200, "Provider binding ID")?;
    validate_identifier(candidate_id, 200, "activation candidate ID")?;
    let target = candidate_target(state, provider_binding_id, candidate_id)?;
    authorize_read(actor, &target.provider_owner_account_id)?;
    let prepared = audit_prepared(state, target.installation_binding).await?;
    let output = state
        .store
        .external_pool_provider_activation_candidate_currentness(candidate_id, prepared)
        .map_err(ActivationCandidateServiceError::Conflict)?
        .ok_or(ActivationCandidateServiceError::NotFound)?;
    if output.current_status != "candidate_current_not_activation_ready"
        || output.activation_closure_status != "activation_closure_not_implemented"
        || output.activation_ready
    {
        return Err(ActivationCandidateServiceError::Conflict(anyhow::anyhow!(
            "activation candidate is historical or its fail-closed closure changed"
        )));
    }
    redacted_json(output)
}

pub(crate) async fn preflight(
    state: &AppState,
    actor: ActivationReadActor<'_>,
    provider_binding_id: &str,
    candidate_id: &str,
    query: ActivationPreflightQuery,
) -> Result<Value, ActivationCandidateServiceError> {
    validate_preflight(provider_binding_id, candidate_id, &query)?;
    let target = candidate_target(state, provider_binding_id, candidate_id)?;
    authorize_read(actor, &target.provider_owner_account_id)?;
    let prepared = audit_prepared(state, target.installation_binding).await?;
    let output = state
        .store
        .current_external_pool_provider_activation_preflight(
            GetCurrentExternalPoolProviderActivationPreflight {
                prepared,
                candidate_id: candidate_id.to_string(),
                expected_candidate_digest: query.expected_candidate_digest,
                vulnerability_reattestation_receipt_id: query
                    .vulnerability_reattestation_receipt_id,
                expected_vulnerability_reattestation_receipt_digest: query
                    .expected_vulnerability_reattestation_receipt_digest,
                sandbox_reattestation_receipt_id: query.sandbox_reattestation_receipt_id,
                expected_sandbox_reattestation_receipt_digest: query
                    .expected_sandbox_reattestation_receipt_digest,
                credential_reattestation_receipt_id: query.credential_reattestation_receipt_id,
                expected_credential_reattestation_receipt_digest: query
                    .expected_credential_reattestation_receipt_digest,
            },
        )
        .map_err(ActivationCandidateServiceError::Conflict)?
        .ok_or(ActivationCandidateServiceError::NotFound)?;
    if output.inputs_status != "inputs_current"
        || output.activation_closure_status != "activation_closure_not_implemented"
        || output.activation_ready
    {
        return Err(ActivationCandidateServiceError::Conflict(anyhow::anyhow!(
            "activation preflight did not preserve the fail-closed closure"
        )));
    }
    redacted_json(output)
}

pub(crate) async fn revoke_for_owner(
    state: &AppState,
    owner_user_id: &str,
    provider_binding_id: &str,
    delegation_id: &str,
    body: RevokeActivationDelegationBody,
) -> Result<Value, ActivationCandidateServiceError> {
    validate_revoke(owner_user_id, provider_binding_id, delegation_id, &body)?;
    let target = state
        .store
        .external_pool_provider_activation_delegation_audit_target(delegation_id)
        .map_err(ActivationCandidateServiceError::Conflict)?
        .ok_or(ActivationCandidateServiceError::NotFound)?;
    require_path(provider_binding_id, &target.provider_binding_id)?;
    require_owner(owner_user_id, &target.provider_owner_account_id)?;
    let output = state
        .store
        .revoke_external_pool_provider_activation_delegation(
            RevokeExternalPoolProviderActivationDelegation {
                delegation_id: delegation_id.to_string(),
                expected_delegation_digest: body.expected_delegation_digest,
                expected_candidate_digest: body.expected_candidate_digest,
                revoked_by_owner_user_id: owner_user_id.to_string(),
                reason: body.reason,
                idempotency_scope: format!("v254:activation-delegation-revoke:{owner_user_id}"),
                idempotency_key: body.idempotency_key,
                confirmation: ACTIVATION_DELEGATION_REVOCATION_CONFIRMATION.to_string(),
            },
        )
        .map_err(ActivationCandidateServiceError::Conflict)?;
    redacted_json(output)
}

fn candidate_target(
    state: &AppState,
    provider_binding_id: &str,
    candidate_id: &str,
) -> Result<
    crate::store::ExternalPoolProviderActivationCandidateAuditTarget,
    ActivationCandidateServiceError,
> {
    let target = state
        .store
        .external_pool_provider_activation_candidate_audit_target(candidate_id)
        .map_err(ActivationCandidateServiceError::Conflict)?
        .ok_or(ActivationCandidateServiceError::NotFound)?;
    require_path(provider_binding_id, &target.provider_binding_id)?;
    Ok(target)
}

async fn audit_prepared(
    state: &AppState,
    binding: ExternalPoolAdapterInstallationBinding,
) -> Result<PreparedExternalPoolAdapterInstallation, ActivationCandidateServiceError> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        audit_external_pool_adapter_installation(&data_dir, binding)
    })
    .await
    .map_err(ActivationCandidateServiceError::Task)?
    .map_err(classify_filesystem_error)
}

fn authorize_read(
    actor: ActivationReadActor<'_>,
    expected_owner: &str,
) -> Result<(), ActivationCandidateServiceError> {
    match actor {
        ActivationReadActor::ProviderOwner(user_id) => require_owner(user_id, expected_owner),
        ActivationReadActor::PlatformAdmin => Ok(()),
    }
}

fn require_owner(user_id: &str, expected: &str) -> Result<(), ActivationCandidateServiceError> {
    if user_id == expected {
        Ok(())
    } else {
        Err(ActivationCandidateServiceError::Forbidden)
    }
}

fn require_path(actual: &str, expected: &str) -> Result<(), ActivationCandidateServiceError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ActivationCandidateServiceError::Conflict(anyhow::anyhow!(
            "activation authority does not belong to the Provider binding in the path"
        )))
    }
}

fn validate_create(
    owner: &str,
    binding: &str,
    body: &CreateActivationCandidateBody,
) -> Result<(), ActivationCandidateServiceError> {
    if !body.confirm_activation_candidate {
        return Err(invalid(
            "activation candidate requires explicit confirmation",
        ));
    }
    validate_identifier(owner, 200, "owner user ID")?;
    validate_identifier(binding, 200, "Provider binding ID")?;
    validate_identifier(&body.idempotency_key, 240, "idempotency key")?;
    validate_digest(
        &body.expected_provider_binding_digest,
        "Provider binding digest",
    )?;
    validate_digest(
        &body.expected_registry_release_digest,
        "registry release digest",
    )
}

fn validate_revoke(
    owner: &str,
    binding: &str,
    delegation: &str,
    body: &RevokeActivationDelegationBody,
) -> Result<(), ActivationCandidateServiceError> {
    if !body.confirm_revocation {
        return Err(invalid(
            "activation delegation revocation requires confirmation",
        ));
    }
    for (value, maximum, label) in [
        (owner, 200, "owner user ID"),
        (binding, 200, "Provider binding ID"),
        (delegation, 200, "activation delegation ID"),
        (body.idempotency_key.as_str(), 240, "idempotency key"),
    ] {
        validate_identifier(value, maximum, label)?;
    }
    validate_digest(&body.expected_delegation_digest, "delegation digest")?;
    validate_digest(&body.expected_candidate_digest, "candidate digest")?;
    if body.reason.trim() != body.reason
        || !(12..=500).contains(&body.reason.chars().count())
        || body.reason.chars().any(char::is_control)
    {
        return Err(invalid(
            "activation delegation revocation reason is invalid",
        ));
    }
    Ok(())
}

fn validate_preflight(
    binding: &str,
    candidate: &str,
    query: &ActivationPreflightQuery,
) -> Result<(), ActivationCandidateServiceError> {
    validate_identifier(binding, 200, "Provider binding ID")?;
    validate_identifier(candidate, 200, "activation candidate ID")?;
    for (value, label) in [
        (
            &query.vulnerability_reattestation_receipt_id,
            "vulnerability receipt ID",
        ),
        (
            &query.sandbox_reattestation_receipt_id,
            "sandbox receipt ID",
        ),
        (
            &query.credential_reattestation_receipt_id,
            "credential receipt ID",
        ),
    ] {
        validate_identifier(value, 200, label)?;
    }
    for (value, label) in [
        (&query.expected_candidate_digest, "candidate digest"),
        (
            &query.expected_vulnerability_reattestation_receipt_digest,
            "vulnerability receipt digest",
        ),
        (
            &query.expected_sandbox_reattestation_receipt_digest,
            "sandbox receipt digest",
        ),
        (
            &query.expected_credential_reattestation_receipt_digest,
            "credential receipt digest",
        ),
    ] {
        validate_digest(value, label)?;
    }
    Ok(())
}

fn validate_identifier(
    value: &str,
    maximum: usize,
    label: &'static str,
) -> Result<(), ActivationCandidateServiceError> {
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
) -> Result<(), ActivationCandidateServiceError> {
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

fn classify_filesystem_error(
    error: ExternalPoolAdapterInstallationFsError,
) -> ActivationCandidateServiceError {
    match error {
        ExternalPoolAdapterInstallationFsError::Authority(_)
        | ExternalPoolAdapterInstallationFsError::InvalidContentAddress
        | ExternalPoolAdapterInstallationFsError::Package(_)
        | ExternalPoolAdapterInstallationFsError::Missing
        | ExternalPoolAdapterInstallationFsError::UnsafeTarget
        | ExternalPoolAdapterInstallationFsError::ContentDrift => {
            ActivationCandidateServiceError::Conflict(AnyError::new(error))
        }
        ExternalPoolAdapterInstallationFsError::Storage(_) => {
            ActivationCandidateServiceError::Storage(error)
        }
    }
}

fn invalid(message: impl Into<String>) -> ActivationCandidateServiceError {
    ActivationCandidateServiceError::Invalid(anyhow::anyhow!(message.into()))
}
