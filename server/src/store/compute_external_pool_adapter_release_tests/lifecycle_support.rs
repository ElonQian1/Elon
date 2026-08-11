use std::path::{Path, PathBuf};

use axum::body::Body;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        external_pool_adapter_artifact_source::{
            intake_quarantined_artifact_bytes, QuarantinedExternalPoolAdapterArtifactBytes,
        },
        external_pool_adapter_release_lifecycle::{
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN,
        },
    },
    store::{
        CreateExternalPoolAdapterReleaseAdmissionTerminal,
        ExternalPoolAdapterArtifactSourceReceipt, RecordExternalPoolAdapterArtifactSource, Store,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION,
    },
};

use super::*;

pub(super) const TERMINAL_ACTOR: &str = "admin-release-terminal";
pub(super) const ARTIFACT_ACTOR: &str = "admin-artifact-intake";

#[derive(Clone, Debug)]
pub(super) struct StagedRelease {
    pub admission_id: String,
    pub admission_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub applied_at: String,
    pub declared_sha256: String,
    pub artifact_bytes: Vec<u8>,
}

pub(super) fn temporary_lifecycle_store() -> (Store, PathBuf, PathBuf) {
    let (store, database_path) = temporary_store();
    let data_dir = std::env::temp_dir().join(format!(
        "elon_external_pool_adapter_lifecycle_data_{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&data_dir).expect("artifact test data directory should exist");
    (store, database_path, data_dir)
}

pub(super) fn cleanup_lifecycle_files(database_path: &Path, data_dir: &Path) {
    remove_store_files(database_path);
    if data_dir.exists() {
        std::fs::remove_dir_all(data_dir)
            .expect("artifact test data directory should be removable");
    }
}

pub(super) fn stage_release(
    store: &Store,
    adapter_id: &str,
    release_version: &str,
    tag: &str,
) -> StagedRelease {
    let artifact_bytes = format!("external-pool Adapter artifact fixture: {tag}").into_bytes();
    let declared_sha256 = sha256(&artifact_bytes);
    let mut submission = submit_input(release_version);
    submission.release.adapter_id = adapter_id.to_string();
    submission.release.candidate_artifact_ref = format!("artifact-ref:{tag}");
    submission.release.declared_implementation_sha256 = declared_sha256.clone();
    submission.idempotency_scope = format!("test-release-submit:{tag}");
    submission.idempotency_key = format!("submit-{tag}");
    let request = store
        .submit_external_pool_adapter_release_request(submission)
        .expect("release fixture request should submit");

    let mut review = review_input(
        &request.request_id,
        &request.request_digest,
        &request.request_material_digest,
        REVIEW_DECISION_APPROVED,
    );
    review.idempotency_scope = format!("test-release-review:{tag}");
    review.idempotency_key = format!("review-{tag}");
    let review = store
        .review_external_pool_adapter_release_request(review)
        .expect("release fixture should receive an independent approval");

    let mut application = apply_input(
        &request.request_id,
        &request.request_digest,
        &request.request_material_digest,
        &review.review_digest,
    );
    application.idempotency_scope = format!("test-release-apply:{tag}");
    application.idempotency_key = format!("apply-{tag}");
    let admission = store
        .apply_external_pool_adapter_release(application)
        .expect("approved release fixture should stage");

    StagedRelease {
        admission_id: admission.admission_id,
        admission_digest: admission.admission_digest,
        adapter_id: admission.adapter_id,
        release_version: admission.release_version,
        applied_at: admission.applied_at,
        declared_sha256,
        artifact_bytes,
    }
}

pub(super) fn terminal_input(
    release: &StagedRelease,
    terminal_status: &str,
    key: &str,
) -> CreateExternalPoolAdapterReleaseAdmissionTerminal {
    let confirmation = match terminal_status {
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN => {
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION
        }
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED => {
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION
        }
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED => {
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION
        }
        other => panic!("unsupported terminal fixture status {other}"),
    };
    CreateExternalPoolAdapterReleaseAdmissionTerminal {
        admission_id: release.admission_id.clone(),
        expected_admission_digest: release.admission_digest.clone(),
        terminal_status: terminal_status.to_string(),
        successor_admission_id: None,
        expected_successor_admission_digest: None,
        actor_id: TERMINAL_ACTOR.to_string(),
        reason: format!("terminal fixture reason for {key}"),
        confirmation: confirmation.to_string(),
        idempotency_scope: format!("test-release-terminal:{TERMINAL_ACTOR}"),
        idempotency_key: key.to_string(),
    }
}

pub(super) fn supersession_input(
    release: &StagedRelease,
    successor: &StagedRelease,
    key: &str,
) -> CreateExternalPoolAdapterReleaseAdmissionTerminal {
    let mut input = terminal_input(
        release,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED,
        key,
    );
    input.successor_admission_id = Some(successor.admission_id.clone());
    input.expected_successor_admission_digest = Some(successor.admission_digest.clone());
    input
}

pub(super) async fn sealed_artifact(
    data_dir: &Path,
    release: &StagedRelease,
) -> QuarantinedExternalPoolAdapterArtifactBytes {
    intake_quarantined_artifact_bytes(
        data_dir,
        &release.declared_sha256,
        Body::from(release.artifact_bytes.clone()),
    )
    .await
    .expect("fixture artifact should enter the real quarantine CAS")
}

pub(super) async fn artifact_record_input(
    data_dir: &Path,
    release: &StagedRelease,
    key: &str,
) -> RecordExternalPoolAdapterArtifactSource {
    RecordExternalPoolAdapterArtifactSource {
        admission_id: release.admission_id.clone(),
        expected_admission_digest: release.admission_digest.clone(),
        recorded_by_admin_user_id: ARTIFACT_ACTOR.to_string(),
        intake_confirmation: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_CONFIRMATION.to_string(),
        idempotency_scope: format!("test-artifact-source:{ARTIFACT_ACTOR}"),
        idempotency_key: key.to_string(),
        artifact: sealed_artifact(data_dir, release).await,
    }
}

pub(super) async fn record_artifact(
    store: &Store,
    data_dir: &Path,
    release: &StagedRelease,
    key: &str,
) -> ExternalPoolAdapterArtifactSourceReceipt {
    store
        .record_external_pool_adapter_artifact_source(
            artifact_record_input(data_dir, release, key).await,
        )
        .expect("fixture artifact receipt should persist")
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
