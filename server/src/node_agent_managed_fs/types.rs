use std::{fmt, fs::File, path::PathBuf};

use super::PlatformFileIdentity;

pub(crate) struct PinnedManagedRoot {
    pub(super) root_path: PathBuf,
    pub(super) root_volume_serial: u64,
    pub(super) root_identity_digest: String,
    pub(super) root_handles: Vec<File>,
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
    pub(super) directory_handles: Vec<File>,
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
    pub(crate) fn filesystem_mutated(&self) -> bool {
        self.filesystem_mutated
    }
}

/// A file handle that was opened but failed identity/type validation. It intentionally exposes no
/// file operations; retaining it prevents callers from silently closing and reopening by path.
pub(crate) struct QuarantinedManagedFile {
    pub(super) _file: File,
    pub(super) _directory_handles: Vec<File>,
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
    pub(super) _directory_handles: Vec<File>,
    pub(super) identity: PlatformFileIdentity,
    pub(super) identity_digest: String,
    pub(super) directory_filesystem_mutated: bool,
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
