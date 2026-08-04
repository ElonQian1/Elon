use std::{
    io::{Read, Seek, SeekFrom},
    time::Instant,
};

use anyhow::{anyhow, Error, Result};
use sha2::{Digest, Sha256};

use super::PinnedManagedFile;

const MAX_MANAGED_HASH_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedFileHashPhase {
    PreHashRevalidate,
    Seek,
    Cancellation,
    Read,
    UnexpectedEof,
    UnexpectedTrailingData,
    PostHashRevalidate,
}

pub(crate) struct ManagedFileHashFailure {
    phase: ManagedFileHashPhase,
    error: Error,
}

impl std::fmt::Debug for ManagedFileHashFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManagedFileHashFailure")
            .field("phase", &self.phase)
            .finish()
    }
}

impl ManagedFileHashFailure {
    pub(crate) fn phase(&self) -> ManagedFileHashPhase {
        self.phase
    }

    pub(crate) fn into_error(self) -> Error {
        self.error
    }

    fn new(phase: ManagedFileHashPhase, error: impl Into<Error>) -> Self {
        Self {
            phase,
            error: error.into(),
        }
    }
}

pub(crate) struct ManagedFileHashResult {
    digest: String,
    completed_at: Instant,
}

impl ManagedFileHashResult {
    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }

    pub(crate) fn completed_at(&self) -> Instant {
        self.completed_at
    }
}

impl std::fmt::Debug for ManagedFileHashResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManagedFileHashResult")
            .field("digest", &"<redacted>")
            .field("completed_at", &"<monotonic>")
            .finish()
    }
}

impl PinnedManagedFile {
    /// Recomputes SHA-256 over the exact expected length on the already pinned exclusive handle.
    /// No resumable hash internals escape this call; a restart must hash from byte zero again.
    pub(crate) fn hash_sha256_and_revalidate(
        &mut self,
        expected_len: u64,
        mut ensure_current: impl FnMut() -> Result<()>,
    ) -> std::result::Result<ManagedFileHashResult, ManagedFileHashFailure> {
        if expected_len == 0 {
            return Err(ManagedFileHashFailure::new(
                ManagedFileHashPhase::PreHashRevalidate,
                anyhow!("NODE_MANAGED_FILE_HASH_EMPTY"),
            ));
        }
        self.revalidate_exact_len(expected_len).map_err(|error| {
            ManagedFileHashFailure::new(ManagedFileHashPhase::PreHashRevalidate, error)
        })?;
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|error| ManagedFileHashFailure::new(ManagedFileHashPhase::Seek, error))?;

        let mut digest = Sha256::new();
        let mut buffer = [0_u8; MAX_MANAGED_HASH_BUFFER_BYTES];
        let mut remaining = expected_len;
        while remaining > 0 {
            ensure_current().map_err(|error| {
                ManagedFileHashFailure::new(ManagedFileHashPhase::Cancellation, error)
            })?;
            let wanted = usize::try_from(remaining.min(buffer.len() as u64))
                .map_err(|error| ManagedFileHashFailure::new(ManagedFileHashPhase::Read, error))?;
            let read = self
                .file
                .read(&mut buffer[..wanted])
                .map_err(|error| ManagedFileHashFailure::new(ManagedFileHashPhase::Read, error))?;
            if read == 0 {
                return Err(ManagedFileHashFailure::new(
                    ManagedFileHashPhase::UnexpectedEof,
                    anyhow!("NODE_MANAGED_FILE_HASH_UNEXPECTED_EOF"),
                ));
            }
            digest.update(&buffer[..read]);
            remaining -= u64::try_from(read)
                .map_err(|error| ManagedFileHashFailure::new(ManagedFileHashPhase::Read, error))?;
        }
        ensure_current().map_err(|error| {
            ManagedFileHashFailure::new(ManagedFileHashPhase::Cancellation, error)
        })?;
        let mut trailing = [0_u8; 1];
        let trailing_read = self
            .file
            .read(&mut trailing)
            .map_err(|error| ManagedFileHashFailure::new(ManagedFileHashPhase::Read, error))?;
        if trailing_read != 0 {
            return Err(ManagedFileHashFailure::new(
                ManagedFileHashPhase::UnexpectedTrailingData,
                anyhow!("NODE_MANAGED_FILE_HASH_TRAILING_DATA"),
            ));
        }
        self.revalidate_exact_len(expected_len).map_err(|error| {
            ManagedFileHashFailure::new(ManagedFileHashPhase::PostHashRevalidate, error)
        })?;
        ensure_current().map_err(|error| {
            ManagedFileHashFailure::new(ManagedFileHashPhase::Cancellation, error)
        })?;
        Ok(ManagedFileHashResult {
            digest: hex::encode(digest.finalize()),
            completed_at: Instant::now(),
        })
    }
}
