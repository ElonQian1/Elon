use std::{
    io::{Seek, SeekFrom, Write},
    time::Instant,
};

use anyhow::{anyhow, bail, Error, Result};

use super::{platform, same_file_identity, validate_regular_file_identity, PinnedManagedFile};

const MAX_MANAGED_WRITE_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedFileSegmentWritePhase {
    PrewriteRevalidate,
    Seek,
    Cancellation,
    Write,
    Flush,
    Sync,
    PostSyncRevalidate,
}

pub(crate) struct ManagedFileSegmentWriteFailure {
    phase: ManagedFileSegmentWritePhase,
    error: Error,
    filesystem_mutation_was_attempted: bool,
}

impl std::fmt::Debug for ManagedFileSegmentWriteFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManagedFileSegmentWriteFailure")
            .field("phase", &self.phase)
            .field(
                "filesystem_mutation_was_attempted",
                &self.filesystem_mutation_was_attempted,
            )
            .finish()
    }
}

impl ManagedFileSegmentWriteFailure {
    pub(crate) fn phase(&self) -> ManagedFileSegmentWritePhase {
        self.phase
    }

    pub(crate) fn filesystem_mutation_was_attempted(&self) -> bool {
        self.filesystem_mutation_was_attempted
    }

    pub(crate) fn into_error(self) -> Error {
        self.error
    }

    fn before(phase: ManagedFileSegmentWritePhase, error: impl Into<Error>) -> Self {
        Self {
            phase,
            error: error.into(),
            filesystem_mutation_was_attempted: false,
        }
    }

    fn after_write_attempt(phase: ManagedFileSegmentWritePhase, error: impl Into<Error>) -> Self {
        Self {
            phase,
            error: error.into(),
            filesystem_mutation_was_attempted: true,
        }
    }
}

impl PinnedManagedFile {
    pub(crate) fn truncate_sync_and_revalidate(&mut self, expected_len: u64) -> Result<()> {
        self.file.set_len(expected_len)?;
        self.file.sync_all()?;
        self.revalidate_exact_len(expected_len)
    }

    pub(crate) fn revalidate_exact_len(&mut self, expected_len: u64) -> Result<()> {
        let identity = platform::inspect(&self.file)?;
        validate_regular_file_identity(identity, self.identity.volume_serial)?;
        if !same_file_identity(identity, self.identity) || identity.file_size != expected_len {
            bail!("NODE_MANAGED_FILE_IDENTITY_OR_LENGTH_CHANGED");
        }
        self.identity = identity;
        Ok(())
    }

    /// Writes one bounded segment without releasing the file or its pinned parent directories.
    /// The cancellation callback runs before every write syscall and once after durability and
    /// identity revalidation. Calling the first write syscall makes the outcome mutation-uncertain,
    /// even if that syscall reports an error or writes zero bytes.
    pub(crate) fn write_segment_sync_and_revalidate(
        &mut self,
        expected_offset: u64,
        bytes: &[u8],
        mut ensure_current: impl FnMut() -> Result<()>,
    ) -> std::result::Result<Instant, ManagedFileSegmentWriteFailure> {
        let segment_len = u64::try_from(bytes.len()).map_err(|error| {
            ManagedFileSegmentWriteFailure::before(
                ManagedFileSegmentWritePhase::PrewriteRevalidate,
                error,
            )
        })?;
        if segment_len == 0 {
            return Err(ManagedFileSegmentWriteFailure::before(
                ManagedFileSegmentWritePhase::PrewriteRevalidate,
                anyhow!("NODE_MANAGED_FILE_SEGMENT_EMPTY"),
            ));
        }
        let expected_end = expected_offset.checked_add(segment_len).ok_or_else(|| {
            ManagedFileSegmentWriteFailure::before(
                ManagedFileSegmentWritePhase::PrewriteRevalidate,
                anyhow!("NODE_MANAGED_FILE_SEGMENT_RANGE_OVERFLOW"),
            )
        })?;
        self.revalidate_exact_len(expected_offset)
            .map_err(|error| {
                ManagedFileSegmentWriteFailure::before(
                    ManagedFileSegmentWritePhase::PrewriteRevalidate,
                    error,
                )
            })?;
        self.file
            .seek(SeekFrom::Start(expected_offset))
            .map_err(|error| {
                ManagedFileSegmentWriteFailure::before(ManagedFileSegmentWritePhase::Seek, error)
            })?;
        ensure_current().map_err(|error| {
            ManagedFileSegmentWriteFailure::before(
                ManagedFileSegmentWritePhase::Cancellation,
                error,
            )
        })?;

        let mut remaining = bytes;
        let mut write_attempted = false;
        while !remaining.is_empty() {
            ensure_current().map_err(|error| {
                if write_attempted {
                    ManagedFileSegmentWriteFailure::after_write_attempt(
                        ManagedFileSegmentWritePhase::Cancellation,
                        error,
                    )
                } else {
                    ManagedFileSegmentWriteFailure::before(
                        ManagedFileSegmentWritePhase::Cancellation,
                        error,
                    )
                }
            })?;
            let buffer_len = remaining.len().min(MAX_MANAGED_WRITE_BUFFER_BYTES);
            write_attempted = true;
            let written = self.file.write(&remaining[..buffer_len]).map_err(|error| {
                ManagedFileSegmentWriteFailure::after_write_attempt(
                    ManagedFileSegmentWritePhase::Write,
                    error,
                )
            })?;
            if written == 0 {
                return Err(ManagedFileSegmentWriteFailure::after_write_attempt(
                    ManagedFileSegmentWritePhase::Write,
                    std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "NODE_MANAGED_FILE_SEGMENT_WRITE_ZERO",
                    ),
                ));
            }
            remaining = &remaining[written..];
        }
        ensure_current().map_err(|error| {
            ManagedFileSegmentWriteFailure::after_write_attempt(
                ManagedFileSegmentWritePhase::Cancellation,
                error,
            )
        })?;
        self.file.flush().map_err(|error| {
            ManagedFileSegmentWriteFailure::after_write_attempt(
                ManagedFileSegmentWritePhase::Flush,
                error,
            )
        })?;
        self.file.sync_all().map_err(|error| {
            ManagedFileSegmentWriteFailure::after_write_attempt(
                ManagedFileSegmentWritePhase::Sync,
                error,
            )
        })?;
        self.revalidate_exact_len(expected_end).map_err(|error| {
            ManagedFileSegmentWriteFailure::after_write_attempt(
                ManagedFileSegmentWritePhase::PostSyncRevalidate,
                error,
            )
        })?;
        ensure_current().map_err(|error| {
            ManagedFileSegmentWriteFailure::after_write_attempt(
                ManagedFileSegmentWritePhase::Cancellation,
                error,
            )
        })?;
        Ok(Instant::now())
    }
}
