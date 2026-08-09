//! Sealed WAL shared-memory custody for one pinned SQLite namespace.
//!
//! This module is intended to be mounted below `sqlite_namespace`.  Raw SHM files, mapping
//! handles and OS lock offsets never cross this boundary.

use super::{
    ManagedSqliteAccess, ManagedSqliteDeleteFailure, ManagedSqliteDeleteOutcome,
    ManagedSqliteFileKind, ManagedSqliteFileOpenFailure, ManagedSqliteOpenMode,
    PinnedManagedSqliteFile, PinnedManagedSqliteNamespace,
};

#[path = "sqlite_namespace_shm/coordinator.rs"]
mod coordinator;
#[path = "sqlite_namespace_shm/locking.rs"]
mod locking;
#[path = "sqlite_namespace_shm/mapping.rs"]
mod mapping;
#[path = "sqlite_namespace_shm/teardown.rs"]
mod teardown;
#[path = "sqlite_namespace_shm/types.rs"]
mod types;

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
}

pub(crate) use coordinator::{
    PinnedManagedSqliteShmConnection, PinnedManagedSqliteWalMainFile, PinnedManagedSqliteWalRuntime,
};
pub(crate) use types::{
    ManagedSqliteShmBudget, ManagedSqliteShmFailure, ManagedSqliteShmFailureClass,
    ManagedSqliteShmFailurePhase, ManagedSqliteShmLockAction, ManagedSqliteShmLockAttempt,
    ManagedSqliteShmLockRequest, ManagedSqliteShmMapMode, ManagedSqliteShmMapOutcome,
    ManagedSqliteShmRegionPointer, ManagedSqliteShmUnmapMode, ManagedSqliteWalMainUnmapFailure,
};
