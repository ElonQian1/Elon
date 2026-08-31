//! Sealed WAL shared-memory custody for one pinned SQLite namespace.
//!
//! This module is intended to be mounted below `sqlite_namespace`.  Raw SHM files, mapping
//! handles and OS lock offsets never cross this boundary.

use super::{
    ManagedSqliteAccess, ManagedSqliteDeleteFailure, ManagedSqliteDeleteOutcome,
    ManagedSqliteFileKind, ManagedSqliteFileOpenFailure, ManagedSqliteOpenMode,
    PinnedManagedSqliteFile, PinnedManagedSqliteNamespace,
};

#[path = "sqlite_namespace_shm/barrier.rs"]
mod barrier;
#[path = "sqlite_namespace_shm/close.rs"]
mod close;
#[path = "sqlite_namespace_shm/coordinator.rs"]
mod coordinator;
#[path = "sqlite_namespace_shm/failure_custody.rs"]
mod failure_custody;
#[path = "sqlite_namespace_shm/locking.rs"]
mod locking;
#[path = "sqlite_namespace_shm/mapping.rs"]
mod mapping;
#[path = "sqlite_namespace_shm/node_initialization.rs"]
mod node_initialization;
#[path = "sqlite_namespace_shm/teardown.rs"]
mod teardown;
#[cfg(test)]
#[path = "sqlite_namespace_shm/test_faults.rs"]
mod test_faults;
#[cfg(all(test, windows))]
#[path = "sqlite_namespace_shm/test_lock_runtime.rs"]
mod test_lock_runtime;
#[cfg(all(test, windows))]
#[path = "sqlite_namespace_shm/test_map_runtime.rs"]
mod test_map_runtime;
#[cfg(all(test, windows))]
#[path = "sqlite_namespace_shm/test_snapshot.rs"]
mod test_snapshot;
#[cfg(test)]
#[path = "sqlite_namespace_shm/test_support.rs"]
mod test_support;
#[cfg(all(test, windows))]
#[path = "sqlite_namespace_shm/test_unmap_runtime.rs"]
mod test_unmap_runtime;
#[path = "sqlite_namespace_shm/types.rs"]
mod types;
#[path = "sqlite_namespace_shm/unmap.rs"]
mod unmap;

#[cfg(windows)]
#[path = "windows_sqlite_shm.rs"]
mod platform_shm;

#[cfg(not(windows))]
mod platform_shm {
    use std::{fs::File, io, ptr::NonNull};

    pub(super) struct OwnedSqliteShmMapping;
    pub(super) struct OwnedSqliteShmView;

    pub(super) fn allocation_granularity() -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "NODE_MANAGED_SQLITE_SHM_PLATFORM_UNSUPPORTED",
        ))
    }

    pub(super) fn create_mapping(
        _file: &File,
        _maximum_size: u64,
    ) -> io::Result<OwnedSqliteShmMapping> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "NODE_MANAGED_SQLITE_SHM_PLATFORM_UNSUPPORTED",
        ))
    }

    pub(super) fn map_view(
        _mapping: &OwnedSqliteShmMapping,
        _aligned_offset: u64,
        _mapped_length: usize,
    ) -> io::Result<OwnedSqliteShmView> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "NODE_MANAGED_SQLITE_SHM_PLATFORM_UNSUPPORTED",
        ))
    }

    impl OwnedSqliteShmView {
        pub(super) fn base(&self) -> Option<NonNull<u8>> {
            None
        }

        pub(super) fn unmap_explicit(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl OwnedSqliteShmMapping {
        pub(super) fn close_explicit(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}

impl PinnedManagedSqliteNamespace {
    fn open_shm_for_wal(&self) -> Result<PinnedManagedSqliteFile, ManagedSqliteFileOpenFailure> {
        self.open_exact(
            ManagedSqliteFileKind::Shm,
            ManagedSqliteAccess::ReadWrite,
            ManagedSqliteOpenMode::OpenOrCreate,
        )
    }

    fn delete_shm_for_wal(&self) -> Result<ManagedSqliteDeleteOutcome, ManagedSqliteDeleteFailure> {
        self.delete_exact(ManagedSqliteFileKind::Shm, false)
    }

    #[cfg(all(test, windows))]
    fn delete_shm_for_wal_with_test_native(
        &self,
        operation: test_unmap_runtime::ManagedSqliteShmTestUnmapNativeOperation,
        before_native: impl FnOnce() -> std::io::Result<()>,
        after_native: impl FnOnce(
            test_unmap_runtime::ManagedSqliteShmTestUnmapNativeObservation,
        ) -> std::io::Result<()>,
    ) -> Result<ManagedSqliteDeleteOutcome, ManagedSqliteDeleteFailure> {
        let native = match operation {
            test_unmap_runtime::ManagedSqliteShmTestUnmapNativeOperation::ExactSiblingDeleteRetryable => {
                super::platform::PlatformManagedSqliteDeleteTestNative::Retryable
            }
            test_unmap_runtime::ManagedSqliteShmTestUnmapNativeOperation::ExactSiblingDeleteOutcomeUncertain => {
                super::platform::PlatformManagedSqliteDeleteTestNative::OutcomeUncertain
            }
            _ => unreachable!("validated exact-delete native operation"),
        };
        self.delete_exact_with(ManagedSqliteFileKind::Shm, false, |file| {
            before_native()?;
            let native_result = super::platform::delete_by_handle_for_test_native(file, native);
            if let Some(observation) = native_result.observation {
                after_native(observation)?;
            }
            native_result.result
        })
    }
}

pub(crate) use close::{
    ManagedSqliteWalMainBindFailure, ManagedSqliteWalMainCloseFailure,
    ManagedSqliteWalMainCloseFailurePhase, ManagedSqliteWalMainCloseReceipt,
};
#[cfg(all(test, windows))]
pub(crate) use close::{
    ManagedSqliteWalMainCloseFailureTestBoundary, ManagedSqliteWalMainCloseFailureTestSnapshot,
};
pub(crate) use coordinator::{
    PinnedManagedSqliteShmConnection, PinnedManagedSqliteWalMainFile, PinnedManagedSqliteWalRuntime,
};
#[cfg(test)]
pub(crate) use test_faults::ManagedSqliteShmTestFaultProbe;
#[cfg(all(test, windows))]
pub(crate) use test_faults::{
    ManagedSqliteShmTestTargetIdentity, ManagedSqliteShmTestTargetObserver,
    ManagedSqliteShmTriggeredTestFaultObservation,
};
#[cfg(all(test, windows))]
pub(crate) use test_lock_runtime::{
    ManagedSqliteShmTestLockExpectation, ManagedSqliteShmTestLockPath,
    ManagedSqliteShmTestLockReceipt,
};
#[cfg(all(test, windows))]
pub(crate) use test_map_runtime::{
    ManagedSqliteShmTestMapDmsPath, ManagedSqliteShmTestMapExpectation,
    ManagedSqliteShmTestMapPath, ManagedSqliteShmTestMapPointerIdentity,
    ManagedSqliteShmTestMapReceipt,
};
#[cfg(all(test, windows))]
pub(crate) use test_snapshot::{
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestTargetSnapshot,
    ManagedSqliteShmTestTopologySnapshot,
};
#[cfg(all(test, windows))]
pub(crate) use test_unmap_runtime::{
    ManagedSqliteShmTestConnectionDetachReceipt, ManagedSqliteShmTestUnmapActionEvent,
    ManagedSqliteShmTestUnmapActionOutcome, ManagedSqliteShmTestUnmapDeleteAuthorityReceipt,
    ManagedSqliteShmTestUnmapDeletePrestate, ManagedSqliteShmTestUnmapDeletePrestateReceipt,
    ManagedSqliteShmTestUnmapNativeObservation, ManagedSqliteShmTestUnmapNativeOperation,
    ManagedSqliteShmTestUnmapNativeReceipt, ManagedSqliteShmTestUnmapNativeTiming,
    ManagedSqliteShmTestUnmapReceipt,
};
pub(crate) use types::{
    ManagedSqliteShmBudget, ManagedSqliteShmFailure, ManagedSqliteShmFailureClass,
    ManagedSqliteShmFailurePhase, ManagedSqliteShmLockAction, ManagedSqliteShmLockAttempt,
    ManagedSqliteShmLockRequest, ManagedSqliteShmMapMode, ManagedSqliteShmMapOutcome,
    ManagedSqliteShmRegionPointer, ManagedSqliteShmUnmapMode, ManagedSqliteWalMainUnmapFailure,
};
