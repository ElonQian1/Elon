//! Administrator orchestration for V249 Provider-neutral release registry bindings.

use anyhow::{bail, Error as AnyError, Result as AnyResult};
use serde::Deserialize;
use thiserror::Error;

use crate::{
    store::{
        ExternalPoolAdapterRegistryProviderBindingCurrentness,
        ExternalPoolAdapterRegistryWriteReceipt, RegisterExternalPoolAdapterInstalledInstance,
    },
    types::AppState,
};

use super::{
    external_pool_adapter_installation::{
        audit_external_pool_adapter_installation, ExternalPoolAdapterInstallationFsError,
    },
    external_pool_adapter_registry::REGISTRY_BINDING_CONFIRMATION,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterExternalPoolAdapterRegistryBindingBody {
    pub installation_receipt_id: String,
    pub expected_installation_receipt_digest: String,
    pub idempotency_key: String,
    pub confirm_registry_binding: bool,
}

#[derive(Debug, Error)]
pub(crate) enum AdapterRegistryServiceError {
    #[error("external-pool Adapter registry binding was not found")]
    NotFound,
    #[error("external-pool Adapter registry binding request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter registry authority conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool Adapter registry filesystem audit task failed")]
    Task(#[source] tokio::task::JoinError),
    #[error("external-pool Adapter registry filesystem audit failed")]
    Storage(#[source] ExternalPoolAdapterInstallationFsError),
}

pub(crate) async fn register_for_admin(
    state: &AppState,
    admin_user_id: &str,
    body: RegisterExternalPoolAdapterRegistryBindingBody,
) -> Result<ExternalPoolAdapterRegistryWriteReceipt, AdapterRegistryServiceError> {
    validate_body(admin_user_id, &body)?;
    let idempotency_scope = format!("v249:registry-bind:{admin_user_id}");
    let target = if let Some(target) = state
        .store
        .external_pool_adapter_registry_replay_target(&idempotency_scope, &body.idempotency_key)
        .map_err(AdapterRegistryServiceError::Conflict)?
    {
        if target.installation_receipt_id != body.installation_receipt_id
            || target.installation_receipt_digest != body.expected_installation_receipt_digest
        {
            return Err(AdapterRegistryServiceError::Conflict(anyhow::anyhow!(
                "Adapter registry idempotency replay request drifted"
            )));
        }
        target
    } else {
        state
            .store
            .external_pool_adapter_registry_fresh_target(
                &body.installation_receipt_id,
                &body.expected_installation_receipt_digest,
            )
            .map_err(AdapterRegistryServiceError::Conflict)?
            .ok_or(AdapterRegistryServiceError::NotFound)?
    };

    let data_dir = state.data_dir.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        audit_external_pool_adapter_installation(&data_dir, target.installation_binding)
    })
    .await
    .map_err(AdapterRegistryServiceError::Task)?
    .map_err(classify_filesystem_error)?;

    state
        .store
        .register_external_pool_adapter_installed_instance(
            RegisterExternalPoolAdapterInstalledInstance {
                prepared,
                expected_installation_receipt_id: body.installation_receipt_id,
                expected_installation_receipt_digest: body.expected_installation_receipt_digest,
                bound_by_admin_user_id: admin_user_id.to_string(),
                idempotency_scope,
                idempotency_key: body.idempotency_key,
                confirmation: REGISTRY_BINDING_CONFIRMATION.to_string(),
            },
        )
        .map_err(AdapterRegistryServiceError::Conflict)
}

pub(crate) async fn currentness_for_admin(
    state: &AppState,
    provider_binding_id: &str,
) -> Result<ExternalPoolAdapterRegistryProviderBindingCurrentness, AdapterRegistryServiceError> {
    validate_identifier(provider_binding_id, 200)?;
    let target = state
        .store
        .external_pool_adapter_registry_provider_binding_audit_target(provider_binding_id)
        .map_err(AdapterRegistryServiceError::Conflict)?
        .ok_or(AdapterRegistryServiceError::NotFound)?;
    let data_dir = state.data_dir.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        audit_external_pool_adapter_installation(&data_dir, target.installation_binding)
    })
    .await
    .map_err(AdapterRegistryServiceError::Task)?
    .map_err(classify_filesystem_error)?;
    state
        .store
        .external_pool_adapter_registry_provider_binding_currentness(provider_binding_id, prepared)
        .map_err(AdapterRegistryServiceError::Conflict)?
        .ok_or(AdapterRegistryServiceError::NotFound)
}

fn validate_body(
    admin_user_id: &str,
    body: &RegisterExternalPoolAdapterRegistryBindingBody,
) -> Result<(), AdapterRegistryServiceError> {
    if !body.confirm_registry_binding {
        return Err(invalid(
            "Adapter registry binding requires explicit confirmation",
        ));
    }
    validate_identifier(admin_user_id, 200)?;
    validate_identifier(&body.installation_receipt_id, 200)?;
    validate_identifier(&body.idempotency_key, 200)?;
    validate_digest(&body.expected_installation_receipt_digest)
}

fn validate_identifier(value: &str, maximum: usize) -> Result<(), AdapterRegistryServiceError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        return Err(invalid("Adapter registry identifier is invalid"));
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), AdapterRegistryServiceError> {
    let result: AnyResult<()> = (|| {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("Adapter registry digest is invalid");
        }
        Ok(())
    })();
    result.map_err(AdapterRegistryServiceError::Invalid)
}

fn classify_filesystem_error(
    error: ExternalPoolAdapterInstallationFsError,
) -> AdapterRegistryServiceError {
    match error {
        ExternalPoolAdapterInstallationFsError::Authority(_)
        | ExternalPoolAdapterInstallationFsError::InvalidContentAddress
        | ExternalPoolAdapterInstallationFsError::Package(_)
        | ExternalPoolAdapterInstallationFsError::Missing
        | ExternalPoolAdapterInstallationFsError::UnsafeTarget
        | ExternalPoolAdapterInstallationFsError::ContentDrift => {
            AdapterRegistryServiceError::Conflict(anyhow::Error::new(error))
        }
        ExternalPoolAdapterInstallationFsError::Storage(_) => {
            AdapterRegistryServiceError::Storage(error)
        }
    }
}

fn invalid(message: &'static str) -> AdapterRegistryServiceError {
    AdapterRegistryServiceError::Invalid(anyhow::anyhow!(message))
}
