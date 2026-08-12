//! Administrator orchestration for V233 deterministic local artifact safety scanning.

use anyhow::Error as AnyError;
use serde::Deserialize;
use thiserror::Error;

use crate::{
    store::{
        CreateExternalPoolAdapterArtifactSecurityReceipt,
        ExternalPoolAdapterArtifactSecurityCurrentnessReceipt,
        ExternalPoolAdapterArtifactSecurityWriteReceipt,
    },
    types::AppState,
};

use super::{
    external_pool_adapter_artifact_package::inspect_external_pool_adapter_artifact_package,
    external_pool_adapter_artifact_security::{
        scan_external_pool_adapter_artifact_security, ARTIFACT_SECURITY_CONFIRMATION,
    },
    external_pool_adapter_artifact_source::{
        open_current_quarantined_artifact_bytes, require_current_quarantined_artifact_bytes,
        ExternalPoolAdapterArtifactSourceFsError,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScanArtifactSecurityBody {
    pub expected_admission_digest: String,
    pub expected_source_receipt_digest: String,
    pub expected_provenance_receipt_digest: String,
    pub expected_package_receipt_digest: String,
    pub idempotency_key: String,
    pub confirm_static_security_scan: bool,
}

#[derive(Debug, Error)]
pub(crate) enum ArtifactSecurityServiceError {
    #[error("external-pool Adapter Artifact security receipt was not found")]
    NotFound,
    #[error("external-pool Adapter Artifact static security policy rejected the package")]
    Rejected(#[source] AnyError),
    #[error("external-pool Adapter Artifact security lineage conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool Adapter Artifact security scan task failed")]
    Task(#[source] tokio::task::JoinError),
    #[error(transparent)]
    Filesystem(#[from] ExternalPoolAdapterArtifactSourceFsError),
}

pub(crate) async fn scan_for_admin(
    state: &AppState,
    admin_user_id: &str,
    admission_id: &str,
    body: ScanArtifactSecurityBody,
) -> Result<ExternalPoolAdapterArtifactSecurityWriteReceipt, ArtifactSecurityServiceError> {
    if !body.confirm_static_security_scan {
        return Err(ArtifactSecurityServiceError::Rejected(anyhow::anyhow!(
            "执行静态安全扫描前必须显式确认"
        )));
    }
    let target = state
        .store
        .external_pool_adapter_artifact_security_scan_target(
            admission_id,
            &body.expected_admission_digest,
            &body.expected_source_receipt_digest,
            &body.expected_provenance_receipt_digest,
            &body.expected_package_receipt_digest,
        )
        .map_err(classify_store_error)?;
    let artifact = open_current_quarantined_artifact_bytes(
        &state.data_dir,
        &target.archive_sha256,
        target.archive_size_bytes,
    )
    .await?;
    let target_for_scan = target.clone();
    let scanned = tokio::task::spawn_blocking(move || {
        let package = inspect_external_pool_adapter_artifact_package(
            artifact,
            &target_for_scan.package_expected(),
        )?;
        scan_external_pool_adapter_artifact_security(package, &target_for_scan)
    })
    .await
    .map_err(ArtifactSecurityServiceError::Task)?
    .map_err(ArtifactSecurityServiceError::Rejected)?;

    state
        .store
        .create_external_pool_adapter_artifact_security_receipt(
            CreateExternalPoolAdapterArtifactSecurityReceipt {
                expected: target,
                scanned_by_admin_user_id: admin_user_id.to_string(),
                confirmation: ARTIFACT_SECURITY_CONFIRMATION.to_string(),
                idempotency_scope: format!(
                    "external-pool-adapter-artifact-security:{admin_user_id}"
                ),
                idempotency_key: body.idempotency_key,
                scanned,
            },
        )
        .map_err(classify_store_error)
}

pub(crate) async fn currentness_for_admin(
    state: &AppState,
    admission_id: &str,
) -> Result<ExternalPoolAdapterArtifactSecurityCurrentnessReceipt, ArtifactSecurityServiceError> {
    let currentness = state
        .store
        .external_pool_adapter_artifact_security_currentness(admission_id)
        .map_err(classify_store_error)?
        .ok_or(ArtifactSecurityServiceError::NotFound)?;
    require_current_quarantined_artifact_bytes(
        &state.data_dir,
        &currentness.security.archive_sha256,
        currentness.security.archive_size_bytes,
    )
    .await?;
    Ok(currentness)
}

fn classify_store_error(error: AnyError) -> ArtifactSecurityServiceError {
    let text = format!("{error:#}");
    if text.contains("was not found") || text.contains("is absent") {
        ArtifactSecurityServiceError::NotFound
    } else {
        ArtifactSecurityServiceError::Conflict(error)
    }
}
