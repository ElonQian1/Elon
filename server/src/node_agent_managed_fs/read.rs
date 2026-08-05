use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use anyhow::{Context, Result};

use super::PinnedManagedFile;

/// A borrowed view that deliberately implements only `Read + Seek`. Archive parsers never receive
/// the underlying `File`, so safe code cannot use this capability to write verified artifacts.
pub(crate) struct ManagedFileReadCursor<'file> {
    file: &'file mut File,
    ensure_current: &'file mut dyn FnMut() -> Result<()>,
}

impl Read for ManagedFileReadCursor<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        (self.ensure_current)().map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                format!("NODE_MANAGED_FILE_READ_CANCELED: {error:#}"),
            )
        })?;
        self.file.read(buffer)
    }
}

impl Seek for ManagedFileReadCursor<'_> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        (self.ensure_current)().map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                format!("NODE_MANAGED_FILE_SEEK_CANCELED: {error:#}"),
            )
        })?;
        self.file.seek(position)
    }
}

impl PinnedManagedFile {
    /// Runs one parser against the existing pinned handle and revalidates the same identity and
    /// exact length even when parsing fails. The parser receives no write trait or owned handle.
    pub(crate) fn with_read_cursor<T>(
        &mut self,
        expected_len: u64,
        mut ensure_current: impl FnMut() -> Result<()>,
        operation: impl FnOnce(&mut ManagedFileReadCursor<'_>) -> Result<T>,
    ) -> Result<T> {
        self.revalidate_exact_len(expected_len)
            .context("NODE_MANAGED_FILE_READ_PRE_REVALIDATE")?;
        ensure_current().context("NODE_MANAGED_FILE_READ_PRE_CANCELLATION")?;
        self.file
            .seek(SeekFrom::Start(0))
            .context("NODE_MANAGED_FILE_READ_INITIAL_SEEK")?;

        let operation_result = {
            let mut cursor = ManagedFileReadCursor {
                file: &mut self.file,
                ensure_current: &mut ensure_current,
            };
            operation(&mut cursor)
        };

        self.revalidate_exact_len(expected_len)
            .context("NODE_MANAGED_FILE_READ_POST_REVALIDATE")?;
        ensure_current().context("NODE_MANAGED_FILE_READ_POST_CANCELLATION")?;
        operation_result
    }
}
