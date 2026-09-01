//! Exact installed-ABI bridge for controlled q12-q18 initialization outcomes.
//!
//! This bridge arms the managed initialization controller only after a real WAL-main target is
//! attached, invokes the installed `xShmLock`, then seals both initialization and requested-range
//! ledgers. It contains no success fallback and cannot manufacture an `actual` receipt.

use std::os::raw::c_int;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestCreatedFirstSharedBusyCloseSucceededReceiptV1,
    ManagedSqliteShmTestExistingFirstSharedBusyCloseSucceededReceiptV1,
    ManagedSqliteShmTestInitializationExpectationV1, ManagedSqliteShmTestInitializationReceiptV1,
    ManagedSqliteShmTestLockReceipt, ManagedSqliteShmTestTargetObserver,
    ManagedSqliteShmTestTargetSnapshot,
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

pub(in super::super) struct PendingManagedTestLockCreatedFirstSharedBusyCloseSucceededObservationV1
{
    pub(in super::super) callback: ManagedTestShmLockCallbackObservation,
    pub(in super::super) before: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) after: ManagedSqliteShmTestTargetSnapshot,
    observer: ManagedSqliteShmTestTargetObserver,
    pending_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestLockCreatedFirstSharedBusyCloseSucceededObservationV1 {
    pub(in super::super) callback: ManagedTestShmLockCallbackObservation,
    pub(in super::super) before: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) after: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) initialization:
        ManagedSqliteShmTestCreatedFirstSharedBusyCloseSucceededReceiptV1,
    pub(in super::super) lock_no_requested_native: ManagedSqliteShmTestLockReceipt,
    pub(in super::super) pending_count: usize,
}

pub(in super::super) struct PendingManagedTestLockExistingFirstSharedBusyCloseSucceededObservationV1
{
    pub(in super::super) callback: ManagedTestShmLockCallbackObservation,
    pub(in super::super) before: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) after: ManagedSqliteShmTestTargetSnapshot,
    observer: ManagedSqliteShmTestTargetObserver,
    pending_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestLockExistingFirstSharedBusyCloseSucceededObservationV1 {
    pub(in super::super) callback: ManagedTestShmLockCallbackObservation,
    pub(in super::super) before: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) after: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) initialization:
        ManagedSqliteShmTestExistingFirstSharedBusyCloseSucceededReceiptV1,
    pub(in super::super) lock_no_requested_native: ManagedSqliteShmTestLockReceipt,
    pub(in super::super) pending_count: usize,
}

impl PendingManagedTestLockCreatedFirstSharedBusyCloseSucceededObservationV1 {
    pub(in super::super) fn abort_after_inspection_failure(self) -> Result<(), &'static str> {
        self.observer
            .abort_created_first_shared_busy_close_succeeded_observation_v1()
    }

    /// The caller must inspect registry terminal custody before consuming this method. Q18's DMS
    /// holder remains locked until this exact finish call explicitly releases it.
    pub(in super::super) fn finish_after_terminal_custody_observed(
        self,
    ) -> Result<ManagedTestLockCreatedFirstSharedBusyCloseSucceededObservationV1, &'static str>
    {
        let initialization = match self
            .observer
            .finish_created_first_shared_busy_close_succeeded_observation_v1()
        {
            Ok(receipt) => receipt,
            Err(error) => {
                let _ = self
                    .observer
                    .abort_created_first_shared_busy_close_succeeded_observation_v1();
                return Err(error);
            }
        };
        let lock_no_requested_native = initialization.requested_lock_receipt();
        Ok(
            ManagedTestLockCreatedFirstSharedBusyCloseSucceededObservationV1 {
                callback: self.callback,
                before: self.before,
                after: self.after,
                initialization,
                lock_no_requested_native,
                pending_count: self.pending_count,
            },
        )
    }
}

impl PendingManagedTestLockExistingFirstSharedBusyCloseSucceededObservationV1 {
    pub(in super::super) fn abort_after_inspection_failure(self) -> Result<(), &'static str> {
        self.observer
            .abort_existing_first_shared_busy_close_succeeded_observation_v1()
    }

    /// The caller must inspect terminal custody before consuming this method. Q19's independent
    /// DMS holder remains locked through the target close and until this explicit finish call.
    pub(in super::super) fn finish_after_terminal_custody_observed(
        self,
    ) -> Result<ManagedTestLockExistingFirstSharedBusyCloseSucceededObservationV1, &'static str>
    {
        let initialization = match self
            .observer
            .finish_existing_first_shared_busy_close_succeeded_observation_v1()
        {
            Ok(receipt) => receipt,
            Err(error) => {
                let _ = self
                    .observer
                    .abort_existing_first_shared_busy_close_succeeded_observation_v1();
                return Err(error);
            }
        };
        let lock_no_requested_native = initialization.requested_lock_receipt();
        Ok(
            ManagedTestLockExistingFirstSharedBusyCloseSucceededObservationV1 {
                callback: self.callback,
                before: self.before,
                after: self.after,
                initialization,
                lock_no_requested_native,
                pending_count: self.pending_count,
            },
        )
    }
}

impl ManagedSqliteRoutedConnectionFixture {
    pub(in super::super) fn observe_main_shm_lock_existing_first_shared_busy_close_succeeded_v1(
        &self,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        raw_flags: c_int,
    ) -> Result<
        PendingManagedTestLockExistingFirstSharedBusyCloseSucceededObservationV1,
        &'static str,
    > {
        let witness = self
            .route_entry
            .as_ref()
            .ok_or("managed Q19 initialization fixture route entry is not live")?
            .installed_shm_fault_witness()?;
        let observer = witness.observer()?;
        let before = observer
            .snapshot()
            .map_err(|_| "managed Q19 initialization pre-snapshot failed")?;
        observer.arm_existing_first_shared_busy_close_succeeded_observation_v1(expectation)?;
        let after_arm = (|| {
            let callback = self.observe_main_shm_lock_raw(
                i32::from(expectation.first),
                i32::from(expectation.count),
                raw_flags,
            )?;
            let after = observer
                .snapshot()
                .map_err(|_| "managed Q19 initialization post-snapshot failed")?;
            let pending_count = witness.pending_count()?;
            Ok((callback, after, pending_count))
        })();
        let (callback, after, pending_count) = match after_arm {
            Ok(values) => values,
            Err(error) => {
                observer.abort_existing_first_shared_busy_close_succeeded_observation_v1()?;
                return Err(error);
            }
        };
        Ok(
            PendingManagedTestLockExistingFirstSharedBusyCloseSucceededObservationV1 {
                callback,
                before,
                after,
                observer,
                pending_count,
            },
        )
    }

    pub(in super::super) fn observe_main_shm_lock_created_first_shared_busy_close_succeeded_v1(
        &self,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        raw_flags: c_int,
    ) -> Result<PendingManagedTestLockCreatedFirstSharedBusyCloseSucceededObservationV1, &'static str>
    {
        let witness = self
            .route_entry
            .as_ref()
            .ok_or("managed Q18 initialization fixture route entry is not live")?
            .installed_shm_fault_witness()?;
        let observer = witness.observer()?;
        let before = observer
            .snapshot()
            .map_err(|_| "managed Q18 initialization pre-snapshot failed")?;
        observer.begin_lock_initialization_failure_observation_v1(expectation)?;
        let after_arm = (|| {
            let callback = self.observe_main_shm_lock_raw(
                i32::from(expectation.first),
                i32::from(expectation.count),
                raw_flags,
            )?;
            let after = observer
                .snapshot()
                .map_err(|_| "managed Q18 initialization post-snapshot failed")?;
            let pending_count = witness.pending_count()?;
            Ok((callback, after, pending_count))
        })();
        let (callback, after, pending_count) = match after_arm {
            Ok(values) => values,
            Err(error) => {
                observer.abort_created_first_shared_busy_close_succeeded_observation_v1()?;
                return Err(error);
            }
        };
        Ok(
            PendingManagedTestLockCreatedFirstSharedBusyCloseSucceededObservationV1 {
                callback,
                before,
                after,
                observer,
                pending_count,
            },
        )
    }

    pub(in super::super) fn observe_main_shm_lock_created_first_exclusive_release_failure_v1(
        &self,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        raw_flags: c_int,
    ) -> Result<ManagedTestLockInitializationFailureObservationV1, &'static str> {
        self.observe_main_shm_lock_initialization_exclusive_release_failure_v1(
            expectation,
            raw_flags,
        )
    }

    pub(in super::super) fn observe_main_shm_lock_existing_first_exclusive_release_failure_v1(
        &self,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        raw_flags: c_int,
    ) -> Result<ManagedTestLockInitializationFailureObservationV1, &'static str> {
        self.observe_main_shm_lock_initialization_exclusive_release_failure_v1(
            expectation,
            raw_flags,
        )
    }

    pub(in super::super) fn observe_main_shm_lock_created_first_truncate_error_release_succeeded_v1(
        &self,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        raw_flags: c_int,
    ) -> Result<ManagedTestLockInitializationFailureObservationV1, &'static str> {
        self.observe_main_shm_lock_initialization_exclusive_release_failure_v1(
            expectation,
            raw_flags,
        )
    }

    pub(in super::super) fn observe_main_shm_lock_existing_first_truncate_error_release_succeeded_v1(
        &self,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        raw_flags: c_int,
    ) -> Result<ManagedTestLockInitializationFailureObservationV1, &'static str> {
        self.observe_main_shm_lock_initialization_exclusive_release_failure_v1(
            expectation,
            raw_flags,
        )
    }

    pub(in super::super) fn observe_main_shm_lock_created_first_truncate_error_release_failed_v1(
        &self,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        raw_flags: c_int,
    ) -> Result<ManagedTestLockInitializationFailureObservationV1, &'static str> {
        self.observe_main_shm_lock_initialization_exclusive_release_failure_v1(
            expectation,
            raw_flags,
        )
    }

    pub(in super::super) fn observe_main_shm_lock_existing_first_truncate_error_release_failed_v1(
        &self,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        raw_flags: c_int,
    ) -> Result<ManagedTestLockInitializationFailureObservationV1, &'static str> {
        self.observe_main_shm_lock_initialization_exclusive_release_failure_v1(
            expectation,
            raw_flags,
        )
    }

    fn observe_main_shm_lock_initialization_exclusive_release_failure_v1(
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
