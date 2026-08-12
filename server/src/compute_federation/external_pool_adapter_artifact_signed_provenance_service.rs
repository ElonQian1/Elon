//! Administrator orchestration for exact Artifact signature challenges and receipts.

use anyhow::{Error as AnyError, Result as AnyResult};
use serde::Deserialize;
use thiserror::Error;

use crate::{
    store::{
        CreateExternalPoolAdapterArtifactSignedProvenance,
        ExternalPoolAdapterArtifactSignedProvenanceCurrentnessReceipt,
        ExternalPoolAdapterArtifactSignedProvenanceWriteReceipt,
        GetExternalPoolAdapterArtifactSignatureChallenge, Store,
    },
    types::AppState,
};

use super::{
    external_pool_adapter_artifact_signed_provenance::{
        ExternalPoolAdapterArtifactSignatureChallengeReceipt,
        ARTIFACT_SIGNED_PROVENANCE_CONFIRMATION,
    },
    external_pool_adapter_artifact_source::{
        require_current_quarantined_artifact_bytes, ExternalPoolAdapterArtifactSourceFsError,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactSignatureChallengeBody {
    pub expected_admission_digest: String,
    pub expected_source_receipt_digest: String,
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub expected_key_id: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordArtifactSignedProvenanceBody {
    pub expected_admission_digest: String,
    pub expected_source_receipt_digest: String,
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub expected_key_id: String,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub idempotency_key: String,
    pub confirm_provenance: bool,
}

#[derive(Debug, Error)]
pub(crate) enum ArtifactSignedProvenanceServiceError {
    #[error("external-pool Adapter Artifact signed provenance was not found")]
    NotFound,
    #[error("external-pool Adapter Artifact signed-provenance request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter Artifact signed-provenance lineage conflicts")]
    Conflict(#[source] AnyError),
    #[error(transparent)]
    Filesystem(#[from] ExternalPoolAdapterArtifactSourceFsError),
}

pub(crate) async fn challenge_for_admin(
    state: &AppState,
    admission_id: &str,
    body: ArtifactSignatureChallengeBody,
) -> Result<
    ExternalPoolAdapterArtifactSignatureChallengeReceipt,
    ArtifactSignedProvenanceServiceError,
> {
    require_current_bytes(state, admission_id, &body.expected_source_receipt_digest).await?;
    state
        .store
        .external_pool_adapter_artifact_signature_challenge(challenge_input(admission_id, &body))
        .map_err(classify_store_error)
}

pub(crate) async fn record_for_admin(
    state: &AppState,
    admin_user_id: &str,
    admission_id: &str,
    body: RecordArtifactSignedProvenanceBody,
) -> Result<
    ExternalPoolAdapterArtifactSignedProvenanceWriteReceipt,
    ArtifactSignedProvenanceServiceError,
> {
    if !body.confirm_provenance {
        return Err(ArtifactSignedProvenanceServiceError::Invalid(
            anyhow::anyhow!("记录签名来源证明前必须显式确认"),
        ));
    }
    require_current_bytes(state, admission_id, &body.expected_source_receipt_digest).await?;
    state
        .store
        .create_external_pool_adapter_artifact_signed_provenance(
            CreateExternalPoolAdapterArtifactSignedProvenance {
                admission_id: admission_id.to_string(),
                expected_admission_digest: body.expected_admission_digest,
                expected_source_receipt_digest: body.expected_source_receipt_digest,
                key_record_id: body.key_record_id,
                expected_key_record_digest: body.expected_key_record_digest,
                expected_key_id: body.expected_key_id,
                expected_signature_message_digest: body.expected_signature_message_digest,
                signature_base64: body.signature_base64,
                verified_by_admin_user_id: admin_user_id.to_string(),
                confirmation: ARTIFACT_SIGNED_PROVENANCE_CONFIRMATION.to_string(),
                idempotency_scope: operation_scope(admin_user_id),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(classify_store_error)
}

pub(crate) async fn currentness_for_admin(
    state: &AppState,
    admission_id: &str,
) -> Result<
    ExternalPoolAdapterArtifactSignedProvenanceCurrentnessReceipt,
    ArtifactSignedProvenanceServiceError,
> {
    let currentness = state
        .store
        .external_pool_adapter_artifact_signed_provenance_currentness(admission_id)
        .map_err(classify_store_error)?
        .ok_or(ArtifactSignedProvenanceServiceError::NotFound)?;
    require_current_bytes(
        state,
        admission_id,
        &currentness.provenance.binding.source_receipt_digest,
    )
    .await?;
    Ok(currentness)
}

async fn require_current_bytes(
    state: &AppState,
    admission_id: &str,
    expected_source_receipt_digest: &str,
) -> Result<(), ArtifactSignedProvenanceServiceError> {
    let source = state
        .store
        .external_pool_adapter_artifact_source_for_admission(admission_id)
        .map_err(classify_store_error)?
        .ok_or(ArtifactSignedProvenanceServiceError::NotFound)?;
    if source.source_receipt_digest != expected_source_receipt_digest {
        return Err(ArtifactSignedProvenanceServiceError::Conflict(
            anyhow::anyhow!("Artifact source receipt digest is stale"),
        ));
    }
    require_current_quarantined_artifact_bytes(
        &state.data_dir,
        source.content_address_digest(),
        source.artifact_size_bytes(),
    )
    .await?;
    Ok(())
}

fn challenge_input(
    admission_id: &str,
    body: &ArtifactSignatureChallengeBody,
) -> GetExternalPoolAdapterArtifactSignatureChallenge {
    GetExternalPoolAdapterArtifactSignatureChallenge {
        admission_id: admission_id.to_string(),
        expected_admission_digest: body.expected_admission_digest.clone(),
        expected_source_receipt_digest: body.expected_source_receipt_digest.clone(),
        key_record_id: body.key_record_id.clone(),
        expected_key_record_digest: body.expected_key_record_digest.clone(),
        expected_key_id: body.expected_key_id.clone(),
    }
}

fn classify_store_error(error: AnyError) -> ArtifactSignedProvenanceServiceError {
    let text = format!("{error:#}");
    if text.contains("was not found") || text.contains("is absent") {
        ArtifactSignedProvenanceServiceError::NotFound
    } else {
        ArtifactSignedProvenanceServiceError::Conflict(error)
    }
}

fn operation_scope(admin_user_id: &str) -> String {
    format!("external-pool-adapter-artifact-signed-provenance:{admin_user_id}")
}
