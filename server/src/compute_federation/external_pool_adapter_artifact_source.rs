//! Server-owned quarantine storage for external-pool Adapter artifact bytes.
//!
//! This boundary proves only that one raw body was installed in the local DATA_DIR quarantine and
//! that the final regular file was reopened with the expected length and SHA-256. It does not
//! resolve `candidate_artifact_ref`, validate an Adapter, or mint execution authority.

mod filesystem;

#[cfg(test)]
#[path = "external_pool_adapter_artifact_source_tests.rs"]
mod tests;

use std::{fmt, fs::File};

use thiserror::Error;

pub(crate) use filesystem::{
    intake_quarantined_artifact_bytes, open_current_quarantined_artifact_bytes,
    require_current_quarantined_artifact_bytes,
};

pub(crate) const MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;

/// Non-forgeable, point-in-time evidence passed from the DATA_DIR intake boundary to the Store.
///
/// The reopened handle is retained until the Store consumes the evidence. The type is non-Clone,
/// has no serde implementation, exposes no path, and has no external constructor.
pub(crate) struct QuarantinedExternalPoolAdapterArtifactBytes {
    _reopened_file: File,
    _pinned_directories: Vec<File>,
    // Field order is intentional: Windows cleanup runs only after the final file and pinned
    // directory handles have closed.
    _temporary_guard: filesystem::TemporaryArtifactGuard,
    intake_sha256: String,
    reopened_sha256: String,
    artifact_size_bytes: u64,
    content_address_digest: String,
}

/// A pathless, non-cloneable handle to the currently verified CAS object.
///
/// Static consumers may read this handle, but cannot derive a local path or execution authority.
pub(crate) struct CurrentQuarantinedExternalPoolAdapterArtifactBytes {
    reopened_file: File,
    _pinned_directories: Vec<File>,
    content_address_digest: String,
    artifact_size_bytes: u64,
}

impl CurrentQuarantinedExternalPoolAdapterArtifactBytes {
    pub(crate) fn reader(&mut self) -> &mut File {
        &mut self.reopened_file
    }

    pub(crate) fn content_address_digest(&self) -> &str {
        &self.content_address_digest
    }

    pub(crate) fn artifact_size_bytes(&self) -> u64 {
        self.artifact_size_bytes
    }
}

impl QuarantinedExternalPoolAdapterArtifactBytes {
    pub(crate) fn intake_sha256(&self) -> &str {
        &self.intake_sha256
    }

    pub(crate) fn reopened_sha256(&self) -> &str {
        &self.reopened_sha256
    }

    pub(crate) fn artifact_size_bytes(&self) -> u64 {
        self.artifact_size_bytes
    }

    pub(crate) fn content_address_digest(&self) -> &str {
        &self.content_address_digest
    }
}

impl fmt::Debug for QuarantinedExternalPoolAdapterArtifactBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuarantinedExternalPoolAdapterArtifactBytes")
            .field("intake_sha256", &self.intake_sha256)
            .field("reopened_sha256", &self.reopened_sha256)
            .field("artifact_size_bytes", &self.artifact_size_bytes)
            .field("content_address_digest", &self.content_address_digest)
            .field("reopened_file", &"<sealed file handle>")
            .field("pinned_directories", &"<sealed directory handles>")
            .finish()
    }
}

#[derive(Debug, Error)]
pub(crate) enum ExternalPoolAdapterArtifactSourceFsError {
    #[error("external-pool Adapter artifact body could not be read")]
    BodyRead(#[source] axum::Error),
    #[error("external-pool Adapter artifact body must not be empty")]
    EmptyBody,
    #[error("external-pool Adapter artifact body exceeds 33554432 bytes")]
    PayloadTooLarge,
    #[error("external-pool Adapter artifact body SHA-256 conflicts with the staged admission")]
    IntakeDigestMismatch,
    #[error("external-pool Adapter artifact content-address digest is invalid")]
    InvalidContentAddress,
    #[error("external-pool Adapter artifact quarantine target is missing")]
    BlobMissing,
    #[error("external-pool Adapter artifact quarantine target is not a safe regular file")]
    UnsafeTarget,
    #[error("external-pool Adapter artifact quarantine bytes drifted from their receipt")]
    BlobDrift,
    #[error("external-pool Adapter artifact quarantine storage failed")]
    Storage(#[source] std::io::Error),
    #[error("external-pool Adapter artifact quarantine task failed")]
    Task(#[source] tokio::task::JoinError),
}
