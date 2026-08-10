//! Concrete exact-route wrapper around the managed registry file bridge.
//!
//! The first main-file SHM map consumes a plan bound to this registration, route ordinal and role,
//! promotes the private file into WAL-main custody, installs into that custody and only then maps.

use std::num::{NonZeroU32, NonZeroU8};

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::{
    sqlite_vfs_abi::HandleBoundSqliteFileOperations,
    sqlite_vfs_policy::{
        HandleBoundSqliteAbiAttempt, HandleBoundSqliteAbiLockLevel,
        HandleBoundSqliteAbiShmLockAction, HandleBoundSqliteAbiShmMap,
        HandleBoundSqliteAbiUnlockLevel, ManagedSqliteLogicalFileRole,
    },
};

type ManagedTestRouteInner = ManagedSqliteTestVfsFile<TestCustody, ManagedTestNonceSource>;

pub(super) struct ManagedTestRouteFile {
    inner: ManagedTestRouteInner,
    shm_faults: ManagedTestShmFaultPlanBinding,
}

impl ManagedTestRouteFile {
    pub(super) fn new(
        inner: ManagedTestRouteInner,
        shm_faults: ManagedTestShmFaultPlanBinding,
        role: ManagedSqliteLogicalFileRole,
    ) -> Result<Self, ()> {
        if shm_faults.role() != role {
            let _ = inner
                .retain_test_fault_bridge_failure("managed SHM fault binding file role mismatch");
            return Err(());
        }
        Ok(Self { inner, shm_faults })
    }

    fn prepare_first_main_shm_map(&mut self) -> Result<(), ()> {
        if self.shm_faults.role() != ManagedSqliteLogicalFileRole::Main {
            return Ok(());
        }
        let plan = match self.shm_faults.claim() {
            Ok(plan) => plan,
            Err(code) => {
                let _ = self.inner.retain_test_fault_bridge_failure(code);
                return Err(());
            }
        };
        match plan {
            Some(plan) => {
                let probe = match self.inner.prepare_wal_main_shm_test_fault_script(
                    plan.before_call(),
                    plan.after_success(),
                ) {
                    Ok(probe) => probe,
                    Err(()) => {
                        let _ = self.inner.retain_test_fault_bridge_failure(
                            "managed SHM fault plan installation failed",
                        );
                        return Err(());
                    }
                };
                if let Err(code) = self.shm_faults.record_installed(probe) {
                    let _ = self.inner.retain_test_fault_bridge_failure(code);
                    return Err(());
                }
            }
            None => self.inner.promote_main_to_wal_for_shm()?,
        }
        Ok(())
    }
}

impl HandleBoundSqliteFileOperations for ManagedTestRouteFile {
    fn read_at_zero_filled(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, ()> {
        self.inner.read_at_zero_filled(offset, buffer)
    }

    fn write_all_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), ()> {
        self.inner.write_all_at(offset, bytes)
    }

    fn truncate(&mut self, size: u64) -> Result<(), ()> {
        self.inner.truncate(size)
    }

    fn size(&mut self) -> Result<u64, ()> {
        self.inner.size()
    }

    fn full_sync(&mut self) -> Result<(), ()> {
        self.inner.full_sync()
    }

    fn lock_to(
        &mut self,
        level: HandleBoundSqliteAbiLockLevel,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        self.inner.lock_to(level)
    }

    fn unlock_to(&mut self, level: HandleBoundSqliteAbiUnlockLevel) -> Result<(), ()> {
        self.inner.unlock_to(level)
    }

    fn check_reserved_lock(&mut self) -> Result<bool, ()> {
        self.inner.check_reserved_lock()
    }

    fn shm_map(
        &mut self,
        region: u32,
        region_size: NonZeroU32,
        extend: bool,
    ) -> Result<HandleBoundSqliteAbiShmMap, ()> {
        self.prepare_first_main_shm_map()?;
        self.inner.shm_map(region, region_size, extend)
    }

    fn shm_lock(
        &mut self,
        first: u8,
        count: NonZeroU8,
        action: HandleBoundSqliteAbiShmLockAction,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        self.inner.shm_lock(first, count, action)
    }

    fn shm_barrier(&mut self) -> Result<(), ()> {
        self.inner.shm_barrier()
    }

    fn shm_unmap(&mut self, delete: bool) -> Result<(), ()> {
        self.inner.shm_unmap(delete)
    }

    fn close(self: Box<Self>) -> Result<(), ()> {
        let Self {
            inner,
            shm_faults: _,
        } = *self;
        Box::new(inner).close()
    }
}
