use super::{
    super::{
        intake_quarantined_artifact_bytes, require_current_quarantined_artifact_bytes,
        ExternalPoolAdapterArtifactSourceFsError,
    },
    support::{artifact_bytes, blob_path, file_identity, intake, sha256, TestRoot},
};
use axum::body::Body;

#[tokio::test]
async fn missing_blob_is_reported_without_creating_namespace() {
    let root = TestRoot::new("missing-blob");
    let digest = sha256(artifact_bytes());

    let error = require_current_quarantined_artifact_bytes(
        root.path(),
        &digest,
        artifact_bytes().len() as u64,
    )
    .await
    .expect_err("missing CAS blob must be reported");
    assert!(matches!(
        error,
        ExternalPoolAdapterArtifactSourceFsError::BlobMissing
    ));
    assert!(
        !root.path().exists(),
        "read-only recovery must not create DATA_DIR namespace"
    );
}

#[tokio::test]
async fn dropped_sealed_evidence_can_be_reopened_and_retried_from_existing_cas() {
    let root = TestRoot::new("dropped-evidence-retry");
    let bytes = artifact_bytes();
    let digest = sha256(bytes);
    let sealed = intake(&root, bytes).await;
    assert_eq!(sealed.reopened_sha256(), digest);
    drop(sealed);

    let path = blob_path(&root, &digest);
    let installed_identity = file_identity(&path);
    require_current_quarantined_artifact_bytes(root.path(), &digest, bytes.len() as u64)
        .await
        .expect("discarded evidence must not make exact CAS unrecoverable");

    let retried =
        intake_quarantined_artifact_bytes(root.path(), &digest, Body::from(bytes.to_vec()))
            .await
            .expect("retry must reopen and reuse exact installed CAS");
    assert_eq!(retried.intake_sha256(), digest);
    assert_eq!(retried.reopened_sha256(), digest);
    drop(retried);

    assert_eq!(file_identity(&path), installed_identity);
    require_current_quarantined_artifact_bytes(root.path(), &digest, bytes.len() as u64)
        .await
        .expect("reused CAS must remain reopenable");
}
