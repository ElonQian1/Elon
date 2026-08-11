use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use sha2::{Digest, Sha256};

use super::super::{
    ExternalPoolAdapterArtifactSourceFsError, MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES,
};

mod paths;
mod verification;

use paths::ArtifactPaths;
pub(super) use paths::{locate_paths, prepare_paths, validate_sha256};
use verification::{open_verified_final, require_current_path_identity, unsafe_regular_file};

pub(super) struct ReopenedArtifact {
    pub(super) file: File,
    pub(super) pinned_directories: Vec<File>,
    pub(super) sha256: String,
    pub(super) size_bytes: u64,
}

pub(super) async fn reopen_final(
    paths: ArtifactPaths,
    expected_sha256: &str,
    expected_size_bytes: Option<u64>,
) -> Result<ReopenedArtifact, ExternalPoolAdapterArtifactSourceFsError> {
    let expected_sha256 = expected_sha256.to_string();
    tokio::task::spawn_blocking(move || {
        reopen_final_blocking(paths, &expected_sha256, expected_size_bytes)
    })
    .await
    .map_err(ExternalPoolAdapterArtifactSourceFsError::Task)?
}

fn reopen_final_blocking(
    paths: ArtifactPaths,
    expected_sha256: &str,
    expected_size_bytes: Option<u64>,
) -> Result<ReopenedArtifact, ExternalPoolAdapterArtifactSourceFsError> {
    let mut file = open_verified_final(paths.final_path(), paths.quarantine_root())?;
    let before = file
        .metadata()
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    if unsafe_regular_file(&before) {
        return Err(ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget);
    }
    if before.len() == 0
        || before.len() > MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES
        || expected_size_bytes.is_some_and(|expected| expected != before.len())
    {
        return Err(ExternalPoolAdapterArtifactSourceFsError::BlobDrift);
    }

    let mut hasher = Sha256::new();
    let mut measured_size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
        if read == 0 {
            break;
        }
        measured_size = measured_size
            .checked_add(read as u64)
            .ok_or(ExternalPoolAdapterArtifactSourceFsError::BlobDrift)?;
        if measured_size > MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES {
            return Err(ExternalPoolAdapterArtifactSourceFsError::BlobDrift);
        }
        hasher.update(&buffer[..read]);
    }
    let reopened_sha256 = hex::encode(hasher.finalize());
    let after = file
        .metadata()
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    if unsafe_regular_file(&after)
        || before.len() != after.len()
        || measured_size != after.len()
        || reopened_sha256 != expected_sha256
        || expected_size_bytes.is_some_and(|expected| expected != measured_size)
    {
        return Err(ExternalPoolAdapterArtifactSourceFsError::BlobDrift);
    }
    require_current_path_identity(paths.final_path(), paths.quarantine_root(), &file)?;
    file.seek(SeekFrom::Start(0))
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    let pinned_directories = paths.into_pinned_directories();
    Ok(ReopenedArtifact {
        file,
        pinned_directories,
        sha256: reopened_sha256,
        size_bytes: measured_size,
    })
}

pub(super) fn install_new_and_sync(
    temporary: &Path,
    final_path: &Path,
) -> Result<TemporaryArtifactDisposition, ExternalPoolAdapterArtifactSourceFsError> {
    let disposition = install_new(temporary, final_path)
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    sync_parent_directory(
        final_path
            .parent()
            .ok_or(ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget)?,
    )?;
    Ok(disposition)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TemporaryArtifactDisposition {
    Removed,
    Deferred,
}

#[cfg(windows)]
fn install_new(source: &Path, target: &Path) -> std::io::Result<TemporaryArtifactDisposition> {
    // A rename requires DELETE sharing on the pinned shard directory and would reopen the
    // directory-swap window this custody boundary exists to close. A same-directory hard link
    // atomically installs a no-clobber final name while every directory handle remains pinned.
    match std::fs::hard_link(source, target) {
        Ok(()) => Ok(TemporaryArtifactDisposition::Deferred),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            Ok(TemporaryArtifactDisposition::Deferred)
        }
        Err(error) => Err(error),
    }
}

#[cfg(not(windows))]
fn install_new(source: &Path, target: &Path) -> std::io::Result<TemporaryArtifactDisposition> {
    // `rename` replaces on Unix. A same-directory hard link is an atomic no-clobber install.
    match std::fs::hard_link(source, target) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    std::fs::remove_file(source)?;
    Ok(TemporaryArtifactDisposition::Removed)
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)
}

#[cfg(not(unix))]
fn sync_parent_directory(_parent: &Path) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    // The file bytes are flushed before the no-clobber hard-link installation on Windows.
    Ok(())
}
