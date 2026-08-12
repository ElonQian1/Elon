use std::{path::Path, path::PathBuf};

use axum::body::Body;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use super::{
    CurrentQuarantinedExternalPoolAdapterArtifactBytes, ExternalPoolAdapterArtifactSourceFsError,
    QuarantinedExternalPoolAdapterArtifactBytes, MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES,
};

mod storage;

use storage::{
    install_new_and_sync, locate_paths, prepare_paths, reopen_final, validate_sha256,
    TemporaryArtifactDisposition,
};

pub(super) struct TemporaryArtifactGuard {
    path: Option<PathBuf>,
}

impl TemporaryArtifactGuard {
    fn unarmed() -> Self {
        Self { path: None }
    }

    fn arm(&mut self, path: PathBuf) {
        self.path = Some(temporary_cleanup_path(path));
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

fn temporary_cleanup_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name()) {
        let file_name = file_name.to_os_string();
        if let Ok(canonical_parent) = std::fs::canonicalize(parent) {
            // `canonicalize` returns an extended-length path on Windows. Capture it while the
            // parent is pinned so best-effort cleanup still works beyond the legacy MAX_PATH.
            return canonical_parent.join(file_name);
        }
    }
    path
}

impl Drop for TemporaryArtifactGuard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Streams one raw body into a same-directory create-new part, installs it without clobbering an
/// existing CAS target, and returns only evidence derived from reopening the final regular file.
pub(crate) async fn intake_quarantined_artifact_bytes(
    data_dir: &Path,
    expected_sha256: &str,
    body: Body,
) -> Result<QuarantinedExternalPoolAdapterArtifactBytes, ExternalPoolAdapterArtifactSourceFsError> {
    validate_sha256(expected_sha256)?;
    // This guard is declared before `paths`, so every failure releases pinned directories before
    // attempting cleanup. On Windows, the successful proof object preserves the same drop order.
    let mut temporary_guard = TemporaryArtifactGuard::unarmed();
    let paths = prepare_paths(data_dir, expected_sha256).await?;
    let temporary = paths.temporary_path();
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    #[cfg(windows)]
    options.share_mode(0);
    let mut file = options
        .open(&temporary)
        .await
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    // A nonce collision cannot delete another file because the guard is armed only after
    // `create_new` succeeds.
    temporary_guard.arm(temporary.clone());

    let intake = stream_and_sync_artifact(&mut file, expected_sha256, body).await;
    let (intake_sha256, artifact_size_bytes) = match intake {
        Ok(value) => value,
        Err(error) => {
            // Tokio may retain a blocking file operation after a future resolves. Converting back
            // to `std::fs::File` waits for it before the pinned directories and part are removed.
            drop(file.into_std().await);
            drop(paths);
            drop(temporary_guard);
            return Err(error);
        }
    };
    drop(file.into_std().await);

    let final_path = paths.final_path().to_path_buf();
    let disposition =
        tokio::task::spawn_blocking(move || install_new_and_sync(&temporary, &final_path))
            .await
            .map_err(ExternalPoolAdapterArtifactSourceFsError::Task)??;
    if disposition == TemporaryArtifactDisposition::Removed {
        temporary_guard.disarm();
    }

    let reopened = reopen_final(paths, expected_sha256, Some(artifact_size_bytes)).await?;
    Ok(QuarantinedExternalPoolAdapterArtifactBytes {
        _reopened_file: reopened.file,
        _pinned_directories: reopened.pinned_directories,
        _temporary_guard: temporary_guard,
        intake_sha256,
        reopened_sha256: reopened.sha256,
        artifact_size_bytes: reopened.size_bytes,
        content_address_digest: expected_sha256.to_string(),
    })
}

async fn stream_and_sync_artifact(
    file: &mut tokio::fs::File,
    expected_sha256: &str,
    body: Body,
) -> Result<(String, u64), ExternalPoolAdapterArtifactSourceFsError> {
    let mut stream = body.into_data_stream();
    let mut intake_hasher = Sha256::new();
    let mut artifact_size_bytes = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(ExternalPoolAdapterArtifactSourceFsError::BodyRead)?;
        artifact_size_bytes = artifact_size_bytes
            .checked_add(chunk.len() as u64)
            .ok_or(ExternalPoolAdapterArtifactSourceFsError::PayloadTooLarge)?;
        if artifact_size_bytes > MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES {
            return Err(ExternalPoolAdapterArtifactSourceFsError::PayloadTooLarge);
        }
        intake_hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    }
    if artifact_size_bytes == 0 {
        return Err(ExternalPoolAdapterArtifactSourceFsError::EmptyBody);
    }
    let intake_sha256 = hex::encode(intake_hasher.finalize());
    if intake_sha256 != expected_sha256 {
        return Err(ExternalPoolAdapterArtifactSourceFsError::IntakeDigestMismatch);
    }
    file.flush()
        .await
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    file.sync_all()
        .await
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    Ok((intake_sha256, artifact_size_bytes))
}

/// Reopens and fully rehashes the current CAS file for every receipt replay, GET, and future use.
pub(crate) async fn require_current_quarantined_artifact_bytes(
    data_dir: &Path,
    content_address_digest: &str,
    expected_size_bytes: u64,
) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    open_current_quarantined_artifact_bytes(data_dir, content_address_digest, expected_size_bytes)
        .await?;
    Ok(())
}

/// Reopens, hashes, and retains the exact current CAS file without exposing its path.
pub(crate) async fn open_current_quarantined_artifact_bytes(
    data_dir: &Path,
    content_address_digest: &str,
    expected_size_bytes: u64,
) -> Result<
    CurrentQuarantinedExternalPoolAdapterArtifactBytes,
    ExternalPoolAdapterArtifactSourceFsError,
> {
    validate_sha256(content_address_digest)?;
    if expected_size_bytes == 0 || expected_size_bytes > MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES {
        return Err(ExternalPoolAdapterArtifactSourceFsError::BlobDrift);
    }
    let paths = locate_paths(data_dir, content_address_digest).await?;
    let reopened = reopen_final(paths, content_address_digest, Some(expected_size_bytes)).await?;
    Ok(CurrentQuarantinedExternalPoolAdapterArtifactBytes {
        reopened_file: reopened.file,
        _pinned_directories: reopened.pinned_directories,
        content_address_digest: reopened.sha256,
        artifact_size_bytes: reopened.size_bytes,
    })
}
