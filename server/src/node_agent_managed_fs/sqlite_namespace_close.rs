use std::{error::Error as StdError, fmt, mem::ManuallyDrop, ptr};

use super::{
    lock_domain::ManagedSqliteLockOwner, platform, ManagedSqliteAccess, ManagedSqliteFileKind,
    ManagedSqliteLockFailure, ManagedSqliteNamespaceInner, ManagedSqliteUnlockTarget,
    PinnedManagedSqliteFile, PinnedManagedSqliteMainFile, PlatformFileIdentity,
    QuarantinedManagedSqliteFile,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteFileCloseFailureClass {
    PlatformUnsupported,
    OutcomeUncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteMainFileCloseFailurePhase {
    LockRelease,
    FileClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "a close receipt proves the exact managed handle was released"]
pub(crate) struct ManagedSqliteFileCloseReceipt {
    kind: ManagedSqliteFileKind,
}

impl ManagedSqliteFileCloseReceipt {
    pub(crate) fn kind(self) -> ManagedSqliteFileKind {
        self.kind
    }
}

#[must_use = "failed file close retains live or terminal handle custody"]
pub(crate) struct ManagedSqliteFileCloseFailure {
    error: std::io::Error,
    class: ManagedSqliteFileCloseFailureClass,
    custody: ManagedSqliteFileCloseCustody,
}

#[must_use = "failed rejected-handle close retains live or terminal handle custody"]
pub(crate) struct ManagedSqliteQuarantinedFileCloseFailure {
    error: std::io::Error,
    class: ManagedSqliteFileCloseFailureClass,
    custody: ManagedSqliteQuarantinedFileCloseCustody,
}

enum ManagedSqliteQuarantinedFileCloseCustody {
    Live(QuarantinedManagedSqliteFile),
    Terminal {
        _raw_handle: usize,
        _namespace: std::sync::Arc<ManagedSqliteNamespaceInner>,
        _kind: ManagedSqliteFileKind,
    },
}

enum ManagedSqliteFileCloseCustody {
    Live(PinnedManagedSqliteFile),
    Terminal(QuarantinedManagedSqliteClosedFile),
}

struct QuarantinedManagedSqliteClosedFile {
    _raw_handle: usize,
    _namespace: std::sync::Arc<ManagedSqliteNamespaceInner>,
    _kind: ManagedSqliteFileKind,
    _access: ManagedSqliteAccess,
    _identity: PlatformFileIdentity,
    _identity_digest: String,
    _created: bool,
}

#[must_use = "a main close receipt proves locks and the exact handle were released"]
pub(crate) struct ManagedSqliteMainFileCloseReceipt {
    file: ManagedSqliteFileCloseReceipt,
}

impl ManagedSqliteMainFileCloseReceipt {
    pub(crate) fn kind(&self) -> ManagedSqliteFileKind {
        self.file.kind()
    }
}

#[must_use = "failed main-file close retains live or terminal file and lock-domain custody"]
pub(crate) struct ManagedSqliteMainFileCloseFailure {
    phase: ManagedSqliteMainFileCloseFailurePhase,
    lock_failure: Option<ManagedSqliteLockFailure>,
    file_failure: Option<ManagedSqliteFileCloseFailure>,
    // Field order is deliberate: uncertain file custody is forgotten before its permanently
    // terminal owner tombstone, while live File custody closes before owner unregistration.
    terminal_main_file: Option<ManuallyDrop<PinnedManagedSqliteFile>>,
    terminal_owner: Option<ManuallyDrop<ManagedSqliteLockOwner>>,
    live_owner: Option<ManagedSqliteLockOwner>,
    main: Option<PinnedManagedSqliteMainFile>,
}

impl PinnedManagedSqliteFile {
    pub(crate) fn close(
        self,
    ) -> Result<ManagedSqliteFileCloseReceipt, ManagedSqliteFileCloseFailure> {
        let Self {
            file,
            namespace,
            kind,
            access,
            identity,
            identity_digest,
            created,
        } = self;
        match platform::close_sqlite_file(file) {
            Ok(()) => Ok(ManagedSqliteFileCloseReceipt { kind }),
            Err(platform_failure) => {
                let error = platform_failure.error;
                let (class, custody) = match platform_failure.custody {
                    platform::PlatformManagedSqliteCloseCustody::Unattempted(file) => (
                        ManagedSqliteFileCloseFailureClass::PlatformUnsupported,
                        ManagedSqliteFileCloseCustody::Live(Self {
                            file,
                            namespace,
                            kind,
                            access,
                            identity,
                            identity_digest,
                            created,
                        }),
                    ),
                    platform::PlatformManagedSqliteCloseCustody::OutcomeUncertainRawHandle(
                        raw_handle,
                    ) => (
                        ManagedSqliteFileCloseFailureClass::OutcomeUncertain,
                        ManagedSqliteFileCloseCustody::Terminal(
                            QuarantinedManagedSqliteClosedFile {
                                _raw_handle: raw_handle,
                                _namespace: namespace,
                                _kind: kind,
                                _access: access,
                                _identity: identity,
                                _identity_digest: identity_digest,
                                _created: created,
                            },
                        ),
                    ),
                };
                Err(ManagedSqliteFileCloseFailure {
                    error,
                    class,
                    custody,
                })
            }
        }
    }
}

impl QuarantinedManagedSqliteFile {
    pub(crate) fn close(
        self,
    ) -> Result<ManagedSqliteFileCloseReceipt, ManagedSqliteQuarantinedFileCloseFailure> {
        let Self {
            _file: file,
            _namespace: namespace,
            kind,
        } = self;
        match platform::close_sqlite_file(file) {
            Ok(()) => Ok(ManagedSqliteFileCloseReceipt { kind }),
            Err(platform_failure) => {
                let error = platform_failure.error;
                let (class, custody) = match platform_failure.custody {
                    platform::PlatformManagedSqliteCloseCustody::Unattempted(file) => (
                        ManagedSqliteFileCloseFailureClass::PlatformUnsupported,
                        ManagedSqliteQuarantinedFileCloseCustody::Live(Self {
                            _file: file,
                            _namespace: namespace,
                            kind,
                        }),
                    ),
                    platform::PlatformManagedSqliteCloseCustody::OutcomeUncertainRawHandle(
                        raw_handle,
                    ) => (
                        ManagedSqliteFileCloseFailureClass::OutcomeUncertain,
                        ManagedSqliteQuarantinedFileCloseCustody::Terminal {
                            _raw_handle: raw_handle,
                            _namespace: namespace,
                            _kind: kind,
                        },
                    ),
                };
                Err(ManagedSqliteQuarantinedFileCloseFailure {
                    error,
                    class,
                    custody,
                })
            }
        }
    }
}

impl PinnedManagedSqliteMainFile {
    pub(crate) fn close(
        mut self,
    ) -> Result<ManagedSqliteMainFileCloseReceipt, ManagedSqliteMainFileCloseFailure> {
        if let Err(lock_failure) = self.unlock_to(ManagedSqliteUnlockTarget::None) {
            let terminal = lock_failure.is_terminal();
            let (terminal_main_file, terminal_owner, main) = if terminal {
                let (file, owner) = into_main_parts(self);
                (
                    Some(ManuallyDrop::new(file)),
                    Some(owner.into_terminal_tombstone()),
                    None,
                )
            } else {
                (None, None, Some(self))
            };
            return Err(ManagedSqliteMainFileCloseFailure {
                phase: ManagedSqliteMainFileCloseFailurePhase::LockRelease,
                lock_failure: Some(lock_failure),
                file_failure: None,
                terminal_main_file,
                terminal_owner,
                live_owner: None,
                main,
            });
        }

        let (file, owner) = into_main_parts(self);
        match file.close() {
            Ok(file) => {
                drop(owner);
                Ok(ManagedSqliteMainFileCloseReceipt { file })
            }
            Err(file_failure) => {
                let outcome_uncertain = file_failure.close_outcome_uncertain();
                let (terminal_owner, live_owner) = if outcome_uncertain {
                    (Some(owner.into_terminal_tombstone()), None)
                } else {
                    (None, Some(owner))
                };
                Err(ManagedSqliteMainFileCloseFailure {
                    phase: ManagedSqliteMainFileCloseFailurePhase::FileClose,
                    lock_failure: None,
                    file_failure: Some(file_failure),
                    terminal_main_file: None,
                    terminal_owner,
                    live_owner,
                    main: None,
                })
            }
        }
    }
}

fn into_main_parts(
    main: PinnedManagedSqliteMainFile,
) -> (PinnedManagedSqliteFile, ManagedSqliteLockOwner) {
    let main = ManuallyDrop::new(main);
    // SAFETY: ManuallyDrop suppresses the Drop body and field destruction. Each field is read once
    // and returned as the unique owner; no reference into `main` escapes.
    unsafe { (ptr::read(&main.file), ptr::read(&main.lock_owner)) }
}

impl ManagedSqliteFileCloseFailure {
    pub(crate) fn class(&self) -> ManagedSqliteFileCloseFailureClass {
        self.class
    }

    pub(crate) fn close_outcome_uncertain(&self) -> bool {
        self.class == ManagedSqliteFileCloseFailureClass::OutcomeUncertain
    }

    pub(crate) fn error_kind(&self) -> std::io::ErrorKind {
        self.error.kind()
    }

    pub(crate) fn raw_os_error(&self) -> Option<i32> {
        self.error.raw_os_error()
    }

    pub(crate) fn into_file(self) -> Result<PinnedManagedSqliteFile, Self> {
        let Self {
            error,
            class,
            custody,
        } = self;
        match custody {
            ManagedSqliteFileCloseCustody::Live(file) => Ok(file),
            custody @ ManagedSqliteFileCloseCustody::Terminal(_) => Err(Self {
                error,
                class,
                custody,
            }),
        }
    }
}

impl ManagedSqliteQuarantinedFileCloseFailure {
    pub(crate) fn class(&self) -> ManagedSqliteFileCloseFailureClass {
        self.class
    }

    pub(crate) fn close_outcome_uncertain(&self) -> bool {
        self.class == ManagedSqliteFileCloseFailureClass::OutcomeUncertain
    }

    pub(crate) fn error_kind(&self) -> std::io::ErrorKind {
        self.error.kind()
    }

    pub(crate) fn raw_os_error(&self) -> Option<i32> {
        self.error.raw_os_error()
    }

    pub(crate) fn into_file(self) -> Result<QuarantinedManagedSqliteFile, Self> {
        let Self {
            error,
            class,
            custody,
        } = self;
        match custody {
            ManagedSqliteQuarantinedFileCloseCustody::Live(file) => Ok(file),
            custody @ ManagedSqliteQuarantinedFileCloseCustody::Terminal { .. } => Err(Self {
                error,
                class,
                custody,
            }),
        }
    }
}

impl ManagedSqliteMainFileCloseFailure {
    pub(crate) fn phase(&self) -> ManagedSqliteMainFileCloseFailurePhase {
        self.phase
    }

    pub(crate) fn close_outcome_uncertain(&self) -> bool {
        self.terminal_main_file.is_some()
            || self
                .lock_failure
                .as_ref()
                .is_some_and(ManagedSqliteLockFailure::is_terminal)
            || self
                .file_failure
                .as_ref()
                .is_some_and(ManagedSqliteFileCloseFailure::close_outcome_uncertain)
    }

    pub(crate) fn into_main(mut self) -> Result<PinnedManagedSqliteMainFile, Self> {
        if let Some(main) = self.main.take() {
            return Ok(main);
        }
        if self.terminal_owner.is_some() {
            return Err(self);
        }
        let Some(file_failure) = self.file_failure.take() else {
            return Err(self);
        };
        match file_failure.into_file() {
            Ok(file) => {
                let Some(lock_owner) = self.live_owner.take() else {
                    self.file_failure = Some(ManagedSqliteFileCloseFailure {
                        error: std::io::Error::other(
                            "NODE_MANAGED_SQLITE_CLOSE_LIVE_OWNER_MISSING",
                        ),
                        class: ManagedSqliteFileCloseFailureClass::PlatformUnsupported,
                        custody: ManagedSqliteFileCloseCustody::Live(file),
                    });
                    return Err(self);
                };
                Ok(PinnedManagedSqliteMainFile { file, lock_owner })
            }
            Err(file_failure) => {
                self.file_failure = Some(file_failure);
                Err(self)
            }
        }
    }
}

impl fmt::Debug for ManagedSqliteFileCloseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteFileCloseFailure")
            .field("class", &self.class)
            .field("error_kind", &self.error.kind())
            .field("raw_os_error", &self.error.raw_os_error())
            .field("custody", &"<retained>")
            .finish()
    }
}

impl fmt::Display for ManagedSqliteFileCloseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NODE_MANAGED_SQLITE_FILE_CLOSE_FAILED")
    }
}

impl StdError for ManagedSqliteFileCloseFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}

impl fmt::Debug for ManagedSqliteQuarantinedFileCloseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteQuarantinedFileCloseFailure")
            .field("class", &self.class)
            .field("error_kind", &self.error.kind())
            .field("raw_os_error", &self.error.raw_os_error())
            .field("custody", &"<retained>")
            .finish()
    }
}

impl fmt::Display for ManagedSqliteQuarantinedFileCloseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NODE_MANAGED_SQLITE_REJECTED_FILE_CLOSE_FAILED")
    }
}

impl StdError for ManagedSqliteQuarantinedFileCloseFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}

impl fmt::Debug for ManagedSqliteMainFileCloseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteMainFileCloseFailure")
            .field("phase", &self.phase)
            .field("lock_failure", &self.lock_failure)
            .field("file_failure", &self.file_failure)
            .field("custody", &"<retained>")
            .finish()
    }
}

impl fmt::Display for ManagedSqliteMainFileCloseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NODE_MANAGED_SQLITE_MAIN_FILE_CLOSE_FAILED")
    }
}

impl StdError for ManagedSqliteMainFileCloseFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.lock_failure
            .as_ref()
            .map(|failure| failure as &(dyn StdError + 'static))
            .or_else(|| {
                self.file_failure
                    .as_ref()
                    .map(|failure| failure as &(dyn StdError + 'static))
            })
    }
}
