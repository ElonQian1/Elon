//! Administrator orchestration for bounded, pathless V232 static package inspection.

use anyhow::Error as AnyError;
use serde::Deserialize;
use thiserror::Error;

use crate::{
    store::{
        CreateExternalPoolAdapterArtifactPackageReceipt,
        ExternalPoolAdapterArtifactPackageCurrentnessReceipt,
        ExternalPoolAdapterArtifactPackageWriteReceipt,
    },
    types::AppState,
};

use super::{
    external_pool_adapter_artifact_package::{
        inspect_external_pool_adapter_artifact_package, ARTIFACT_PACKAGE_CONFIRMATION,
    },
    external_pool_adapter_artifact_source::{
        open_current_quarantined_artifact_bytes, require_current_quarantined_artifact_bytes,
        ExternalPoolAdapterArtifactSourceFsError,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectArtifactPackageBody {
    pub expected_admission_digest: String,
    pub expected_source_receipt_digest: String,
    pub expected_provenance_receipt_digest: String,
    pub idempotency_key: String,
    pub confirm_package_inspection: bool,
}

#[derive(Debug, Error)]
pub(crate) enum ArtifactPackageServiceError {
    #[error("external-pool Adapter Artifact package was not found")]
    NotFound,
    #[error("external-pool Adapter Artifact package is invalid")]
    InvalidPackage(#[source] AnyError),
    #[error("external-pool Adapter Artifact package lineage conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool Adapter Artifact package inspection task failed")]
    Task(#[source] tokio::task::JoinError),
    #[error(transparent)]
    Filesystem(#[from] ExternalPoolAdapterArtifactSourceFsError),
}

pub(crate) async fn inspect_for_admin(
    state: &AppState,
    admin_user_id: &str,
    admission_id: &str,
    body: InspectArtifactPackageBody,
) -> Result<ExternalPoolAdapterArtifactPackageWriteReceipt, ArtifactPackageServiceError> {
    if !body.confirm_package_inspection {
        return Err(ArtifactPackageServiceError::InvalidPackage(
            anyhow::anyhow!("检查静态包前必须显式确认"),
        ));
    }
    let target = state
        .store
        .external_pool_adapter_artifact_package_inspection_target(
            admission_id,
            &body.expected_admission_digest,
            &body.expected_source_receipt_digest,
            &body.expected_provenance_receipt_digest,
        )
        .map_err(classify_store_error)?;
    let artifact = open_current_quarantined_artifact_bytes(
        &state.data_dir,
        &target.artifact_sha256,
        target.artifact_size_bytes,
    )
    .await?;
    let inspected = tokio::task::spawn_blocking(move || {
        inspect_external_pool_adapter_artifact_package(artifact, &target.expected())
    })
    .await
    .map_err(ArtifactPackageServiceError::Task)?
    .map_err(ArtifactPackageServiceError::InvalidPackage)?;

    state
        .store
        .create_external_pool_adapter_artifact_package_receipt(
            CreateExternalPoolAdapterArtifactPackageReceipt {
                expected_admission_id: admission_id.to_string(),
                expected_admission_digest: body.expected_admission_digest,
                expected_source_receipt_digest: body.expected_source_receipt_digest,
                expected_provenance_receipt_digest: body.expected_provenance_receipt_digest,
                inspected_by_admin_user_id: admin_user_id.to_string(),
                confirmation: ARTIFACT_PACKAGE_CONFIRMATION.to_string(),
                idempotency_scope: operation_scope(admin_user_id),
                idempotency_key: body.idempotency_key,
                inspected,
            },
        )
        .map_err(classify_store_error)
}

pub(crate) async fn currentness_for_admin(
    state: &AppState,
    admission_id: &str,
) -> Result<ExternalPoolAdapterArtifactPackageCurrentnessReceipt, ArtifactPackageServiceError> {
    let currentness = state
        .store
        .external_pool_adapter_artifact_package_currentness(admission_id)
        .map_err(classify_store_error)?
        .ok_or(ArtifactPackageServiceError::NotFound)?;
    require_current_quarantined_artifact_bytes(
        &state.data_dir,
        &currentness.package.archive_sha256,
        currentness.package.archive_size_bytes,
    )
    .await?;
    Ok(currentness)
}

fn classify_store_error(error: AnyError) -> ArtifactPackageServiceError {
    let text = format!("{error:#}");
    if text.contains("was not found") || text.contains("is absent") {
        ArtifactPackageServiceError::NotFound
    } else {
        ArtifactPackageServiceError::Conflict(error)
    }
}

fn operation_scope(admin_user_id: &str) -> String {
    format!("external-pool-adapter-artifact-package:{admin_user_id}")
}
