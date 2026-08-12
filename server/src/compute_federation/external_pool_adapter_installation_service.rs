//! Administrator orchestration for pathless, inert external-pool Adapter installation.

use anyhow::{bail, Error as AnyError, Result as AnyResult};
use serde::Deserialize;
use thiserror::Error;

use crate::{
    store::{
        ExternalPoolAdapterInstallationCurrentness,
        ExternalPoolAdapterInstallationTerminalWriteReceipt,
        ExternalPoolAdapterInstallationWriteReceipt, InstallExternalPoolAdapter,
        RevokeExternalPoolAdapterInstallation,
    },
    types::AppState,
};

use super::{
    external_pool_adapter_artifact_source::open_current_quarantined_artifact_bytes,
    external_pool_adapter_installation::{
        audit_external_pool_adapter_installation, prepare_external_pool_adapter_installation,
        ExternalPoolAdapterInstallationFsError, INSTALLATION_CONFIRMATION,
        INSTALLATION_REVOCATION_CONFIRMATION,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InstallExternalPoolAdapterBody {
    pub adoption_receipt_id: String,
    pub expected_adoption_receipt_digest: String,
    pub expected_package_receipt_digest: String,
    pub expected_source_receipt_digest: String,
    pub idempotency_key: String,
    pub confirm_installed_inert: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeExternalPoolAdapterInstallationBody {
    pub expected_installation_receipt_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}

#[derive(Debug, Error)]
pub(crate) enum AdapterInstallationServiceError {
    #[error("external-pool Adapter installation authority was not found")]
    NotFound,
    #[error("external-pool Adapter installation request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter installation authority conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool Adapter installation task failed")]
    Task(#[source] tokio::task::JoinError),
    #[error("external-pool Adapter installation storage failed")]
    Storage(#[source] ExternalPoolAdapterInstallationFsError),
}

pub(crate) async fn install_for_admin(
    state: &AppState,
    admin_user_id: &str,
    body: InstallExternalPoolAdapterBody,
) -> Result<ExternalPoolAdapterInstallationWriteReceipt, AdapterInstallationServiceError> {
    validate_body(admin_user_id, &body)?;
    let idempotency_scope = format!("v246:install:{admin_user_id}");
    if let Some(binding) = state
        .store
        .external_pool_adapter_installation_replay_target(&idempotency_scope, &body.idempotency_key)
        .map_err(AdapterInstallationServiceError::Conflict)?
    {
        if binding.adoption_receipt_id != body.adoption_receipt_id
            || binding.adoption_receipt_digest != body.expected_adoption_receipt_digest
            || binding.package_receipt_digest != body.expected_package_receipt_digest
            || binding.source_receipt_digest != body.expected_source_receipt_digest
        {
            return Err(AdapterInstallationServiceError::Conflict(anyhow::anyhow!(
                "Adapter installation idempotency replay request drifted"
            )));
        }
        let data_dir = state.data_dir.clone();
        let prepared = tokio::task::spawn_blocking(move || {
            audit_external_pool_adapter_installation(&data_dir, binding)
        })
        .await
        .map_err(AdapterInstallationServiceError::Task)?
        .map_err(classify_filesystem_error)?;
        return state
            .store
            .install_external_pool_adapter(InstallExternalPoolAdapter {
                prepared,
                expected_adoption_receipt_digest: body.expected_adoption_receipt_digest,
                expected_package_receipt_digest: body.expected_package_receipt_digest,
                expected_source_receipt_digest: body.expected_source_receipt_digest,
                installed_by_admin_user_id: admin_user_id.to_string(),
                confirmation: INSTALLATION_CONFIRMATION.to_string(),
                idempotency_scope,
                idempotency_key: body.idempotency_key,
            })
            .map_err(AdapterInstallationServiceError::Conflict);
    }
    let target = state
        .store
        .external_pool_adapter_installation_target(
            &body.adoption_receipt_id,
            &body.expected_adoption_receipt_digest,
            &body.expected_package_receipt_digest,
            &body.expected_source_receipt_digest,
        )
        .map_err(AdapterInstallationServiceError::Conflict)?
        .ok_or(AdapterInstallationServiceError::NotFound)?;
    let archive_sha256 = target.package_receipt.package.archive_sha256.clone();
    let archive_size_bytes = target.package_receipt.package.archive_size_bytes;
    let artifact = open_current_quarantined_artifact_bytes(
        &state.data_dir,
        &archive_sha256,
        archive_size_bytes,
    )
    .await
    .map_err(|error| AdapterInstallationServiceError::Conflict(anyhow::Error::new(error)))?;
    let data_dir = state.data_dir.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        prepare_external_pool_adapter_installation(&data_dir, artifact, target)
    })
    .await
    .map_err(AdapterInstallationServiceError::Task)?
    .map_err(classify_filesystem_error)?;

    state
        .store
        .install_external_pool_adapter(InstallExternalPoolAdapter {
            prepared,
            expected_adoption_receipt_digest: body.expected_adoption_receipt_digest,
            expected_package_receipt_digest: body.expected_package_receipt_digest,
            expected_source_receipt_digest: body.expected_source_receipt_digest,
            installed_by_admin_user_id: admin_user_id.to_string(),
            confirmation: INSTALLATION_CONFIRMATION.to_string(),
            idempotency_scope,
            idempotency_key: body.idempotency_key,
        })
        .map_err(AdapterInstallationServiceError::Conflict)
}

pub(crate) fn revoke_for_admin(
    state: &AppState,
    admin_user_id: &str,
    installation_receipt_id: &str,
    body: RevokeExternalPoolAdapterInstallationBody,
) -> Result<ExternalPoolAdapterInstallationTerminalWriteReceipt, AdapterInstallationServiceError> {
    if !body.confirm_revocation {
        return Err(invalid(
            "Adapter installation revocation requires explicit confirmation",
        ));
    }
    validate_identifier(admin_user_id, 200)?;
    validate_identifier(installation_receipt_id, 200)?;
    validate_identifier(&body.reason, 1000)?;
    validate_identifier(&body.idempotency_key, 240)?;
    validate_digest(&body.expected_installation_receipt_digest)?;
    state
        .store
        .external_pool_adapter_installation_audit_target(installation_receipt_id)
        .map_err(AdapterInstallationServiceError::Conflict)?
        .ok_or(AdapterInstallationServiceError::NotFound)?;

    state
        .store
        .revoke_external_pool_adapter_installation(RevokeExternalPoolAdapterInstallation {
            installation_receipt_id: installation_receipt_id.to_string(),
            expected_installation_receipt_digest: body.expected_installation_receipt_digest,
            revoked_by_admin_user_id: admin_user_id.to_string(),
            reason: body.reason,
            confirmation: INSTALLATION_REVOCATION_CONFIRMATION.to_string(),
            idempotency_scope: revocation_idempotency_scope(admin_user_id),
            idempotency_key: body.idempotency_key,
        })
        .map_err(AdapterInstallationServiceError::Conflict)
}

pub(crate) async fn currentness_for_admin(
    state: &AppState,
    installation_receipt_id: &str,
) -> Result<ExternalPoolAdapterInstallationCurrentness, AdapterInstallationServiceError> {
    validate_identifier(installation_receipt_id, 200)?;
    let binding = state
        .store
        .external_pool_adapter_installation_audit_target(installation_receipt_id)
        .map_err(AdapterInstallationServiceError::Conflict)?
        .ok_or(AdapterInstallationServiceError::NotFound)?;
    let data_dir = state.data_dir.clone();
    let _prepared = tokio::task::spawn_blocking(move || {
        audit_external_pool_adapter_installation(&data_dir, binding)
    })
    .await
    .map_err(AdapterInstallationServiceError::Task)?
    .map_err(classify_filesystem_error)?;
    let currentness = state
        .store
        .external_pool_adapter_installation_currentness(installation_receipt_id)
        .map_err(AdapterInstallationServiceError::Conflict)?
        .ok_or(AdapterInstallationServiceError::NotFound)?;
    Ok(currentness)
}

fn validate_body(
    admin_user_id: &str,
    body: &InstallExternalPoolAdapterBody,
) -> Result<(), AdapterInstallationServiceError> {
    if !body.confirm_installed_inert {
        return Err(invalid(
            "inert Adapter installation requires explicit confirmation",
        ));
    }
    validate_identifier(admin_user_id, 200)?;
    validate_identifier(&body.adoption_receipt_id, 200)?;
    validate_identifier(&body.idempotency_key, 160)?;
    for digest in [
        &body.expected_adoption_receipt_digest,
        &body.expected_package_receipt_digest,
        &body.expected_source_receipt_digest,
    ] {
        validate_digest(digest)?;
    }
    Ok(())
}

fn validate_identifier(value: &str, maximum: usize) -> Result<(), AdapterInstallationServiceError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        return Err(invalid("Adapter installation identifier is invalid"));
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), AdapterInstallationServiceError> {
    let result: AnyResult<()> = (|| {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("Adapter installation digest is invalid");
        }
        Ok(())
    })();
    result.map_err(AdapterInstallationServiceError::Invalid)
}

fn classify_filesystem_error(
    error: ExternalPoolAdapterInstallationFsError,
) -> AdapterInstallationServiceError {
    match error {
        ExternalPoolAdapterInstallationFsError::Authority(_)
        | ExternalPoolAdapterInstallationFsError::InvalidContentAddress
        | ExternalPoolAdapterInstallationFsError::Package(_)
        | ExternalPoolAdapterInstallationFsError::Missing
        | ExternalPoolAdapterInstallationFsError::UnsafeTarget
        | ExternalPoolAdapterInstallationFsError::ContentDrift => {
            AdapterInstallationServiceError::Conflict(anyhow::Error::new(error))
        }
        ExternalPoolAdapterInstallationFsError::Storage(_) => {
            AdapterInstallationServiceError::Storage(error)
        }
    }
}

fn invalid(message: &'static str) -> AdapterInstallationServiceError {
    AdapterInstallationServiceError::Invalid(anyhow::anyhow!(message))
}

fn revocation_idempotency_scope(admin_user_id: &str) -> String {
    format!("v247:installation-revoke:{admin_user_id}")
}

#[cfg(test)]
mod tests {
    use super::revocation_idempotency_scope;

    #[test]
    fn installation_revocation_scope_fits_the_sqlite_contract() {
        let maximum_actor = "a".repeat(200);
        assert!(revocation_idempotency_scope(&maximum_actor).chars().count() <= 240);
    }
}
