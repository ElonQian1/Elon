//! Real, same-process Win32 byte-range contention for the isolated Lock quotient runner.

use std::{
    fs::{File, OpenOptions},
    os::windows::io::AsRawHandle,
};

use crate::node_agent_managed_fs::{
    platform, same_file_identity, ManagedSqliteAccess, ManagedSqliteFileKind,
    PlatformManagedSqliteLockAttempt,
};

use super::{
    super::{
        coordinator::ManagedSqliteShmDmsCustody,
        types::{SHM_LOCK_BASE, SHM_LOCK_COUNT},
    },
    api::ManagedSqliteShmTestTargetObserver,
};

/// Copy-only receipt for the holder whose lifetime enclosed the installed `xShmLock` callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestNativeContentionReceipt {
    pub(crate) runtime_generation: u64,
    pub(crate) shm_connection_id: u64,
    pub(crate) absolute_offset: u64,
    pub(crate) length: u64,
    pub(crate) target_identity_verified: bool,
    pub(crate) holder_identity_verified: bool,
    pub(crate) distinct_handle: bool,
    pub(crate) exclusive_holder: bool,
    pub(crate) acquire_attempts: u8,
    pub(crate) acquired: bool,
    pub(crate) held_during_callback: bool,
    pub(crate) released: bool,
}

struct NativeContentionLease {
    file: Option<File>,
    receipt: ManagedSqliteShmTestNativeContentionReceipt,
}

impl ManagedSqliteShmTestTargetObserver {
    /// Holds one real exclusive `LockFileEx` range on a separately opened handle while `callback`
    /// invokes the installed VFS method. Returning a receipt requires an explicit `UnlockFileEx`.
    pub(crate) fn with_native_lock_contention<T>(
        &self,
        first: u8,
        count: u8,
        callback: impl FnOnce() -> Result<T, &'static str>,
    ) -> Result<(T, ManagedSqliteShmTestNativeContentionReceipt), &'static str> {
        let mut lease = self.acquire_native_lock_contention(first, count)?;
        let callback_result = callback();
        lease.receipt.held_during_callback = true;
        let release_result = lease.release();
        match (callback_result, release_result) {
            (Ok(value), Ok(receipt)) => Ok((value, receipt)),
            (Err(error), _) => Err(error),
            (_, Err(error)) => Err(error),
        }
    }

    fn acquire_native_lock_contention(
        &self,
        first: u8,
        count: u8,
    ) -> Result<NativeContentionLease, &'static str> {
        let end = first
            .checked_add(count)
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_RANGE_OVERFLOW")?;
        if count == 0 || first >= SHM_LOCK_COUNT || end > SHM_LOCK_COUNT {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_RANGE_INVALID");
        }
        let (runtime_generation, shm_connection_id) = self.target.identity();
        let state = self
            .coordinator
            .state
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_STATE_POISONED")?;
        if runtime_generation == 0
            || shm_connection_id == 0
            || self.coordinator.generation.get() != runtime_generation
            || self.coordinator.test_fault_target(shm_connection_id) != self.target
            || state.poisoned.is_some()
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_TARGET_MISMATCH");
        }
        let connection = state
            .connections
            .get(&shm_connection_id)
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_TARGET_DETACHED")?;
        if connection.shared_mask != 0 || connection.exclusive_mask != 0 {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_PRESTATE_INVALID");
        }
        let node = state
            .node
            .as_ref()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_NODE_MISSING")?;
        if node.dms != ManagedSqliteShmDmsCustody::Shared {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_DMS_INVALID");
        }
        if node.file.kind != ManagedSqliteFileKind::Shm
            || node.file.access != ManagedSqliteAccess::ReadWrite
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_FILE_INVALID");
        }
        let target_identity = platform::inspect(&node.file.file)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_TARGET_INSPECT_FAILED")?;
        if target_identity.is_directory
            || target_identity.is_reparse_point
            || !same_file_identity(target_identity, node.file.identity)
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_TARGET_IDENTITY_CHANGED");
        }
        let path = platform::canonical_path(&node.file.file)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_PATH_FAILED")?;
        let holder = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_REOPEN_FAILED")?;
        let holder_identity = platform::inspect(&holder)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_HOLDER_INSPECT_FAILED")?;
        if holder_identity.is_directory
            || holder_identity.is_reparse_point
            || !same_file_identity(target_identity, holder_identity)
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_HOLDER_IDENTITY_MISMATCH");
        }
        if holder.as_raw_handle() == node.file.file.as_raw_handle() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_HANDLE_NOT_DISTINCT");
        }
        let absolute_offset = SHM_LOCK_BASE + u64::from(first);
        match platform::try_lock_sqlite_byte_range(&holder, absolute_offset, u64::from(count), true)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_ACQUIRE_FAILED")?
        {
            PlatformManagedSqliteLockAttempt::Acquired => {}
            PlatformManagedSqliteLockAttempt::Contended => {
                return Err("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_ALREADY_CONTENDED")
            }
        }
        drop(state);
        Ok(NativeContentionLease {
            file: Some(holder),
            receipt: ManagedSqliteShmTestNativeContentionReceipt {
                runtime_generation,
                shm_connection_id,
                absolute_offset,
                length: u64::from(count),
                target_identity_verified: true,
                holder_identity_verified: true,
                distinct_handle: true,
                exclusive_holder: true,
                acquire_attempts: 1,
                acquired: true,
                held_during_callback: false,
                released: false,
            },
        })
    }
}

impl NativeContentionLease {
    fn release(mut self) -> Result<ManagedSqliteShmTestNativeContentionReceipt, &'static str> {
        let file = self
            .file
            .take()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_HOLDER_MISSING")?;
        platform::unlock_sqlite_byte_range(
            &file,
            self.receipt.absolute_offset,
            self.receipt.length,
        )
        .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_NATIVE_CONTENTION_RELEASE_FAILED")?;
        self.receipt.released = true;
        Ok(self.receipt)
    }
}

impl Drop for NativeContentionLease {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = platform::unlock_sqlite_byte_range(
                &file,
                self.receipt.absolute_offset,
                self.receipt.length,
            );
        }
    }
}
