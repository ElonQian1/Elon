//! Administrator-only orchestration for server-local external-pool Adapter byte quarantine.

use anyhow::Error as AnyError;
use axum::body::Body;
use thiserror::Error;

use crate::{
    store::{
        ExternalPoolAdapterArtifactSourceReceipt, RecordExternalPoolAdapterArtifactSource,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_CONFIRMATION,
    },
    types::AppState,
};

use super::external_pool_adapter_artifact_source::{
    intake_quarantined_artifact_bytes, require_current_quarantined_artifact_bytes,
    ExternalPoolAdapterArtifactSourceFsError,
};

pub(crate) struct PutExternalPoolAdapterArtifactSource {
    pub idempotency_key: String,
    pub expected_admission_digest: String,
    pub intake_confirmation: String,
    pub body: Body,
}

#[derive(Debug, Error)]
pub(crate) enum ExternalPoolAdapterArtifactSourceServiceError {
    #[error("external-pool Adapter artifact source receipt was not found")]
    NotFound,
    #[error("external-pool Adapter artifact source request conflicts with immutable lineage")]
    Conflict(#[source] AnyError),
    #[error(transparent)]
    Filesystem(#[from] ExternalPoolAdapterArtifactSourceFsError),
}

pub(crate) async fn put_for_admin(
    state: &AppState,
    admin_user_id: &str,
    admission_id: &str,
    input: PutExternalPoolAdapterArtifactSource,
) -> Result<ExternalPoolAdapterArtifactSourceReceipt, ExternalPoolAdapterArtifactSourceServiceError>
{
    // Receipt replay must prove the old blob is healthy before this request body can reach the CAS
    // writer. A missing or drifted blob is historical corruption, never an invitation to repair.
    if let Some(existing) = state
        .store
        .external_pool_adapter_artifact_source_for_admission(admission_id)
        .map_err(ExternalPoolAdapterArtifactSourceServiceError::Conflict)?
    {
        require_current_quarantined_artifact_bytes(
            &state.data_dir,
            existing.content_address_digest(),
            existing.artifact_size_bytes(),
        )
        .await?;
        if existing.admission_digest != input.expected_admission_digest {
            return Err(ExternalPoolAdapterArtifactSourceServiceError::Conflict(
                anyhow::anyhow!("artifact source admission digest mismatch"),
            ));
        }
    }

    let authority = state
        .store
        .external_pool_adapter_artifact_intake_authority(
            admission_id,
            &input.expected_admission_digest,
        )
        .map_err(ExternalPoolAdapterArtifactSourceServiceError::Conflict)?
        .ok_or(ExternalPoolAdapterArtifactSourceServiceError::NotFound)?;
    if authority.admission_id() != admission_id
        || authority.admission_digest() != input.expected_admission_digest
    {
        return Err(ExternalPoolAdapterArtifactSourceServiceError::Conflict(
            anyhow::anyhow!("artifact source intake authority mismatch"),
        ));
    }

    let artifact = intake_quarantined_artifact_bytes(
        &state.data_dir,
        authority.declared_implementation_sha256(),
        input.body,
    )
    .await?;
    state
        .store
        .record_external_pool_adapter_artifact_source(RecordExternalPoolAdapterArtifactSource {
            admission_id: admission_id.to_string(),
            expected_admission_digest: input.expected_admission_digest,
            recorded_by_admin_user_id: admin_user_id.to_string(),
            intake_confirmation: input.intake_confirmation,
            idempotency_scope: operation_scope(admin_user_id),
            idempotency_key: input.idempotency_key,
            artifact,
        })
        .map_err(ExternalPoolAdapterArtifactSourceServiceError::Conflict)
}

pub(crate) async fn get_for_admin(
    state: &AppState,
    admission_id: &str,
) -> Result<ExternalPoolAdapterArtifactSourceReceipt, ExternalPoolAdapterArtifactSourceServiceError>
{
    let receipt = state
        .store
        .external_pool_adapter_artifact_source_for_admission(admission_id)
        .map_err(ExternalPoolAdapterArtifactSourceServiceError::Conflict)?
        .ok_or(ExternalPoolAdapterArtifactSourceServiceError::NotFound)?;
    require_current_quarantined_artifact_bytes(
        &state.data_dir,
        receipt.content_address_digest(),
        receipt.artifact_size_bytes(),
    )
    .await?;
    Ok(receipt)
}

pub(crate) fn intake_confirmation() -> &'static str {
    EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_CONFIRMATION
}

fn operation_scope(admin_user_id: &str) -> String {
    format!("external-pool-artifact-source:{admin_user_id}")
}
