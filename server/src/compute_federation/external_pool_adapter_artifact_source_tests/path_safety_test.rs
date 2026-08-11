use axum::body::Body;

use super::{
    super::{
        intake_quarantined_artifact_bytes, require_current_quarantined_artifact_bytes,
        ExternalPoolAdapterArtifactSourceFsError,
    },
    support::{
        artifact_bytes, assert_unsafe_target, blob_path, create_directory_link, create_file_link,
        intake, namespace_paths, remove_directory_link, remove_file_link, sha256, shard_path,
        TestRoot,
    },
};

#[tokio::test]
async fn data_dir_regular_file_is_rejected_without_mutation() {
    let root = TestRoot::new("data-dir-file");
    let sentinel = b"DATA_DIR is not a directory";
    std::fs::write(root.path(), sentinel).expect("create DATA_DIR file fixture");
    let bytes = artifact_bytes();
    let digest = sha256(bytes);

    let require_error =
        require_current_quarantined_artifact_bytes(root.path(), &digest, bytes.len() as u64)
            .await
            .expect_err("regular-file DATA_DIR must be rejected by recovery");
    assert_unsafe_target(require_error);

    let intake_error =
        intake_quarantined_artifact_bytes(root.path(), &digest, Body::from(bytes.to_vec()))
            .await
            .expect_err("regular-file DATA_DIR must be rejected");
    assert!(matches!(
        intake_error,
        ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget
            | ExternalPoolAdapterArtifactSourceFsError::Storage(_)
    ));
    assert_eq!(
        std::fs::read(root.path()).expect("read rejected DATA_DIR file"),
        sentinel
    );
    std::fs::remove_file(root.path()).expect("clean DATA_DIR file fixture");
}

#[cfg(unix)]
#[tokio::test]
async fn unix_group_or_other_writable_data_dir_is_rejected() {
    use std::os::unix::fs::PermissionsExt;

    for (label, mode) in [("group-writable", 0o770), ("other-writable", 0o702)] {
        let root = TestRoot::new(label);
        root.create();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(mode))
            .expect("make DATA_DIR writable outside owner");
        let bytes = artifact_bytes();
        let digest = sha256(bytes);

        let error =
            intake_quarantined_artifact_bytes(root.path(), &digest, Body::from(bytes.to_vec()))
                .await
                .expect_err("group/other-writable DATA_DIR must be rejected");
        assert_unsafe_target(error);
        assert!(
            !root.path().join("compute-federation").exists(),
            "unsafe DATA_DIR must not gain a quarantine namespace"
        );
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700))
            .expect("restore DATA_DIR permissions for cleanup");
    }
}

#[tokio::test]
async fn namespace_component_regular_file_is_rejected_without_mutation() {
    let root = TestRoot::new("namespace-component-file");
    root.create();
    let component = root.path().join("compute-federation");
    let sentinel = b"not a quarantine namespace";
    std::fs::write(&component, sentinel).expect("create namespace file fixture");
    let bytes = artifact_bytes();
    let digest = sha256(bytes);

    let error = intake_quarantined_artifact_bytes(root.path(), &digest, Body::from(bytes.to_vec()))
        .await
        .expect_err("regular-file namespace component must be rejected");
    assert_unsafe_target(error);
    assert_eq!(
        std::fs::read(&component).expect("read rejected namespace component"),
        sentinel
    );
    assert_eq!(
        std::fs::read_dir(root.path())
            .expect("read DATA_DIR after rejection")
            .count(),
        1,
        "rejected component must not create sibling or external paths"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn unix_created_namespace_and_blob_have_private_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let root = TestRoot::new("unix-private-permissions");
    let bytes = artifact_bytes();
    let digest = sha256(bytes);
    let sealed = intake(&root, bytes).await;

    for directory in namespace_paths(&root)
        .into_iter()
        .chain(std::iter::once(shard_path(&root, &digest)))
    {
        let mode = std::fs::symlink_metadata(&directory)
            .expect("read private namespace metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "private namespace mode: {directory:?}");
    }
    let blob_mode = std::fs::symlink_metadata(blob_path(&root, &digest))
        .expect("read private blob metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(blob_mode, 0o600);
    drop(sealed);
}

#[cfg(unix)]
#[tokio::test]
async fn unix_permissive_blob_is_rejected() {
    use std::os::unix::fs::PermissionsExt;

    let root = TestRoot::new("unix-permissive-blob");
    let bytes = artifact_bytes();
    let digest = sha256(bytes);
    drop(intake(&root, bytes).await);
    let path = blob_path(&root, &digest);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("make fixture blob unsafe");

    let error =
        require_current_quarantined_artifact_bytes(root.path(), &digest, bytes.len() as u64)
            .await
            .expect_err("world-readable CAS blob must be rejected");
    assert_unsafe_target(error);
}

#[cfg(unix)]
#[tokio::test]
async fn unix_permissive_namespace_is_rejected() {
    use std::os::unix::fs::PermissionsExt;

    let root = TestRoot::new("unix-permissive-namespace");
    let bytes = artifact_bytes();
    let digest = sha256(bytes);
    drop(intake(&root, bytes).await);
    let namespace = namespace_paths(&root)
        .into_iter()
        .next()
        .expect("namespace component");
    std::fs::set_permissions(&namespace, std::fs::Permissions::from_mode(0o755))
        .expect("make fixture namespace unsafe");

    let error =
        require_current_quarantined_artifact_bytes(root.path(), &digest, bytes.len() as u64)
            .await
            .expect_err("permissive quarantine namespace must be rejected");
    assert_unsafe_target(error);
}

#[tokio::test]
async fn leaf_symlink_or_reparse_point_is_rejected() {
    let root = TestRoot::new("leaf-link");
    let outside = TestRoot::new("leaf-link-outside");
    outside.create();
    let bytes = artifact_bytes();
    let digest = sha256(bytes);
    drop(intake(&root, bytes).await);

    let outside_blob = outside.path().join("outside.blob");
    std::fs::write(&outside_blob, bytes).expect("write exact outside blob");
    let path = blob_path(&root, &digest);
    std::fs::remove_file(&path).expect("remove fixture CAS leaf");
    if !create_file_link(&outside_blob, &path) {
        remove_file_link(&path);
        #[cfg(windows)]
        {
            // Windows requires Developer Mode or elevated symlink privilege. Directory
            // junction/reparse rejection remains covered by the adjacent portable fixture.
            eprintln!("file symlink fixture unavailable; skipping privileged Windows branch");
            return;
        }
        #[cfg(not(windows))]
        panic!("platform must support creating a file symlink/reparse fixture for this test");
    }

    let error =
        require_current_quarantined_artifact_bytes(root.path(), &digest, bytes.len() as u64)
            .await
            .expect_err("linked CAS leaf must be rejected even when bytes are exact");
    assert_unsafe_target(error);
    remove_file_link(&path);
    assert_eq!(
        std::fs::read(outside_blob).expect("read outside blob after rejection"),
        bytes
    );
}

#[tokio::test]
async fn namespace_directory_symlink_junction_or_reparse_point_is_rejected() {
    let root = TestRoot::new("directory-link");
    let outside = TestRoot::new("directory-link-outside");
    root.create();
    outside.create();
    let linked_namespace = root.path().join("compute-federation");
    if !create_directory_link(outside.path(), &linked_namespace) {
        remove_directory_link(&linked_namespace);
        panic!("platform must support creating a directory symlink/junction fixture for this test");
    }

    let bytes = artifact_bytes();
    let digest = sha256(bytes);
    let error = intake_quarantined_artifact_bytes(root.path(), &digest, Body::from(bytes.to_vec()))
        .await
        .expect_err("linked quarantine namespace must be rejected");
    assert_unsafe_target(error);
    remove_directory_link(&linked_namespace);
    assert!(
        std::fs::read_dir(outside.path())
            .expect("read outside directory")
            .next()
            .is_none(),
        "rejected namespace link must not write outside DATA_DIR"
    );
}
