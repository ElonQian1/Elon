use std::{error::Error as StdError, fmt};

use anyhow::{bail, Result};

use super::{
    lock_domain::{register_lock_owner, ManagedSqliteLockDomainGuard, ManagedSqliteLockOwner},
    ManagedSqliteFileKind, PinnedManagedSqliteFile,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteRequestedLock {
    Shared,
    Reserved,
    Exclusive,
}

impl ManagedSqliteRequestedLock {
    pub(super) fn rank(self) -> u8 {
        match self {
            Self::Shared => 1,
            Self::Reserved => 2,
            Self::Exclusive => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteUnlockTarget {
    None,
    Shared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteObservedLock {
    None,
    Shared,
    Reserved,
    Pending,
    Exclusive,
}

impl ManagedSqliteObservedLock {
    pub(super) fn rank(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Shared => 1,
            Self::Reserved => 2,
            Self::Pending => 3,
            Self::Exclusive => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteLockAttempt {
    Acquired,
    Contended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlatformManagedSqliteLockAttempt {
    Acquired,
    Contended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteLockFailureKind {
    InvalidTransition,
    ReadOnly,
    Platform,
    StateUncertain,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteLockFailurePhase {
    Gate,
    RequestValidation,
    AcquirePending,
    ReleaseTemporaryPending,
    AcquireShared,
    AcquireReserved,
    ReleaseSharedForExclusive,
    AcquireExclusive,
    RestoreShared,
    ReleaseExclusive,
    ReleaseReserved,
    ReleaseShared,
    ReleasePending,
    ReservedProbe,
    ReservedProbeRelease,
}

pub(crate) struct ManagedSqliteLockFailure {
    pub(super) phase: ManagedSqliteLockFailurePhase,
    pub(super) kind: ManagedSqliteLockFailureKind,
    pub(super) error: std::io::Error,
    pub(super) terminal: bool,
}

pub(super) fn invalid_lock_failure(code: &'static str) -> ManagedSqliteLockFailure {
    ManagedSqliteLockFailure::message(
        ManagedSqliteLockFailurePhase::RequestValidation,
        ManagedSqliteLockFailureKind::InvalidTransition,
        code,
        false,
    )
}

pub(super) fn platform_lock_failure(
    phase: ManagedSqliteLockFailurePhase,
    error: std::io::Error,
) -> ManagedSqliteLockFailure {
    ManagedSqliteLockFailure::new(phase, ManagedSqliteLockFailureKind::Platform, error, false)
}

/// A consuming, non-forgeable upgrade of one pinned SQLite main-database file.
///
/// Field order is deliberate: Drop explicitly releases proven locks, then the File closes before
/// its independent same-process owner unregisters from the FileId lock domain.
#[must_use = "dropping the SQLite main file releases its retained file and lock custody"]
pub(crate) struct PinnedManagedSqliteMainFile {
    pub(super) file: PinnedManagedSqliteFile,
    pub(super) lock_owner: ManagedSqliteLockOwner,
}

#[must_use = "the rejected pinned file remains retained by this failure"]
pub(crate) struct ManagedSqliteMainFileBindFailure {
    error: std::io::Error,
    _file: PinnedManagedSqliteFile,
}

impl PinnedManagedSqliteFile {
    pub(crate) fn into_main_file(
        self,
    ) -> std::result::Result<PinnedManagedSqliteMainFile, ManagedSqliteMainFileBindFailure> {
        if self.kind != ManagedSqliteFileKind::Main {
            return Err(ManagedSqliteMainFileBindFailure::new(
                "NODE_MANAGED_SQLITE_MAIN_FILE_KIND_INVALID",
                self,
            ));
        }
        let lock_owner =
            match register_lock_owner(self.identity.volume_serial, self.identity.file_id) {
                Ok(owner) => owner,
                Err(error) => {
                    return Err(ManagedSqliteMainFileBindFailure { error, _file: self });
                }
            };
        Ok(PinnedManagedSqliteMainFile {
            file: self,
            lock_owner,
        })
    }
}

impl PinnedManagedSqliteMainFile {
    pub(super) fn live_lock_domain(
        &self,
    ) -> std::result::Result<ManagedSqliteLockDomainGuard<'_>, ManagedSqliteLockFailure> {
        let domain = self.lock_owner.lock().map_err(|error| {
            ManagedSqliteLockFailure::new(
                ManagedSqliteLockFailurePhase::Gate,
                ManagedSqliteLockFailureKind::Terminal,
                error,
                true,
            )
        })?;
        if domain.is_terminal() {
            return Err(ManagedSqliteLockFailure::message(
                ManagedSqliteLockFailurePhase::Gate,
                ManagedSqliteLockFailureKind::Terminal,
                "NODE_MANAGED_SQLITE_LOCK_STATE_TERMINAL",
                true,
            ));
        }
        Ok(domain)
    }

    pub(crate) fn was_created(&self) -> bool {
        self.file.was_created()
    }

    pub(crate) fn identity_digest(&self) -> &str {
        self.file.identity_digest()
    }

    pub(crate) fn read_at_zero_filled(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        self.ensure_live()?;
        self.file.read_at_zero_filled(offset, buffer)
    }

    pub(crate) fn write_all_at(&mut self, offset: u64, bytes: &[u8]) -> Result<()> {
        self.ensure_live()?;
        self.file.write_all_at(offset, bytes)
    }

    pub(crate) fn truncate(&mut self, size: u64) -> Result<()> {
        self.ensure_live()?;
        self.file.truncate(size)
    }

    pub(crate) fn size(&mut self) -> Result<u64> {
        self.ensure_live()?;
        self.file.size()
    }

    pub(crate) fn full_sync(&mut self) -> Result<()> {
        self.ensure_live()?;
        self.file.full_sync()
    }

    pub(crate) fn revalidate(&mut self) -> Result<()> {
        self.ensure_live()?;
        self.file.revalidate()
    }

    pub(crate) fn is_terminal(&self) -> bool {
        self.lock_owner
            .lock()
            .map_or(true, |domain| domain.is_terminal())
    }

    pub(super) fn is_read_write(&self) -> bool {
        self.file.access == super::ManagedSqliteAccess::ReadWrite
    }

    pub(super) fn ensure_live(&self) -> Result<()> {
        let domain = self
            .lock_owner
            .lock()
            .map_err(|_| anyhow::anyhow!("NODE_MANAGED_SQLITE_LOCK_DOMAIN_UNAVAILABLE"))?;
        if domain.is_terminal() {
            bail!("NODE_MANAGED_SQLITE_MAIN_FILE_TERMINAL");
        }
        Ok(())
    }
}

impl Drop for PinnedManagedSqliteMainFile {
    fn drop(&mut self) {
        self.release_locks_for_drop();
    }
}

impl ManagedSqliteLockFailure {
    pub(super) fn new(
        phase: ManagedSqliteLockFailurePhase,
        kind: ManagedSqliteLockFailureKind,
        error: std::io::Error,
        terminal: bool,
    ) -> Self {
        Self {
            phase,
            kind,
            error,
            terminal,
        }
    }

    pub(super) fn message(
        phase: ManagedSqliteLockFailurePhase,
        kind: ManagedSqliteLockFailureKind,
        code: &'static str,
        terminal: bool,
    ) -> Self {
        Self::new(phase, kind, std::io::Error::other(code), terminal)
    }

    pub(crate) fn phase(&self) -> ManagedSqliteLockFailurePhase {
        self.phase
    }

    pub(crate) fn kind(&self) -> ManagedSqliteLockFailureKind {
        self.kind
    }

    pub(crate) fn is_terminal(&self) -> bool {
        self.terminal
    }
}

impl ManagedSqliteMainFileBindFailure {
    fn new(code: &'static str, file: PinnedManagedSqliteFile) -> Self {
        Self {
            error: std::io::Error::new(std::io::ErrorKind::InvalidInput, code),
            _file: file,
        }
    }

    pub(crate) fn into_file(self) -> PinnedManagedSqliteFile {
        self._file
    }
}

impl fmt::Debug for PinnedManagedSqliteMainFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedSqliteMainFile")
            .field("file", &self.file)
            .field("lock_level", &self.lock_level().ok())
            .field("terminal", &self.is_terminal())
            .finish()
    }
}

impl fmt::Debug for ManagedSqliteLockFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteLockFailure")
            .field("phase", &self.phase)
            .field("kind", &self.kind)
            .field("error_kind", &self.error.kind())
            .field("raw_os_error", &self.error.raw_os_error())
            .field("terminal", &self.terminal)
            .finish()
    }
}

impl fmt::Display for ManagedSqliteLockFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NODE_MANAGED_SQLITE_LOCK_FAILED")
    }
}

impl StdError for ManagedSqliteLockFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}

impl fmt::Debug for ManagedSqliteMainFileBindFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteMainFileBindFailure")
            .field("error_kind", &self.error.kind())
            .field("file", &"<retained>")
            .finish()
    }
}

impl fmt::Display for ManagedSqliteMainFileBindFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NODE_MANAGED_SQLITE_MAIN_FILE_BIND_FAILED")
    }
}

impl StdError for ManagedSqliteMainFileBindFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}
