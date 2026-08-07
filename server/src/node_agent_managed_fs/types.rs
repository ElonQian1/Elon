use std::{error::Error as StdError, fmt, fs::File, path::PathBuf, sync::Arc};

use super::{namespace::ManagedObjectBinding, PlatformFileIdentity};

pub(crate) struct PinnedManagedRoot {
    pub(super) root_path: PathBuf,
    pub(super) root_volume_serial: u64,
    pub(super) installation_binding_digest: String,
    pub(super) root_identity_digest: String,
    pub(super) root_handles: Vec<Arc<File>>,
}

impl fmt::Debug for PinnedManagedRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedRoot")
            .field("root_path", &"<redacted>")
            .field("root_identity_digest", &"<redacted>")
            .field("pinned_prefixes", &self.root_handles.len())
            .finish()
    }
}

pub(crate) struct PinnedManagedDirectory {
    pub(super) path: PathBuf,
    pub(super) root_volume_serial: u64,
    pub(super) root_identity_digest: String,
    pub(super) directory_handles: Vec<Arc<File>>,
    pub(super) binding: Option<ManagedObjectBinding>,
    pub(super) filesystem_mutated: bool,
}

impl fmt::Debug for PinnedManagedDirectory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedDirectory")
            .field("path", &"<redacted>")
            .field("pinned_prefixes", &self.directory_handles.len())
            .field("filesystem_mutated", &self.filesystem_mutated)
            .finish()
    }
}

impl PinnedManagedDirectory {
    pub(crate) fn object_binding(&self) -> Option<&ManagedObjectBinding> {
        self.binding.as_ref()
    }

    pub(crate) fn filesystem_mutated(&self) -> bool {
        self.filesystem_mutated
    }
}

/// A file handle that was opened but failed identity/type validation. It intentionally exposes no
/// file operations; retaining it prevents callers from silently closing and reopening by path.
pub(crate) struct QuarantinedManagedFile {
    pub(super) _file: File,
    pub(super) _directory_handles: Vec<Arc<File>>,
    pub(super) directory_filesystem_mutated: bool,
}

impl fmt::Debug for QuarantinedManagedFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuarantinedManagedFile")
            .field("file", &"<retained>")
            .field("directory_handles", &"<retained>")
            .finish()
    }
}

impl QuarantinedManagedFile {
    pub(crate) fn directory_filesystem_mutated(&self) -> bool {
        self.directory_filesystem_mutated
    }
}

pub(crate) struct PinnedManagedFile {
    pub(super) file: File,
    pub(super) _directory_handles: Vec<Arc<File>>,
    pub(super) identity: PlatformFileIdentity,
    pub(super) identity_digest: String,
    pub(super) binding: ManagedObjectBinding,
    pub(super) directory_filesystem_mutated: bool,
}

/// Opaque ownership of one share-none file handle opened below an already pinned directory.
/// Holding this value is the lock; dropping it releases the operating-system exclusion but never
/// removes the persistent lock file. No raw file or mutation interface is exposed to callers.
#[must_use = "dropping the managed exclusive file lock releases operating-system exclusion"]
pub(crate) struct PinnedManagedExclusiveFileLock {
    pub(super) _file: File,
    pub(super) _directory_handles: Vec<Arc<File>>,
    pub(super) identity_digest: String,
}

impl fmt::Debug for PinnedManagedExclusiveFileLock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedExclusiveFileLock")
            .field("file", &"<exclusive-retained>")
            .field("directory_handles", &"<retained>")
            .field("identity_digest", &"<redacted>")
            .finish()
    }
}

/// Fail-closed result of acquiring an exclusive managed file lock. Once a handle was opened but
/// could not be proven to be one regular, non-reparse, single-link file on the pinned volume, the
/// rejected handle remains quarantined in this error for its lifetime.
pub(crate) struct ManagedExclusiveFileLockFailure {
    inner: ManagedExclusiveFileLockFailureInner,
}

enum ManagedExclusiveFileLockFailureInner {
    NotAcquired {
        error: std::io::Error,
        _directory: PinnedManagedDirectory,
    },
    OpenedRejected {
        error: anyhow::Error,
        _file: QuarantinedManagedFile,
    },
}

impl ManagedExclusiveFileLockFailure {
    pub(super) fn not_acquired(error: std::io::Error, directory: PinnedManagedDirectory) -> Self {
        Self {
            inner: ManagedExclusiveFileLockFailureInner::NotAcquired {
                error,
                _directory: directory,
            },
        }
    }

    pub(super) fn opened_rejected(error: anyhow::Error, file: QuarantinedManagedFile) -> Self {
        Self {
            inner: ManagedExclusiveFileLockFailureInner::OpenedRejected { error, _file: file },
        }
    }
}

impl fmt::Debug for ManagedExclusiveFileLockFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            ManagedExclusiveFileLockFailureInner::NotAcquired { error, .. } => formatter
                .debug_struct("ManagedExclusiveFileLockFailure")
                .field("phase", &"not_acquired")
                .field("error_kind", &error.kind())
                .field("raw_os_error", &error.raw_os_error())
                .field("directory", &"<retained>")
                .finish(),
            ManagedExclusiveFileLockFailureInner::OpenedRejected { .. } => formatter
                .debug_struct("ManagedExclusiveFileLockFailure")
                .field("phase", &"opened_rejected")
                .field("file", &"<quarantined-retained>")
                .finish(),
        }
    }
}

impl fmt::Display for ManagedExclusiveFileLockFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match &self.inner {
            ManagedExclusiveFileLockFailureInner::NotAcquired { .. } => {
                "NODE_MANAGED_EXCLUSIVE_FILE_LOCK_NOT_ACQUIRED"
            }
            ManagedExclusiveFileLockFailureInner::OpenedRejected { .. } => {
                "NODE_MANAGED_EXCLUSIVE_FILE_LOCK_OPENED_REJECTED"
            }
        };
        formatter.write_str(code)
    }
}

impl StdError for ManagedExclusiveFileLockFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &self.inner {
            ManagedExclusiveFileLockFailureInner::NotAcquired { error, .. } => Some(error),
            ManagedExclusiveFileLockFailureInner::OpenedRejected { error, .. } => {
                Some(error.as_ref())
            }
        }
    }
}

impl fmt::Debug for PinnedManagedFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedFile")
            .field("file", &"<retained>")
            .field("identity_digest", &"<redacted>")
            .field("file_size", &self.identity.file_size)
            .finish()
    }
}

pub(crate) enum ManagedFileOpenFailure {
    NotOpened(std::io::Error),
    FileNotOpened {
        error: std::io::Error,
        directory: PinnedManagedDirectory,
    },
    Opened {
        error: anyhow::Error,
        file: QuarantinedManagedFile,
    },
}

impl fmt::Debug for ManagedFileOpenFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOpened(error) => formatter
                .debug_struct("ManagedFileOpenFailure")
                .field("phase", &"not_opened")
                .field("error_kind", &error.kind())
                .finish(),
            Self::FileNotOpened { error, directory } => formatter
                .debug_struct("ManagedFileOpenFailure")
                .field("phase", &"file_not_opened")
                .field("error_kind", &error.kind())
                .field("directory_mutated", &directory.filesystem_mutated)
                .finish(),
            Self::Opened { .. } => formatter
                .debug_struct("ManagedFileOpenFailure")
                .field("phase", &"opened_rejected")
                .field("file", &"<retained>")
                .finish(),
        }
    }
}

pub(crate) enum ManagedDirectoryPrepareFailure {
    Unchanged(anyhow::Error),
    Mutated(anyhow::Error),
}

impl ManagedDirectoryPrepareFailure {
    pub(crate) fn filesystem_mutated(&self) -> bool {
        matches!(self, Self::Mutated(_))
    }

    pub(crate) fn into_error(self) -> anyhow::Error {
        match self {
            Self::Unchanged(error) | Self::Mutated(error) => error,
        }
    }
}

impl fmt::Debug for ManagedDirectoryPrepareFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedDirectoryPrepareFailure")
            .field("filesystem_mutated", &self.filesystem_mutated())
            .finish()
    }
}
