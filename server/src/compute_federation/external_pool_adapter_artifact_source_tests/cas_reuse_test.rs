use axum::body::Body;

use super::{
    super::{
        intake_quarantined_artifact_bytes, require_current_quarantined_artifact_bytes,
        ExternalPoolAdapterArtifactSourceFsError,
    },
    support::{
        artifact_bytes, assert_blob_drift, assert_no_part, blob_path, file_identity, intake,
        sha256, TestRoot,
    },
};

#[tokio::test]
async fn exact_existing_cas_is_reused_without_overwrite() {
    let root = TestRoot::new("exact-existing-cas");
    let bytes = artifact_bytes();
    let digest = sha256(bytes);

    let first = intake(&root, bytes).await;
    assert_eq!(first.intake_sha256(), digest);
    assert_eq!(first.reopened_sha256(), digest);
    assert_eq!(first.artifact_size_bytes(), bytes.len() as u64);
    assert_eq!(first.content_address_digest(), digest);
    drop(first);

    let path = blob_path(&root, &digest);
    let identity_before = file_identity(&path);
    let modified_before = std::fs::metadata(&path)
        .expect("read initial CAS metadata")
        .modified()
        .expect("read initial CAS modification time");

    let replay =
        intake_quarantined_artifact_bytes(root.path(), &digest, Body::from(bytes.to_vec()))
            .await
            .expect("exact existing CAS must be reusable");
    assert_eq!(replay.reopened_sha256(), digest);
    assert_eq!(replay.artifact_size_bytes(), bytes.len() as u64);
    drop(replay);

    assert_eq!(std::fs::read(&path).expect("read reused CAS"), bytes);
    assert_eq!(file_identity(&path), identity_before);
    assert_eq!(
        std::fs::metadata(&path)
            .expect("read replayed CAS metadata")
            .modified()
            .expect("read replayed CAS modification time"),
        modified_before
    );
    assert_no_part(&root, &digest);
}

#[tokio::test]
async fn corrupt_existing_cas_is_rejected_without_repair_or_replacement() {
    let root = TestRoot::new("corrupt-existing-cas");
    let bytes = artifact_bytes();
    let digest = sha256(bytes);
    let sealed = intake(&root, bytes).await;
    drop(sealed);

    let path = blob_path(&root, &digest);
    let corrupt = vec![b'x'; bytes.len()];
    std::fs::write(&path, &corrupt).expect("corrupt existing CAS in fixture");
    let corrupt_identity = file_identity(&path);

    let error = intake_quarantined_artifact_bytes(root.path(), &digest, Body::from(bytes.to_vec()))
        .await
        .expect_err("corrupt existing CAS must not be repaired by replay");
    assert_blob_drift(error);

    assert_eq!(
        std::fs::read(&path).expect("read rejected corrupt CAS"),
        corrupt
    );
    assert_eq!(file_identity(&path), corrupt_identity);
    assert_no_part(&root, &digest);
}

#[tokio::test]
async fn final_directory_replacement_is_rejected_by_require_and_retry_without_repair() {
    let root = TestRoot::new("final-directory-replacement");
    let bytes = artifact_bytes();
    let digest = sha256(bytes);
    drop(intake(&root, bytes).await);
    let path = blob_path(&root, &digest);
    std::fs::remove_file(&path).expect("remove installed CAS leaf");
    std::fs::create_dir(&path).expect("replace final leaf with directory");

    let require_error =
        require_current_quarantined_artifact_bytes(root.path(), &digest, bytes.len() as u64)
            .await
            .expect_err("non-regular final must be rejected by require");
    assert!(matches!(
        require_error,
        ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget
    ));

    let retry_error =
        intake_quarantined_artifact_bytes(root.path(), &digest, Body::from(bytes.to_vec()))
            .await
            .expect_err("non-regular final must be rejected by retry");
    assert!(matches!(
        retry_error,
        ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget
            | ExternalPoolAdapterArtifactSourceFsError::Storage(_)
    ));
    assert!(
        std::fs::symlink_metadata(&path)
            .expect("read rejected final directory")
            .is_dir(),
        "retry must not replace or repair a non-regular final"
    );
    assert!(
        std::fs::read_dir(&path)
            .expect("read rejected final directory")
            .next()
            .is_none(),
        "retry must not write inside the final directory"
    );
    assert_no_part(&root, &digest);
}
