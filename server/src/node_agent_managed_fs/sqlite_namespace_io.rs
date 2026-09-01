use anyhow::{bail, Result};

#[cfg(all(test, windows))]
#[derive(Debug)]
struct ManagedSqliteTruncateReturnReceiptUnavailable;

#[cfg(all(test, windows))]
impl std::fmt::Display for ManagedSqliteTruncateReturnReceiptUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("NODE_MANAGED_SQLITE_TRUNCATE_RETURN_RECEIPT_UNAVAILABLE")
    }
}

#[cfg(all(test, windows))]
impl std::error::Error for ManagedSqliteTruncateReturnReceiptUnavailable {}

#[cfg(all(test, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteTruncateOutcomeUnavailableReceiptV1 {
    requested_size: u64,
    native_attempts: u8,
    exact_call_occurrence: u32,
}

#[cfg(all(test, windows))]
impl ManagedSqliteTruncateOutcomeUnavailableReceiptV1 {
    pub(crate) const fn requested_size(self) -> u64 {
        self.requested_size
    }

    pub(crate) const fn native_attempts(self) -> u8 {
        self.native_attempts
    }

    pub(crate) const fn exact_call_occurrence(self) -> u32 {
        self.exact_call_occurrence
    }
}

use super::{
    platform, same_file_identity, validate_regular_file_identity, ManagedSqliteAccess,
    ManagedSqliteFileKind, PinnedManagedSqliteFile,
};

impl PinnedManagedSqliteFile {
    pub(crate) fn kind(&self) -> ManagedSqliteFileKind {
        self.kind
    }

    pub(crate) fn was_created(&self) -> bool {
        self.created
    }

    pub(crate) fn identity_digest(&self) -> &str {
        &self.identity_digest
    }

    /// Reads from one explicit byte offset. A short read is successful, and every unread byte in
    /// the caller's buffer is zero-filled to match SQLite's xRead contract.
    pub(crate) fn read_at_zero_filled(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        validate_platform_range(
            offset,
            u64::try_from(buffer.len())?,
            "NODE_MANAGED_SQLITE_READ_RANGE_OVERFLOW",
        )?;
        self.revalidate()?;
        buffer.fill(0);
        let mut total = 0usize;
        while total < buffer.len() {
            let current_offset = offset
                .checked_add(u64::try_from(total)?)
                .ok_or_else(|| anyhow::anyhow!("NODE_MANAGED_SQLITE_READ_RANGE_OVERFLOW"))?;
            let read =
                platform::read_sqlite_file_at(&self.file, &mut buffer[total..], current_offset)?;
            if read == 0 {
                break;
            }
            total = total
                .checked_add(read)
                .ok_or_else(|| anyhow::anyhow!("NODE_MANAGED_SQLITE_READ_COUNT_OVERFLOW"))?;
        }
        self.revalidate()?;
        Ok(total)
    }

    pub(crate) fn write_all_at(&mut self, offset: u64, bytes: &[u8]) -> Result<()> {
        if self.access != ManagedSqliteAccess::ReadWrite {
            bail!("NODE_MANAGED_SQLITE_FILE_READ_ONLY");
        }
        validate_platform_range(
            offset,
            u64::try_from(bytes.len())?,
            "NODE_MANAGED_SQLITE_WRITE_RANGE_OVERFLOW",
        )?;
        self.revalidate()?;
        let mut total = 0usize;
        while total < bytes.len() {
            let current_offset = offset
                .checked_add(u64::try_from(total)?)
                .ok_or_else(|| anyhow::anyhow!("NODE_MANAGED_SQLITE_WRITE_RANGE_OVERFLOW"))?;
            let written =
                platform::write_sqlite_file_at(&self.file, &bytes[total..], current_offset)?;
            if written == 0 {
                bail!("NODE_MANAGED_SQLITE_WRITE_ZERO");
            }
            total = total
                .checked_add(written)
                .ok_or_else(|| anyhow::anyhow!("NODE_MANAGED_SQLITE_WRITE_COUNT_OVERFLOW"))?;
        }
        self.revalidate()
    }

    pub(crate) fn truncate(&mut self, size: u64) -> Result<()> {
        self.truncate_with(size, |file, requested_size| {
            file.set_len(requested_size)?;
            Ok(())
        })
    }

    #[cfg(all(test, windows))]
    pub(crate) fn truncate_outcome_unavailable_for_initialization_test_v1(
        &mut self,
        size: u64,
    ) -> Result<ManagedSqliteTruncateOutcomeUnavailableReceiptV1> {
        let mut native_attempted = false;
        let outcome = self.truncate_with(size, |file, requested_size| {
            native_attempted = true;
            let _return_receipt_intentionally_unread = file.set_len(requested_size);
            Err(anyhow::Error::new(
                ManagedSqliteTruncateReturnReceiptUnavailable,
            ))
        });
        match outcome {
            Err(error)
                if native_attempted
                    && error
                        .downcast_ref::<ManagedSqliteTruncateReturnReceiptUnavailable>()
                        .is_some() =>
            {
                Ok(ManagedSqliteTruncateOutcomeUnavailableReceiptV1 {
                    requested_size: size,
                    native_attempts: 1,
                    exact_call_occurrence: 1,
                })
            }
            Err(error) => Err(error),
            Ok(()) => bail!("NODE_MANAGED_SQLITE_TRUNCATE_UNAVAILABLE_SEAM_NOT_REACHED"),
        }
    }

    fn truncate_with(
        &mut self,
        size: u64,
        native: impl FnOnce(&std::fs::File, u64) -> Result<()>,
    ) -> Result<()> {
        if self.access != ManagedSqliteAccess::ReadWrite {
            bail!("NODE_MANAGED_SQLITE_FILE_READ_ONLY");
        }
        validate_platform_range(0, size, "NODE_MANAGED_SQLITE_TRUNCATE_RANGE_OVERFLOW")?;
        self.revalidate()?;
        native(&self.file, size)?;
        self.revalidate()?;
        if self.identity.file_size != size {
            bail!("NODE_MANAGED_SQLITE_TRUNCATE_LENGTH_MISMATCH");
        }
        Ok(())
    }

    pub(crate) fn size(&mut self) -> Result<u64> {
        self.revalidate()?;
        Ok(self.identity.file_size)
    }

    pub(crate) fn full_sync(&mut self) -> Result<()> {
        if self.access != ManagedSqliteAccess::ReadWrite {
            bail!("NODE_MANAGED_SQLITE_FILE_READ_ONLY");
        }
        self.revalidate()?;
        platform::flush_sqlite_file(&self.file)?;
        self.revalidate()
    }

    pub(crate) fn revalidate(&mut self) -> Result<()> {
        let actual = platform::inspect(&self.file)?;
        validate_regular_file_identity(actual, self.namespace.root_volume_serial)?;
        if !same_file_identity(actual, self.identity) {
            bail!("NODE_MANAGED_SQLITE_FILE_IDENTITY_CHANGED");
        }
        self.identity = actual;
        Ok(())
    }
}

fn validate_platform_range(offset: u64, length: u64, error_code: &'static str) -> Result<()> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| anyhow::anyhow!(error_code))?;
    if offset > i64::MAX as u64 || end > i64::MAX as u64 {
        bail!(error_code);
    }
    Ok(())
}
