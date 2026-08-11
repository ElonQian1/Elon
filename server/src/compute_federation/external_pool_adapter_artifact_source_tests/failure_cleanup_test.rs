use axum::body::Body;

use super::{
    super::{
        intake_quarantined_artifact_bytes, require_current_quarantined_artifact_bytes,
        ExternalPoolAdapterArtifactSourceFsError,
    },
    support::{
        artifact_bytes, assert_no_final_or_part, body_that_fails_after, over_limit_body, sha256,
        TestRoot,
    },
};

#[tokio::test]
async fn invalid_digest_and_path_injection_are_rejected_before_data_dir_creation() {
    let invalid_digests = vec![
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        format!("{}../{}", "a".repeat(29), "b".repeat(32)),
        format!("{}\\{}", "a".repeat(31), "b".repeat(32)),
    ];

    for (index, invalid_digest) in invalid_digests.into_iter().enumerate() {
        let root = TestRoot::new(&format!("invalid-digest-{index}"));
        let error = intake_quarantined_artifact_bytes(
            root.path(),
            &invalid_digest,
            Body::from(artifact_bytes()),
        )
        .await
        .expect_err("invalid digest must be rejected before path preparation");
        assert!(matches!(
            error,
            ExternalPoolAdapterArtifactSourceFsError::InvalidContentAddress
        ));
        let require_error = require_current_quarantined_artifact_bytes(
            root.path(),
            &invalid_digest,
            artifact_bytes().len() as u64,
        )
        .await
        .expect_err("invalid digest must be rejected by read-only recovery");
        assert!(matches!(
            require_error,
            ExternalPoolAdapterArtifactSourceFsError::InvalidContentAddress
        ));
        assert!(
            !root.path().exists(),
            "invalid digest must not create DATA_DIR"
        );
    }
}

#[tokio::test]
async fn empty_body_leaves_neither_final_blob_nor_part() {
    let root = TestRoot::new("empty-body");
    let digest = sha256(artifact_bytes());

    let error = intake_quarantined_artifact_bytes(root.path(), &digest, Body::empty())
        .await
        .expect_err("empty body must be rejected");
    assert!(matches!(
        error,
        ExternalPoolAdapterArtifactSourceFsError::EmptyBody
    ));
    assert_no_final_or_part(&root, &digest);
}

#[tokio::test]
async fn digest_mismatch_leaves_neither_final_blob_nor_part() {
    let root = TestRoot::new("digest-mismatch");
    let digest = sha256(artifact_bytes());

    let error = intake_quarantined_artifact_bytes(
        root.path(),
        &digest,
        Body::from("different artifact bytes"),
    )
    .await
    .expect_err("digest mismatch must be rejected");
    assert!(matches!(
        error,
        ExternalPoolAdapterArtifactSourceFsError::IntakeDigestMismatch
    ));
    assert_no_final_or_part(&root, &digest);
}

#[tokio::test]
async fn body_read_failure_leaves_neither_final_blob_nor_part() {
    let root = TestRoot::new("body-read-failure");
    let digest = sha256(artifact_bytes());

    let error =
        intake_quarantined_artifact_bytes(root.path(), &digest, body_that_fails_after(b"partial"))
            .await
            .expect_err("body read failure must be rejected");
    assert!(matches!(
        error,
        ExternalPoolAdapterArtifactSourceFsError::BodyRead(_)
    ));
    assert_no_final_or_part(&root, &digest);
}

#[tokio::test]
async fn over_limit_body_leaves_neither_final_blob_nor_part() {
    let root = TestRoot::new("over-limit-body");
    let digest = sha256(artifact_bytes());

    let error = intake_quarantined_artifact_bytes(root.path(), &digest, over_limit_body())
        .await
        .expect_err("over-limit body must be rejected");
    assert!(matches!(
        error,
        ExternalPoolAdapterArtifactSourceFsError::PayloadTooLarge
    ));
    assert_no_final_or_part(&root, &digest);
}
