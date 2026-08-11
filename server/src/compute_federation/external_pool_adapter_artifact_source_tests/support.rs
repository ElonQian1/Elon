use std::{
    convert::Infallible,
    path::{Path, PathBuf},
};

#[cfg(windows)]
use std::fs::File;

use axum::body::{Body, Bytes};
use futures::stream;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::super::{
    intake_quarantined_artifact_bytes, ExternalPoolAdapterArtifactSourceFsError,
    QuarantinedExternalPoolAdapterArtifactBytes, MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES,
};

const NAMESPACE_COMPONENTS: [&str; 6] = [
    "compute-federation",
    "external-pool-adapter-artifacts",
    "v1",
    "quarantine",
    "blobs",
    "sha256",
];

pub(super) struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    pub(super) fn new(label: &str) -> Self {
        Self {
            path: std::env::temp_dir().join(format!(
                "elon-external-pool-artifact-{label}-{}",
                Uuid::new_v4().simple()
            )),
        }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn create(&self) {
        std::fs::create_dir_all(&self.path).expect("create isolated artifact test root");
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        match std::fs::symlink_metadata(&self.path) {
            Ok(metadata) if metadata.is_dir() => {
                let _ = std::fs::remove_dir_all(&self.path);
            }
            Ok(_) => {
                let _ = std::fs::remove_file(&self.path);
            }
            Err(_) => {}
        }
    }
}

pub(super) fn artifact_bytes() -> &'static [u8] {
    b"v227-external-pool-adapter-artifact"
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub(super) async fn intake(
    root: &TestRoot,
    bytes: &[u8],
) -> QuarantinedExternalPoolAdapterArtifactBytes {
    let digest = sha256(bytes);
    intake_quarantined_artifact_bytes(root.path(), &digest, Body::from(bytes.to_vec()))
        .await
        .expect("install quarantined artifact bytes")
}

pub(super) fn shard_path(root: &TestRoot, digest: &str) -> PathBuf {
    namespace_paths(root)
        .last()
        .expect("namespace has sha256 root")
        .join(&digest[..2])
}

pub(super) fn blob_path(root: &TestRoot, digest: &str) -> PathBuf {
    shard_path(root, digest).join(format!("{digest}.blob"))
}

pub(super) fn namespace_paths(root: &TestRoot) -> Vec<PathBuf> {
    let mut current = root.path().to_path_buf();
    NAMESPACE_COMPONENTS
        .iter()
        .map(|component| {
            current.push(component);
            current.clone()
        })
        .collect()
}

pub(super) fn assert_no_final_or_part(root: &TestRoot, digest: &str) {
    assert!(
        !blob_path(root, digest).exists(),
        "failed intake must not publish a final CAS blob"
    );
    assert_no_part(root, digest);
}

pub(super) fn assert_no_part(root: &TestRoot, digest: &str) {
    let shard = shard_path(root, digest);
    if !shard.exists() {
        return;
    }
    let parts = std::fs::read_dir(shard)
        .expect("read artifact shard")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".part"))
        .collect::<Vec<_>>();
    assert!(
        parts.is_empty(),
        "failed intake leaked part files: {parts:?}"
    );
}

pub(super) fn body_that_fails_after(bytes: &[u8]) -> Body {
    let chunks = vec![
        Ok(Bytes::copy_from_slice(bytes)),
        Err(std::io::Error::other("synthetic artifact body failure")),
    ];
    Body::from_stream(stream::iter(chunks))
}

pub(super) fn over_limit_body() -> Body {
    const CHUNK_BYTES: usize = 64 * 1024;
    let chunk = Bytes::from(vec![0_u8; CHUNK_BYTES]);
    let chunk_count = (MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES as usize / CHUNK_BYTES) + 1;
    Body::from_stream(stream::iter(
        (0..chunk_count).map(move |_| Ok::<Bytes, Infallible>(chunk.clone())),
    ))
}

pub(super) fn assert_blob_drift(error: ExternalPoolAdapterArtifactSourceFsError) {
    assert!(
        matches!(error, ExternalPoolAdapterArtifactSourceFsError::BlobDrift),
        "expected BlobDrift, got {error:?}"
    );
}

pub(super) fn assert_unsafe_target(error: ExternalPoolAdapterArtifactSourceFsError) {
    assert!(
        matches!(
            error,
            ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget
        ),
        "expected UnsafeTarget, got {error:?}"
    );
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct FileIdentity {
    volume: u64,
    index: u64,
}

#[cfg(unix)]
pub(super) fn file_identity(path: &Path) -> FileIdentity {
    use std::os::unix::fs::MetadataExt;

    let metadata = std::fs::metadata(path).expect("read CAS blob identity");
    FileIdentity {
        volume: metadata.dev(),
        index: metadata.ino(),
    }
}

#[cfg(windows)]
pub(super) fn file_identity(path: &Path) -> FileIdentity {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let file = File::open(path).expect("open CAS blob for identity");
    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) };
    assert_ne!(ok, 0, "read CAS blob file identity");
    FileIdentity {
        volume: information.dwVolumeSerialNumber as u64,
        index: ((information.nFileIndexHigh as u64) << 32) | information.nFileIndexLow as u64,
    }
}

#[cfg(not(any(unix, windows)))]
pub(super) fn file_identity(path: &Path) -> FileIdentity {
    let metadata = std::fs::metadata(path).expect("read CAS blob identity");
    FileIdentity {
        volume: 0,
        index: metadata.len(),
    }
}

#[cfg(windows)]
pub(super) fn create_file_link(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

#[cfg(not(windows))]
pub(super) fn create_file_link(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

pub(super) fn remove_file_link(link: &Path) {
    let _ = std::fs::remove_file(link);
}

#[cfg(windows)]
pub(super) fn create_directory_link(target: &Path, link: &Path) -> bool {
    if std::os::windows::fs::symlink_dir(target, link).is_ok() {
        return true;
    }
    std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(windows))]
pub(super) fn create_directory_link(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
pub(super) fn remove_directory_link(link: &Path) {
    let _ = std::fs::remove_dir(link);
}

#[cfg(not(windows))]
pub(super) fn remove_directory_link(link: &Path) {
    let _ = std::fs::remove_file(link);
}
