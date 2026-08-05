use std::{
    io::{Read, Seek, SeekFrom, Write},
    time::Instant,
};

use anyhow::{anyhow, bail, Result};
use sha2::{Digest, Sha256};

use super::PinnedManagedFile;

const MAX_MANAGED_COPY_BUFFER_BYTES: usize = 64 * 1024;

pub(crate) struct ManagedFileCopyResult {
    digest: String,
    completed_at: Instant,
}

impl ManagedFileCopyResult {
    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }

    pub(crate) fn completed_at(&self) -> Instant {
        self.completed_at
    }
}

impl std::fmt::Debug for ManagedFileCopyResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManagedFileCopyResult")
            .field("digest", &"<redacted>")
            .field("completed_at", &"<monotonic>")
            .finish()
    }
}

impl PinnedManagedFile {
    /// Copies exactly one expected byte range from an untrusted reader into a create-new managed
    /// file. The destination is flushed, fsynced and identity-revalidated before success.
    pub(crate) fn copy_reader_sync_hash_and_revalidate(
        &mut self,
        source: &mut impl Read,
        expected_len: u64,
        mut ensure_current: impl FnMut() -> Result<()>,
    ) -> Result<ManagedFileCopyResult> {
        self.revalidate_exact_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        ensure_current()?;

        let mut digest = Sha256::new();
        let mut buffer = [0_u8; MAX_MANAGED_COPY_BUFFER_BYTES];
        let mut remaining = expected_len;
        let mut written_total = 0_u64;
        while remaining > 0 {
            ensure_current()?;
            let wanted = usize::try_from(remaining.min(buffer.len() as u64))?;
            let read = source.read(&mut buffer[..wanted])?;
            if read == 0 {
                bail!("NODE_MANAGED_FILE_COPY_UNEXPECTED_EOF");
            }
            digest.update(&buffer[..read]);

            let mut unwritten = &buffer[..read];
            while !unwritten.is_empty() {
                ensure_current()?;
                let written = self.file.write(unwritten)?;
                if written == 0 {
                    return Err(anyhow!("NODE_MANAGED_FILE_COPY_WRITE_ZERO"));
                }
                written_total = written_total
                    .checked_add(u64::try_from(written)?)
                    .ok_or_else(|| anyhow!("NODE_MANAGED_FILE_COPY_SIZE_OVERFLOW"))?;
                unwritten = &unwritten[written..];
            }
            remaining = remaining
                .checked_sub(u64::try_from(read)?)
                .ok_or_else(|| anyhow!("NODE_MANAGED_FILE_COPY_SIZE_UNDERFLOW"))?;
        }

        ensure_current()?;
        let mut trailing = [0_u8; 1];
        if source.read(&mut trailing)? != 0 {
            bail!("NODE_MANAGED_FILE_COPY_TRAILING_DATA");
        }
        if written_total != expected_len {
            bail!("NODE_MANAGED_FILE_COPY_LENGTH_MISMATCH");
        }
        self.file.flush()?;
        self.file.sync_all()?;
        self.revalidate_exact_len(expected_len)?;
        ensure_current()?;
        Ok(ManagedFileCopyResult {
            digest: hex::encode(digest.finalize()),
            completed_at: Instant::now(),
        })
    }
}
