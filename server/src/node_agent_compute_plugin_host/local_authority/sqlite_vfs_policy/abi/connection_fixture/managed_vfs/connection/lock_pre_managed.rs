//! Narrow fixture projection for q9 pre-managed Lock evidence.

use super::ManagedSqliteRoutedConnectionFixture;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
    abi::connection_fixture::managed_vfs::lifecycle_faults::{
        ManagedTestPreManagedLockPath, ManagedTestPreManagedLockSnapshot,
    },
    registry::ManagedSqliteRegistryCallbackCounterPrimeReceipt,
};
use crate::node_agent_managed_fs::ManagedSqliteShmLockRequest;

impl ManagedSqliteRoutedConnectionFixture {
    pub(in super::super) fn arm_pre_managed_lock_observation(
        &self,
        path: ManagedTestPreManagedLockPath,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), &'static str> {
        self.registration
            .as_ref()
            .ok_or("pre-managed Lock fixture registration missing")?
            .lifecycle()
            .arm_pre_managed_lock_observation(self.route_ordinal(), path, request)
    }

    pub(in super::super) fn pre_managed_lock_snapshot(
        &self,
    ) -> Result<ManagedTestPreManagedLockSnapshot, &'static str> {
        self.registration
            .as_ref()
            .ok_or("pre-managed Lock fixture registration missing")?
            .lifecycle()
            .pre_managed_lock_snapshot(self.route_ordinal())
    }

    pub(in super::super) fn finish_abi_rejected_lock_observation(
        &self,
    ) -> Result<ManagedTestPreManagedLockSnapshot, &'static str> {
        self.registration
            .as_ref()
            .ok_or("ABI-rejected Lock fixture registration missing")?
            .lifecycle()
            .finish_abi_rejected_lock_observation(self.route_ordinal())
    }

    pub(in super::super) fn finish_raw_rejected_lock_observation(
        &self,
    ) -> Result<ManagedTestPreManagedLockSnapshot, &'static str> {
        self.registration
            .as_ref()
            .ok_or("raw-rejected Lock fixture registration missing")?
            .lifecycle()
            .finish_raw_rejected_lock_observation(self.route_ordinal())
    }

    pub(in super::super) fn prime_lock_callback_counter_overflow(
        &self,
    ) -> Result<ManagedSqliteRegistryCallbackCounterPrimeReceipt, &'static str> {
        self.route
            .prime_lock_callback_counter_overflow_for_test()
            .map_err(|()| "pre-managed Lock callback counter prime rejected")
    }

    pub(in super::super) fn quarantine_for_lock_admission_test(&self) -> Result<(), &'static str> {
        self.route
            .retain_failure("pre-managed Lock admission RouteUnknown sentinel")
            .map_err(|()| "pre-managed Lock admission route quarantine failed")
    }

    pub(in super::super) fn lock_callback_counter_overflow_terminal(
        &self,
    ) -> Result<bool, &'static str> {
        self.route
            .lock_callback_counter_overflow_terminal_for_test()
            .map_err(|()| "pre-managed Lock counter-overflow terminal witness unavailable")
    }
}
