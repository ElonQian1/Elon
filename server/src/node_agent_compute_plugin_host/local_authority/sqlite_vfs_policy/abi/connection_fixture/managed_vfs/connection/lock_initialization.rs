//! Exact installed-ABI bridge for the q12 initialization release failure.
//!
//! This bridge arms the managed initialization controller only after a real WAL-main target is
//! attached, invokes the installed `xShmLock`, then seals both initialization and requested-range
//! ledgers. It contains no success fallback and cannot manufacture an `actual` receipt.

use std::os::raw::c_int;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestInitializationExpectationV1, ManagedSqliteShmTestInitializationReceiptV1,
    ManagedSqliteShmTestLockReceipt, ManagedSqliteShmTestTargetSnapshot,
};

use super::{ManagedSqliteRoutedConnectionFixture, ManagedTestShmLockCallbackObservation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestLockInitializationFailureObservationV1 {
    pub(in super::super) callback: ManagedTestShmLockCallbackObservation,
    pub(in super::super) before: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) after: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) initialization: ManagedSqliteShmTestInitializationReceiptV1,
    pub(in super::super) lock_no_requested_native: ManagedSqliteShmTestLockReceipt,
    pub(in super::super) pending_count: usize,
}

impl ManagedSqliteRoutedConnectionFixture {
    pub(in super::super) fn observe_main_shm_lock_created_first_exclusive_release_failure_v1(
        &self,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        raw_flags: c_int,
    ) -> Result<ManagedTestLockInitializationFailureObservationV1, &'static str> {
        let witness = self
            .route_entry
            .as_ref()
            .ok_or("managed initialization fixture route entry is not live")?
            .installed_shm_fault_witness()?;
        let observer = witness.observer()?;
        let before = observer
            .snapshot()
            .map_err(|_| "managed initialization pre-snapshot failed")?;
        observer.begin_lock_initialization_failure_observation_v1(expectation)?;
        let callback = self.observe_main_shm_lock_raw(
            i32::from(expectation.first),
            i32::from(expectation.count),
            raw_flags,
        )?;
        let after = observer
            .snapshot()
            .map_err(|_| "managed initialization post-snapshot failed")?;
        let initialization = observer.finish_lock_initialization_failure_observation_v1()?;
        let lock_no_requested_native = initialization.requested_lock_receipt();
        let pending_count = witness.pending_count()?;
        Ok(ManagedTestLockInitializationFailureObservationV1 {
            callback,
            before,
            after,
            initialization,
            lock_no_requested_native,
            pending_count,
        })
    }
}
