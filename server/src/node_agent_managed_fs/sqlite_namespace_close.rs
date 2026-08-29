use std::{error::Error as StdError, fmt, mem::ManuallyDrop};

use super::{
    lock_domain::ManagedSqliteLockOwner, platform, ManagedSqliteAccess, ManagedSqliteFileKind,
    ManagedSqliteLockFailure, ManagedSqliteNamespaceInner, PinnedManagedSqliteFile,
    PinnedManagedSqliteMainFile, QuarantinedManagedSqliteFile,
};
use crate::node_agent_managed_fs::PlatformFileIdentity;

#[path = "sqlite_namespace_close/main_close.rs"]
mod main_close;
#[cfg(test)]
#[path = "sqlite_namespace_close/test_faults.rs"]
mod test_faults;
#[cfg(test)]
pub(crate) use test_faults::{
    ManagedSqliteMainCloseTestFault, ManagedSqliteMainCloseTestFaultPhase,
    ManagedSqliteMainCloseTestFaultTiming, ManagedSqliteMainCloseTestFaults,
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

#[derive(Debug, PartialEq, Eq)]
#[must_use = "a close receipt proves the exact managed handle was released"]
pub(crate) struct ManagedSqliteFileCloseReceipt {
    kind: ManagedSqliteFileKind,
}

impl ManagedSqliteFileCloseReceipt {
    pub(crate) fn kind(&self) -> ManagedSqliteFileKind {
        self.kind
    }

    #[cfg(test)]
    pub(crate) fn test_value(kind: ManagedSqliteFileKind) -> Self {
        Self { kind }
    }
}

#[must_use = "failed file close retains live or terminal handle custody"]
pub(crate) struct ManagedSqliteFileCloseFailure {
    error: std::io::Error,
    class: ManagedSqliteFileCloseFailureClass,
    custody: ManagedSqliteFileCloseCustody,
}

#[cfg(all(test, windows))]
pub(crate) struct ManagedSqliteFileCloseTestNativeResult {
    pub(crate) result: Result<ManagedSqliteFileCloseReceipt, ManagedSqliteFileCloseFailure>,
    pub(crate) observation: Option<super::ManagedSqliteShmTestUnmapNativeObservation>,
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

    #[cfg(test)]
    pub(crate) fn test_value() -> Self {
        Self {
            file: ManagedSqliteFileCloseReceipt::test_value(ManagedSqliteFileKind::Main),
        }
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
    #[cfg(test)]
    test_fault: Option<ManagedSqliteMainCloseTestFault>,
    #[cfg(test)]
    _completed_unlock_main: Option<ManuallyDrop<PinnedManagedSqliteMainFile>>,
    #[cfg(test)]
    completed_file: Option<ManagedSqliteMainFileCloseReceipt>,
    #[cfg(test)]
    close_test_faults: Option<std::sync::Arc<dyn ManagedSqliteMainCloseTestFaults>>,
}

impl PinnedManagedSqliteFile {
    pub(crate) fn close(
        self,
    ) -> Result<ManagedSqliteFileCloseReceipt, ManagedSqliteFileCloseFailure> {
        self.close_with(platform::close_sqlite_file)
    }

    #[cfg(all(test, windows))]
    pub(in crate::node_agent_managed_fs::sqlite_namespace) fn close_for_unmap_test_native(
        self,
        native: platform::PlatformManagedSqliteCloseTestNative,
    ) -> ManagedSqliteFileCloseTestNativeResult {
        let observation = std::cell::Cell::new(None);
        let result = self.close_with(|file| {
            let platform_result = platform::close_sqlite_file_for_test_native(file, native);
            observation.set(platform_result.observation);
            platform_result.result
        });
        ManagedSqliteFileCloseTestNativeResult {
            result,
            observation: observation.get(),
        }
    }

    fn close_with<F>(
        self,
        close: F,
    ) -> Result<ManagedSqliteFileCloseReceipt, ManagedSqliteFileCloseFailure>
    where
        F: FnOnce(std::fs::File) -> Result<(), platform::PlatformManagedSqliteCloseFailure>,
    {
        let Self {
            file,
            namespace,
            kind,
            access,
            identity,
            identity_digest,
            created,
        } = self;
        match close(file) {
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
