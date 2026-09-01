//! Exact production-xShmLock bridge for q11 controlled raw-state evidence.

use std::num::NonZeroU8;

use rusqlite::ffi;

use super::ManagedSqliteRoutedConnectionFixture;
use crate::{
    node_agent_compute_plugin_host::local_authority::{
        sqlite_vfs_abi::{
            arm_test_x_shm_lock_raw_state_rejection_v1,
            HandleBoundSqliteAbiRawLockRejectionCaseV1,
            HandleBoundSqliteAbiRawLockRejectionReceiptV1,
        },
        sqlite_vfs_policy::abi::connection_fixture::managed_vfs::lifecycle_faults::{
            ManagedTestPreManagedLockPath,
        },
    },
    node_agent_managed_fs::{ManagedSqliteShmLockAction, ManagedSqliteShmLockRequest},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestLockRawStateRejectionObservationV1 {
    abi: HandleBoundSqliteAbiRawLockRejectionReceiptV1,
    route_no_entry: [u64; 18],
}

impl ManagedTestLockRawStateRejectionObservationV1 {
    pub(in super::super) const fn abi(self) -> HandleBoundSqliteAbiRawLockRejectionReceiptV1 {
        self.abi
    }

    pub(in super::super) const fn route_no_entry(self) -> [u64; 18] {
        self.route_no_entry
    }
}

impl ManagedSqliteRoutedConnectionFixture {
    /// Arms the registry no-entry sentinel, installs one closed memory-safe raw representation,
    /// invokes the saved production xShmLock callback with canonical valid scalars, then consumes
    /// both one-shot ledgers. Corrupt/retained fixture state must remain child-process isolated.
    pub(in super::super) fn observe_main_shm_lock_raw_state_rejection_v1(
        &self,
        case_v1: HandleBoundSqliteAbiRawLockRejectionCaseV1,
    ) -> Result<ManagedTestLockRawStateRejectionObservationV1, &'static str> {
        let request = ManagedSqliteShmLockRequest::new(
            0,
            NonZeroU8::new(1).ok_or("q11 raw Lock canonical count was zero")?,
            ManagedSqliteShmLockAction::LockShared,
        )
        .map_err(|_| "q11 raw Lock canonical managed request was rejected")?;
        self.arm_pre_managed_lock_observation(ManagedTestPreManagedLockPath::RawRejected, request)?;

        let file = self.main_file_pointer()?;
        // SAFETY: main_file_pointer returns this live FULL_MUTEX fixture's exact serialized main
        // allocation. The closed q11 case enum cannot construct invalid non-null pointers.
        let guard = unsafe { arm_test_x_shm_lock_raw_state_rejection_v1(file, case_v1) }?;
        let flags = ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_SHARED;
        // SAFETY: the fixture remains alive and deliberately performs no overlapping SQLite work.
        let abi = unsafe { guard.invoke(0, 1, flags) }?;
        let route_no_entry = self
            .finish_raw_rejected_lock_observation()?
            .ordered_values();
        Ok(ManagedTestLockRawStateRejectionObservationV1 {
            abi,
            route_no_entry,
        })
    }
}
