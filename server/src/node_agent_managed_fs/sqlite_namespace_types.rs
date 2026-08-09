use std::{error::Error as StdError, ffi::OsStr, fmt, fs::File, sync::Arc};

use super::super::{
    namespace::ManagedObjectBinding, PinnedManagedDirectory, PlatformFileIdentity,
    PlatformNamespaceFlushFailureKind,
};
use super::close::ManagedSqliteQuarantinedFileCloseFailure;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteFileKind {
    Main,
    Journal,
    Wal,
    Shm,
}

impl ManagedSqliteFileKind {
    pub(in crate::node_agent_managed_fs) fn name(self) -> &'static OsStr {
        OsStr::new(match self {
            Self::Main => "compute-plugin-state.sqlite3",
            Self::Journal => "compute-plugin-state.sqlite3-journal",
            Self::Wal => "compute-plugin-state.sqlite3-wal",
            Self::Shm => "compute-plugin-state.sqlite3-shm",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteOpenMode {
    Existing,
    OpenOrCreate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteAccess {
    ReadOnly,
    ReadWrite,
}

pub(super) struct ManagedSqliteNamespaceInner {
    pub(super) root_volume_serial: u64,
    pub(super) root_identity_digest: String,
    pub(super) directory_identity: PlatformFileIdentity,
    pub(super) directory_binding: ManagedObjectBinding,
    pub(super) directory_handles: Vec<Arc<File>>,
}

#[must_use = "dropping the SQLite namespace releases its retained parent handle chain"]
pub(crate) struct PinnedManagedSqliteNamespace {
    pub(super) inner: Arc<ManagedSqliteNamespaceInner>,
}

#[must_use = "dropping the SQLite file releases its retained file and parent handles"]
pub(crate) struct PinnedManagedSqliteFile {
    pub(super) file: File,
    pub(super) namespace: Arc<ManagedSqliteNamespaceInner>,
    pub(super) kind: ManagedSqliteFileKind,
    pub(super) access: ManagedSqliteAccess,
    pub(super) identity: PlatformFileIdentity,
    pub(super) identity_digest: String,
    pub(super) created: bool,
}

#[must_use = "rejected SQLite handles must remain retained until their outcome is classified"]
pub(crate) struct QuarantinedManagedSqliteFile {
    pub(super) _file: File,
    pub(super) _namespace: Arc<ManagedSqliteNamespaceInner>,
    pub(super) kind: ManagedSqliteFileKind,
}

pub(crate) struct ManagedSqliteNamespaceBindFailure {
    error: std::io::Error,
    _directory: PinnedManagedDirectory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteFileOpenFailurePhase {
    ParentValidation,
    PlatformOpen,
    OpenCompletion,
    FileValidation,
    HandleClose,
}

pub(crate) struct ManagedSqliteFileOpenFailure {
    phase: ManagedSqliteFileOpenFailurePhase,
    error: std::io::Error,
    _custody: Option<QuarantinedManagedSqliteFile>,
    close_custody: Option<ManagedSqliteQuarantinedFileCloseFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteDeleteFailurePhase {
    ParentValidation,
    PlatformOpen,
    OpenCompletion,
    FileValidation,
    Disposition,
    PreBarrierObservation,
    ParentBarrier,
    PostBarrierObservation,
    PostDispositionParentValidation,
    PostBarrierParentValidation,
    PostDispositionHandleClose,
    PreBarrierObservationHandleClose,
    PostBarrierObservationHandleClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteDirectoryBarrierFailureKind {
    RetryableBeforeBarrier,
    OutcomeUncertain,
    PlatformUnsupported,
}

pub(crate) struct ManagedSqliteDeleteFailure {
    phase: ManagedSqliteDeleteFailurePhase,
    error: std::io::Error,
    barrier_failure_kind: Option<ManagedSqliteDirectoryBarrierFailureKind>,
    _custody: Option<QuarantinedManagedSqliteFile>,
    close_custody: Option<ManagedSqliteQuarantinedFileCloseFailure>,
}

pub(super) struct ManagedSqliteFailureHandleCustody {
    pub(super) live: Option<QuarantinedManagedSqliteFile>,
    pub(super) close_failure: Option<ManagedSqliteQuarantinedFileCloseFailure>,
}

pub(super) struct ManagedSqliteFileOpenFailureParts {
    pub(super) error: std::io::Error,
    pub(super) custody: ManagedSqliteFailureHandleCustody,
}

pub(super) struct ManagedSqliteDeleteFailureParts {
    pub(super) error: std::io::Error,
    pub(super) mutation_may_have_occurred: bool,
    pub(super) custody: ManagedSqliteFailureHandleCustody,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteDeleteOutcome {
    Deleted,
    NotFound,
}

impl ManagedSqliteNamespaceBindFailure {
    pub(super) fn new(error: impl Into<std::io::Error>, directory: PinnedManagedDirectory) -> Self {
        Self {
            error: error.into(),
            _directory: directory,
        }
    }
}

impl ManagedSqliteFileOpenFailure {
    pub(super) fn not_opened(
        phase: ManagedSqliteFileOpenFailurePhase,
        error: impl Into<std::io::Error>,
    ) -> Self {
        Self {
            phase,
            error: error.into(),
            _custody: None,
            close_custody: None,
        }
    }

    pub(super) fn opened_rejected(
        phase: ManagedSqliteFileOpenFailurePhase,
        error: impl Into<std::io::Error>,
        custody: QuarantinedManagedSqliteFile,
    ) -> Self {
        Self {
            phase,
            error: error.into(),
            _custody: Some(custody),
            close_custody: None,
        }
    }

    pub(super) fn close_failed(custody: ManagedSqliteQuarantinedFileCloseFailure) -> Self {
        Self {
            phase: ManagedSqliteFileOpenFailurePhase::HandleClose,
            error: std::io::Error::other("NODE_MANAGED_SQLITE_OBSERVATION_HANDLE_CLOSE_FAILED"),
            _custody: None,
            close_custody: Some(custody),
        }
    }

    pub(crate) fn phase(&self) -> ManagedSqliteFileOpenFailurePhase {
        self.phase
    }

    pub(crate) fn handle_retained(&self) -> bool {
        self._custody.is_some() || self.close_custody.is_some()
    }

    pub(crate) fn into_retained_file(mut self) -> Result<QuarantinedManagedSqliteFile, Self> {
        match self._custody.take() {
            Some(file) => Ok(file),
            None => Err(self),
        }
    }

    pub(crate) fn close_failure(&self) -> Option<&ManagedSqliteQuarantinedFileCloseFailure> {
        self.close_custody.as_ref()
    }

    pub(crate) fn into_close_failure(
        mut self,
    ) -> Result<ManagedSqliteQuarantinedFileCloseFailure, Self> {
        match self.close_custody.take() {
            Some(failure) => Ok(failure),
            None => Err(self),
        }
    }

    pub(super) fn into_shm_parts(self) -> ManagedSqliteFileOpenFailureParts {
        ManagedSqliteFileOpenFailureParts {
            error: self.error,
            custody: ManagedSqliteFailureHandleCustody {
                live: self._custody,
                close_failure: self.close_custody,
            },
        }
    }
}

impl ManagedSqliteDeleteFailure {
    pub(super) fn new(
        phase: ManagedSqliteDeleteFailurePhase,
        error: impl Into<std::io::Error>,
        custody: Option<QuarantinedManagedSqliteFile>,
    ) -> Self {
        Self {
            phase,
            error: error.into(),
            barrier_failure_kind: None,
            _custody: custody,
            close_custody: None,
        }
    }

    pub(super) fn barrier(error: std::io::Error, kind: PlatformNamespaceFlushFailureKind) -> Self {
        let kind = match kind {
            PlatformNamespaceFlushFailureKind::RetryableBeforeBarrier => {
                ManagedSqliteDirectoryBarrierFailureKind::RetryableBeforeBarrier
            }
            PlatformNamespaceFlushFailureKind::OutcomeUncertain => {
                ManagedSqliteDirectoryBarrierFailureKind::OutcomeUncertain
            }
            PlatformNamespaceFlushFailureKind::PlatformUnsupported => {
                ManagedSqliteDirectoryBarrierFailureKind::PlatformUnsupported
            }
        };
        Self {
            phase: ManagedSqliteDeleteFailurePhase::ParentBarrier,
            error,
            barrier_failure_kind: Some(kind),
            _custody: None,
            close_custody: None,
        }
    }

    pub(super) fn close_failed(
        phase: ManagedSqliteDeleteFailurePhase,
        custody: ManagedSqliteQuarantinedFileCloseFailure,
    ) -> Self {
        Self {
            phase,
            error: std::io::Error::other("NODE_MANAGED_SQLITE_DELETE_HANDLE_CLOSE_FAILED"),
            barrier_failure_kind: None,
            _custody: None,
            close_custody: Some(custody),
        }
    }

    pub(crate) fn phase(&self) -> ManagedSqliteDeleteFailurePhase {
        self.phase
    }

    pub(crate) fn barrier_failure_kind(&self) -> Option<ManagedSqliteDirectoryBarrierFailureKind> {
        self.barrier_failure_kind
    }

    pub(crate) fn handle_retained(&self) -> bool {
        self._custody.is_some() || self.close_custody.is_some()
    }

    pub(crate) fn into_retained_file(mut self) -> Result<QuarantinedManagedSqliteFile, Self> {
        match self._custody.take() {
            Some(file) => Ok(file),
            None => Err(self),
        }
    }

    pub(crate) fn close_failure(&self) -> Option<&ManagedSqliteQuarantinedFileCloseFailure> {
        self.close_custody.as_ref()
    }

    pub(crate) fn into_close_failure(
        mut self,
    ) -> Result<ManagedSqliteQuarantinedFileCloseFailure, Self> {
        match self.close_custody.take() {
            Some(failure) => Ok(failure),
            None => Err(self),
        }
    }

    pub(crate) fn mutation_may_have_occurred(&self) -> bool {
        matches!(
            self.phase,
            ManagedSqliteDeleteFailurePhase::Disposition
                | ManagedSqliteDeleteFailurePhase::PreBarrierObservation
                | ManagedSqliteDeleteFailurePhase::ParentBarrier
                | ManagedSqliteDeleteFailurePhase::PostBarrierObservation
                | ManagedSqliteDeleteFailurePhase::PostDispositionParentValidation
                | ManagedSqliteDeleteFailurePhase::PostBarrierParentValidation
                | ManagedSqliteDeleteFailurePhase::PostDispositionHandleClose
                | ManagedSqliteDeleteFailurePhase::PreBarrierObservationHandleClose
                | ManagedSqliteDeleteFailurePhase::PostBarrierObservationHandleClose
        )
    }

    pub(crate) fn directory_barrier_proven_completed(&self) -> bool {
        matches!(
            self.phase,
            ManagedSqliteDeleteFailurePhase::PostBarrierObservation
                | ManagedSqliteDeleteFailurePhase::PostBarrierParentValidation
                | ManagedSqliteDeleteFailurePhase::PostBarrierObservationHandleClose
        )
    }

    pub(super) fn into_shm_parts(self) -> ManagedSqliteDeleteFailureParts {
        let mutation_may_have_occurred = self.mutation_may_have_occurred();
        ManagedSqliteDeleteFailureParts {
            error: self.error,
            mutation_may_have_occurred,
            custody: ManagedSqliteFailureHandleCustody {
                live: self._custody,
                close_failure: self.close_custody,
            },
        }
    }
}

impl fmt::Debug for PinnedManagedSqliteNamespace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedSqliteNamespace")
            .field("parent", &"<retained>")
            .field("binding", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for PinnedManagedSqliteFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedSqliteFile")
            .field("kind", &self.kind)
            .field("access", &self.access)
            .field("identity", &"<redacted>")
            .field("created", &self.created)
            .finish()
    }
}

impl fmt::Debug for QuarantinedManagedSqliteFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuarantinedManagedSqliteFile")
            .field("kind", &self.kind)
            .field("file", &"<retained>")
            .field("parent", &"<retained>")
            .finish()
    }
}

macro_rules! impl_failure {
    ($type:ty, $code:literal) => {
        impl fmt::Debug for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($type))
                    .field("error_kind", &self.error.kind())
                    .field("raw_os_error", &self.error.raw_os_error())
                    .finish()
            }
        }

        impl fmt::Display for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str($code)
            }
        }

        impl StdError for $type {
            fn source(&self) -> Option<&(dyn StdError + 'static)> {
                Some(&self.error)
            }
        }
    };
}

impl_failure!(
    ManagedSqliteNamespaceBindFailure,
    "NODE_MANAGED_SQLITE_NAMESPACE_BIND_FAILED"
);
impl_failure!(
    ManagedSqliteFileOpenFailure,
    "NODE_MANAGED_SQLITE_FILE_OPEN_FAILED"
);
impl_failure!(
    ManagedSqliteDeleteFailure,
    "NODE_MANAGED_SQLITE_DELETE_FAILED"
);
