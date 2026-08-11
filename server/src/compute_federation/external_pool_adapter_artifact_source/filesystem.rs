use std::path::Path;

use axum::body::Body;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use super::{
    ExternalPoolAdapterArtifactSourceFsError, QuarantinedExternalPoolAdapterArtifactBytes,
    MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES,
};

mod storage;

use storage::{
    install_new_and_sync, locate_paths, prepare_paths, reopen_final, validate_sha256,
    TemporaryArtifactGuard,
};

/// Streams one raw body into a same-directory create-new part, installs it without clobbering an
/// existing CAS target, and returns only evidence derived from reopening the final regular file.
pub(crate) async fn intake_quarantined_artifact_bytes(
    data_dir: &Path,
    expected_sha256: &str,
    body: Body,
) -> Result<QuarantinedExternalPoolAdapterArtifactBytes, ExternalPoolAdapterArtifactSourceFsError> {
    validate_sha256(expected_sha256)?;
    let paths = prepare_paths(data_dir, expected_sha256).await?;
    let temporary = paths.temporary_path();
    // Declare the guard before the file so Rust closes the file first on every early return.
    // It stays unarmed until create_new succeeds, so a nonce collision cannot delete another file.
    let mut temporary_guard = TemporaryArtifactGuard::unarmed(temporary.clone());
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
    temporary_guard.arm();

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
    drop(file);

    let final_path = paths.final_path().to_path_buf();
    tokio::task::spawn_blocking(move || install_new_and_sync(&temporary, &final_path))
        .await
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Task)??;
    temporary_guard.disarm();

    let reopened = reopen_final(paths, expected_sha256, Some(artifact_size_bytes)).await?;
    Ok(QuarantinedExternalPoolAdapterArtifactBytes {
        _reopened_file: reopened.file,
        _pinned_directories: reopened.pinned_directories,
        intake_sha256,
        reopened_sha256: reopened.sha256,
        artifact_size_bytes: reopened.size_bytes,
        content_address_digest: expected_sha256.to_string(),
    })
}

/// Reopens and fully rehashes the current CAS file for every receipt replay, GET, and future use.
pub(crate) async fn require_current_quarantined_artifact_bytes(
    data_dir: &Path,
    content_address_digest: &str,
    expected_size_bytes: u64,
) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    validate_sha256(content_address_digest)?;
    if expected_size_bytes == 0 || expected_size_bytes > MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES {
        return Err(ExternalPoolAdapterArtifactSourceFsError::BlobDrift);
    }
    let paths = locate_paths(data_dir, content_address_digest).await?;
    let _reopened = reopen_final(paths, content_address_digest, Some(expected_size_bytes)).await?;
    Ok(())
}
