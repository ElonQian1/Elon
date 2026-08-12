//! Transparent file-operations wrapper selected by one fixture-owned fault controller.

use std::{
    num::{NonZeroU32, NonZeroU8},
    sync::Arc,
};

use crate::node_agent_compute_plugin_host::local_authority::{
    sqlite_vfs_abi::HandleBoundSqliteFileOperations,
    sqlite_vfs_policy::{
        HandleBoundSqliteAbiAttempt, HandleBoundSqliteAbiLockLevel,
        HandleBoundSqliteAbiShmLockAction, HandleBoundSqliteAbiShmMap,
        HandleBoundSqliteAbiUnlockLevel, ManagedSqliteLogicalFileRole,
    },
};

use super::{
    ManagedTestCallbackFaultController, ManagedTestCallbackFaultOperation, ManagedTestRouteOrdinal,
};

pub(in super::super) struct ManagedTestFaultingFile<Inner> {
    inner: Option<Inner>,
    controller: Arc<ManagedTestCallbackFaultController>,
    route: ManagedTestRouteOrdinal,
    role: ManagedSqliteLogicalFileRole,
}

impl<Inner> ManagedTestFaultingFile<Inner> {
    pub(in super::super) fn new(
        inner: Inner,
        controller: Arc<ManagedTestCallbackFaultController>,
        route: ManagedTestRouteOrdinal,
        role: ManagedSqliteLogicalFileRole,
    ) -> Self {
        Self {
            inner: Some(inner),
            controller,
            route,
            role,
        }
    }

    fn begin(&self, operation: ManagedTestCallbackFaultOperation) -> Result<bool, ()> {
        self.controller
            .begin_operation(self.route, self.role, operation)
    }

    fn inner(&mut self) -> Result<&mut Inner, ()> {
        self.inner.as_mut().ok_or(())
    }
}

impl<Inner> HandleBoundSqliteFileOperations for ManagedTestFaultingFile<Inner>
where
    Inner: HandleBoundSqliteFileOperations,
{
    fn read_at_zero_filled(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, ()> {
        self.inner()?.read_at_zero_filled(offset, buffer)
    }

    fn write_all_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), ()> {
        self.inner()?.write_all_at(offset, bytes)
    }

    fn truncate(&mut self, size: u64) -> Result<(), ()> {
        self.inner()?.truncate(size)
    }

    fn size(&mut self) -> Result<u64, ()> {
        self.inner()?.size()
    }

    fn full_sync(&mut self) -> Result<(), ()> {
        self.inner()?.full_sync()
    }

    fn lock_to(
        &mut self,
        level: HandleBoundSqliteAbiLockLevel,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        self.inner()?.lock_to(level)
    }

    fn unlock_to(&mut self, level: HandleBoundSqliteAbiUnlockLevel) -> Result<(), ()> {
        self.inner()?.unlock_to(level)
    }

    fn check_reserved_lock(&mut self) -> Result<bool, ()> {
        self.inner()?.check_reserved_lock()
    }

    fn shm_map(
        &mut self,
        region: u32,
        region_size: NonZeroU32,
        extend: bool,
    ) -> Result<HandleBoundSqliteAbiShmMap, ()> {
        if self.begin(ManagedTestCallbackFaultOperation::ShmMap)? {
            return Err(());
        }
        self.inner()?.shm_map(region, region_size, extend)
    }

    fn shm_lock(
        &mut self,
        first: u8,
        count: NonZeroU8,
        action: HandleBoundSqliteAbiShmLockAction,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        if self.begin(ManagedTestCallbackFaultOperation::ShmLock)? {
            return Err(());
        }
        self.inner()?.shm_lock(first, count, action)
    }

    fn shm_barrier(&mut self) -> Result<(), ()> {
        if self.begin(ManagedTestCallbackFaultOperation::ShmBarrier)? {
            return Err(());
        }
        self.inner()?.shm_barrier()
    }

    fn shm_unmap(&mut self, delete: bool) -> Result<(), ()> {
        if self.begin(ManagedTestCallbackFaultOperation::ShmUnmap)? {
            return Err(());
        }
        self.inner()?.shm_unmap(delete)
    }

    fn close(mut self: Box<Self>) -> Result<(), ()> {
        if self.begin(ManagedTestCallbackFaultOperation::FileClose)? {
            // Dropping the managed inner file retains terminal registry custody. Its Drop path
            // does not call the physical close primitive, so this is not a hidden retry.
            return Err(());
        }
        let inner = self.inner.take().ok_or(())?;
        Box::new(inner).close()
    }
}
