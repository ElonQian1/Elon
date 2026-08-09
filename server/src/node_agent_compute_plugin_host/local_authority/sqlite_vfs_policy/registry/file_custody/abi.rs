//! Narrow projection of exact file custody into SQLite callback semantics.
//!
//! The adapter keeps raw managed handles and registry errors private. Its only raw address is an
//! SHM region that SQLite may observe while this same value retains the mapped connection.

use std::{
    num::{NonZeroU32, NonZeroU8},
    os::raw::c_void,
    ptr::NonNull,
};

use crate::node_agent_managed_fs::{
    ManagedSqliteLockAttempt, ManagedSqliteRequestedLock, ManagedSqliteShmLockAction,
    ManagedSqliteShmLockAttempt, ManagedSqliteShmLockRequest, ManagedSqliteShmMapMode,
    ManagedSqliteShmMapOutcome, ManagedSqliteShmUnmapMode, ManagedSqliteUnlockTarget,
    PinnedManagedSqliteWalRuntime,
};

use super::{
    ManagedSqliteRegistryCustody, ManagedSqliteRegistryNonceSource, ManagedSqliteRegistryPinnedFile,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum HandleBoundSqliteAbiLockLevel {
    Shared,
    Reserved,
    Exclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum HandleBoundSqliteAbiUnlockLevel
{
    None,
    Shared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum HandleBoundSqliteAbiShmLockAction
{
    LockShared,
    LockExclusive,
    UnlockShared,
    UnlockExclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum HandleBoundSqliteAbiAttempt {
    Acquired,
    Busy,
}

#[derive(Debug)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum HandleBoundSqliteAbiShmMap {
    NotPresent,
    Mapped(NonNull<c_void>),
}

/// Concrete production state that keeps registry route, callback leases, managed handles and SHM
/// connection custody inseparable. Construction remains inside the registry until a real xOpen
/// producer exists.
pub(in crate::node_agent_compute_plugin_host::local_authority) struct HandleBoundSqliteAbiFile<
    Custody,
    NonceSource,
> where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    file: ManagedSqliteRegistryPinnedFile<Custody, NonceSource>,
}

pub(in crate::node_agent_compute_plugin_host::local_authority) type ComputePluginHandleBoundSqliteAbiFile =
    HandleBoundSqliteAbiFile<
        crate::node_agent_compute_plugin_host::local_authority::ComputePluginHandleBoundAuthorityOpenIntent,
        super::super::process_owner::ManagedSqliteRegistrySystemNonceSource,
    >;

impl<Custody, NonceSource> HandleBoundSqliteAbiFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn from_pinned(
        file: ManagedSqliteRegistryPinnedFile<Custody, NonceSource>,
    ) -> Self {
        Self { file }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn read_at_zero_filled(
        &mut self,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<usize, ()> {
        self.file.read_at_zero_filled(offset, buffer).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn write_all_at(
        &mut self,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), ()> {
        self.file.write_all_at(offset, bytes).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn truncate(
        &mut self,
        size: u64,
    ) -> Result<(), ()> {
        self.file.truncate(size).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn size(
        &mut self,
    ) -> Result<u64, ()> {
        self.file.size().map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn full_sync(
        &mut self,
    ) -> Result<(), ()> {
        self.file.full_sync().map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn lock_to(
        &mut self,
        level: HandleBoundSqliteAbiLockLevel,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        let requested = match level {
            HandleBoundSqliteAbiLockLevel::Shared => ManagedSqliteRequestedLock::Shared,
            HandleBoundSqliteAbiLockLevel::Reserved => ManagedSqliteRequestedLock::Reserved,
            HandleBoundSqliteAbiLockLevel::Exclusive => ManagedSqliteRequestedLock::Exclusive,
        };
        self.file
            .lock_to(requested)
            .map(map_lock_attempt)
            .map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn unlock_to(
        &mut self,
        level: HandleBoundSqliteAbiUnlockLevel,
    ) -> Result<(), ()> {
        let target = match level {
            HandleBoundSqliteAbiUnlockLevel::None => ManagedSqliteUnlockTarget::None,
            HandleBoundSqliteAbiUnlockLevel::Shared => ManagedSqliteUnlockTarget::Shared,
        };
        self.file.unlock_to(target).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn check_reserved_lock(
        &mut self,
    ) -> Result<bool, ()> {
        self.file.check_reserved_lock().map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn promote_main_to_wal(
        &mut self,
        runtime: &PinnedManagedSqliteWalRuntime,
    ) -> Result<(), ()> {
        self.file.promote_main_to_wal(runtime).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn shm_map(
        &mut self,
        region: u32,
        region_size: NonZeroU32,
        extend: bool,
    ) -> Result<HandleBoundSqliteAbiShmMap, ()> {
        let mode = if extend {
            ManagedSqliteShmMapMode::Extend
        } else {
            ManagedSqliteShmMapMode::Observe
        };
        match self.file.shm_map(region, region_size, mode).map_err(drop)? {
            ManagedSqliteShmMapOutcome::NotPresent => Ok(HandleBoundSqliteAbiShmMap::NotPresent),
            ManagedSqliteShmMapOutcome::Mapped(pointer) => {
                if pointer.region() != region || pointer.length() != region_size.get() as usize {
                    return Err(());
                }
                // SAFETY: the returned address remains owned by `self.file` until explicit SHM
                // unmap/close. This adapter never transfers ownership of the mapped region.
                let pointer = NonNull::new(unsafe { pointer.as_mut_ptr() }.cast()).ok_or(())?;
                Ok(HandleBoundSqliteAbiShmMap::Mapped(pointer))
            }
        }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn shm_lock(
        &mut self,
        first: u8,
        count: NonZeroU8,
        action: HandleBoundSqliteAbiShmLockAction,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        let action = match action {
            HandleBoundSqliteAbiShmLockAction::LockShared => ManagedSqliteShmLockAction::LockShared,
            HandleBoundSqliteAbiShmLockAction::LockExclusive => {
                ManagedSqliteShmLockAction::LockExclusive
            }
            HandleBoundSqliteAbiShmLockAction::UnlockShared => {
                ManagedSqliteShmLockAction::UnlockShared
            }
            HandleBoundSqliteAbiShmLockAction::UnlockExclusive => {
                ManagedSqliteShmLockAction::UnlockExclusive
            }
        };
        let request = ManagedSqliteShmLockRequest::new(first, count, action).map_err(drop)?;
        self.file
            .shm_lock(request)
            .map(|attempt| match attempt {
                ManagedSqliteShmLockAttempt::Acquired => HandleBoundSqliteAbiAttempt::Acquired,
                ManagedSqliteShmLockAttempt::Contended => HandleBoundSqliteAbiAttempt::Busy,
            })
            .map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn shm_barrier(
        &mut self,
    ) -> Result<(), ()> {
        self.file.shm_barrier().map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn shm_unmap(
        &mut self,
        delete: bool,
    ) -> Result<(), ()> {
        let mode = if delete {
            ManagedSqliteShmUnmapMode::Delete
        } else {
            ManagedSqliteShmUnmapMode::Keep
        };
        self.file.shm_unmap(mode).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn close(
        self,
    ) -> Result<(), ()> {
        self.file.close().map_err(drop)
    }
}

fn map_lock_attempt(attempt: ManagedSqliteLockAttempt) -> HandleBoundSqliteAbiAttempt {
    match attempt {
        ManagedSqliteLockAttempt::Acquired => HandleBoundSqliteAbiAttempt::Acquired,
        ManagedSqliteLockAttempt::Contended => HandleBoundSqliteAbiAttempt::Busy,
    }
}
