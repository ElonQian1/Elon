use std::{error::Error as StdError, fmt, num::NonZeroU64};

use super::{
    coordinator::PinnedManagedSqliteWalMainFile,
    types::{
        ManagedSqliteShmFailure, ManagedSqliteShmFailureClass, ManagedSqliteShmUnmapMode,
        ManagedSqliteWalMainUnmapFailure,
    },
};

#[must_use = "failed WAL-main bind retains the exact main-file custody"]
pub(crate) struct ManagedSqliteWalMainBindFailure {
    failure: ManagedSqliteShmFailure,
    main: super::super::PinnedManagedSqliteMainFile,
}

impl ManagedSqliteWalMainBindFailure {
    pub(super) fn new(
        failure: ManagedSqliteShmFailure,
        main: super::super::PinnedManagedSqliteMainFile,
    ) -> Self {
        Self { failure, main }
    }

    pub(crate) fn failure(&self) -> &ManagedSqliteShmFailure {
        &self.failure
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        ManagedSqliteShmFailure,
        super::super::PinnedManagedSqliteMainFile,
    ) {
        (self.failure, self.main)
    }
}
use super::super::{ManagedSqliteFileKind, ManagedSqliteMainFileCloseFailure};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteWalMainCloseFailurePhase {
    ShmUnmap,
    MainClose,
}

#[derive(Debug, PartialEq, Eq)]
#[must_use = "a WAL-main close receipt proves SHM and main custody were released"]
pub(crate) struct ManagedSqliteWalMainCloseReceipt {
    kind: ManagedSqliteFileKind,
}

impl ManagedSqliteWalMainCloseReceipt {
    pub(crate) fn kind(&self) -> ManagedSqliteFileKind {
        self.kind
    }

    #[cfg(test)]
    pub(crate) fn test_value() -> Self {
        Self {
            kind: ManagedSqliteFileKind::Main,
        }
    }
}

#[must_use = "failed WAL-main close retains live or terminal main and SHM custody"]
pub(crate) struct ManagedSqliteWalMainCloseFailure {
    phase: ManagedSqliteWalMainCloseFailurePhase,
    unmap_failure: Option<ManagedSqliteWalMainUnmapFailure>,
    main_failure: Option<ManagedSqliteMainFileCloseFailure>,
    runtime_generation: Option<NonZeroU64>,
}

impl PinnedManagedSqliteWalMainFile {
    pub(crate) fn close(
        self,
    ) -> Result<ManagedSqliteWalMainCloseReceipt, ManagedSqliteWalMainCloseFailure> {
        let wal_main = if self.shm.is_some() {
            match self.unmap_shm(ManagedSqliteShmUnmapMode::Keep) {
                Ok(wal_main) => wal_main,
                Err(unmap_failure) => {
                    return Err(ManagedSqliteWalMainCloseFailure {
                        phase: ManagedSqliteWalMainCloseFailurePhase::ShmUnmap,
                        unmap_failure: Some(unmap_failure),
                        main_failure: None,
                        runtime_generation: None,
                    });
                }
            }
        } else {
            self
        };
        let Self {
            shm: _,
            main,
            runtime_generation,
        } = wal_main;
        match main.close() {
            Ok(receipt) => Ok(ManagedSqliteWalMainCloseReceipt {
                kind: receipt.kind(),
            }),
            Err(main_failure) => Err(ManagedSqliteWalMainCloseFailure {
                phase: ManagedSqliteWalMainCloseFailurePhase::MainClose,
                unmap_failure: None,
                main_failure: Some(main_failure),
                runtime_generation: Some(runtime_generation),
            }),
        }
    }
}

impl ManagedSqliteWalMainCloseFailure {
    pub(crate) fn phase(&self) -> ManagedSqliteWalMainCloseFailurePhase {
        self.phase
    }

    pub(crate) fn close_outcome_uncertain(&self) -> bool {
        self.unmap_failure.as_ref().is_some_and(|failure| {
            failure.failure().class() == ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
                || failure.failure().lock_outcome_uncertain()
        }) || self
            .main_failure
            .as_ref()
            .is_some_and(ManagedSqliteMainFileCloseFailure::close_outcome_uncertain)
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        ManagedSqliteWalMainCloseFailurePhase,
        Option<ManagedSqliteWalMainUnmapFailure>,
        Option<ManagedSqliteMainFileCloseFailure>,
        Option<NonZeroU64>,
    ) {
        (
            self.phase,
            self.unmap_failure,
            self.main_failure,
            self.runtime_generation,
        )
    }
}

impl fmt::Debug for ManagedSqliteWalMainCloseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteWalMainCloseFailure")
            .field("phase", &self.phase)
            .field("unmap_failure", &self.unmap_failure)
            .field("main_failure", &self.main_failure)
            .field("custody", &"<retained>")
            .finish()
    }
}

impl fmt::Debug for ManagedSqliteWalMainBindFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteWalMainBindFailure")
            .field("failure", &self.failure)
            .field("main", &"<retained>")
            .finish()
    }
}

impl fmt::Display for ManagedSqliteWalMainBindFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NODE_MANAGED_SQLITE_WAL_MAIN_BIND_FAILED")
    }
}

impl StdError for ManagedSqliteWalMainBindFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.failure)
    }
}

impl fmt::Display for ManagedSqliteWalMainCloseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NODE_MANAGED_SQLITE_WAL_MAIN_CLOSE_FAILED")
    }
}

impl StdError for ManagedSqliteWalMainCloseFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.unmap_failure
            .as_ref()
            .map(|failure| failure as &(dyn StdError + 'static))
            .or_else(|| {
                self.main_failure
                    .as_ref()
                    .map(|failure| failure as &(dyn StdError + 'static))
            })
    }
}
