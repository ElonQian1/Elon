//! Concrete state and unwind boundary for live SQLite I/O callbacks.
//!
//! Production construction is intentionally unreachable while xOpen remains inert. The state
//! nonetheless owns the exact file-custody adapter so callbacks cannot separate a handle from its
//! registry route, leases or SHM lifetime.

use std::{
    num::{NonZeroU32, NonZeroU8},
    os::raw::{c_int, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
    ptr::NonNull,
};

#[cfg(test)]
use std::mem::MaybeUninit;

use rusqlite::ffi;

use super::raw_state;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
    ComputePluginHandleBoundSqliteAbiFile, HandleBoundSqliteAbiAttempt, HandleBoundSqliteAbiFile,
    HandleBoundSqliteAbiLockLevel, HandleBoundSqliteAbiShmLockAction, HandleBoundSqliteAbiShmMap,
    HandleBoundSqliteAbiUnlockLevel, ManagedSqliteRegistryCustody,
    ManagedSqliteRegistryNonceSource,
};

pub(in crate::node_agent_compute_plugin_host::local_authority) trait HandleBoundSqliteFileOperations:
    'static
{
    fn read_at_zero_filled(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, ()>;
    fn write_all_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), ()>;
    fn truncate(&mut self, size: u64) -> Result<(), ()>;
    fn size(&mut self) -> Result<u64, ()>;
    fn full_sync(&mut self) -> Result<(), ()>;
    fn lock_to(
        &mut self,
        level: HandleBoundSqliteAbiLockLevel,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()>;
    fn unlock_to(&mut self, level: HandleBoundSqliteAbiUnlockLevel) -> Result<(), ()>;
    fn check_reserved_lock(&mut self) -> Result<bool, ()>;
    fn shm_map(
        &mut self,
        region: u32,
        region_size: NonZeroU32,
        extend: bool,
    ) -> Result<HandleBoundSqliteAbiShmMap, ()>;
    fn shm_lock(
        &mut self,
        first: u8,
        count: NonZeroU8,
        action: HandleBoundSqliteAbiShmLockAction,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()>;
    fn shm_barrier(&mut self) -> Result<(), ()>;
    fn shm_unmap(&mut self, delete: bool) -> Result<(), ()>;
    fn close(self: Box<Self>) -> Result<(), ()>;
}

#[cfg(test)]
pub(in crate::node_agent_compute_plugin_host::local_authority) fn test_vfs_file_size() -> c_int {
    std::mem::size_of::<super::types::InertHandleBoundSqliteFile>() as c_int
}

/// Installs concrete operations into fresh storage supplied to a test-only registered VFS.
/// Any failed installation drops the operations object, whose managed file custody fails closed.
#[cfg(test)]
pub(in crate::node_agent_compute_plugin_host::local_authority) unsafe fn initialize_test_vfs_file(
    file: *mut ffi::sqlite3_file,
) -> bool {
    // SAFETY: forwarded from the test VFS for its own fresh `szOsFile` allocation.
    unsafe { raw_state::initialize_fresh_file(file) }
}

/// Installs concrete operations into storage initialized by `initialize_test_vfs_file`.
#[cfg(test)]
pub(in crate::node_agent_compute_plugin_host::local_authority) unsafe fn install_test_vfs_file(
    file: *mut ffi::sqlite3_file,
    operations: impl HandleBoundSqliteFileOperations,
) -> Result<(), ()> {
    let state = HandleBoundSqliteFileState::from_test(operations);
    // SAFETY: the exact allocation was initialized by the same serialized xOpen invocation.
    unsafe { raw_state::install_state(file, state) }.map_err(|(_reason, state)| drop(state))
}

impl<Custody, NonceSource> HandleBoundSqliteFileOperations
    for HandleBoundSqliteAbiFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    fn read_at_zero_filled(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, ()> {
        Self::read_at_zero_filled(self, offset, buffer)
    }

    fn write_all_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), ()> {
        Self::write_all_at(self, offset, bytes)
    }

    fn truncate(&mut self, size: u64) -> Result<(), ()> {
        Self::truncate(self, size)
    }

    fn size(&mut self) -> Result<u64, ()> {
        Self::size(self)
    }

    fn full_sync(&mut self) -> Result<(), ()> {
        Self::full_sync(self)
    }

    fn lock_to(
        &mut self,
        level: HandleBoundSqliteAbiLockLevel,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        Self::lock_to(self, level)
    }

    fn unlock_to(&mut self, level: HandleBoundSqliteAbiUnlockLevel) -> Result<(), ()> {
        Self::unlock_to(self, level)
    }

    fn check_reserved_lock(&mut self) -> Result<bool, ()> {
        Self::check_reserved_lock(self)
    }

    fn shm_map(
        &mut self,
        region: u32,
        region_size: NonZeroU32,
        extend: bool,
    ) -> Result<HandleBoundSqliteAbiShmMap, ()> {
        Self::shm_map(self, region, region_size, extend)
    }

    fn shm_lock(
        &mut self,
        first: u8,
        count: NonZeroU8,
        action: HandleBoundSqliteAbiShmLockAction,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        Self::shm_lock(self, first, count, action)
    }

    fn shm_barrier(&mut self) -> Result<(), ()> {
        Self::shm_barrier(self)
    }

    fn shm_unmap(&mut self, delete: bool) -> Result<(), ()> {
        Self::shm_unmap(self, delete)
    }

    fn close(self: Box<Self>) -> Result<(), ()> {
        Self::close(*self)
    }
}

/// Test-only owner for one raw SQLite file allocation installed with real callback state.
/// Production construction remains impossible because this type and constructor are absent from
/// non-test builds.
#[cfg(test)]
pub(in crate::node_agent_compute_plugin_host::local_authority) struct HandleBoundSqliteAbiTestFile {
    storage: Box<MaybeUninit<super::types::InertHandleBoundSqliteFile>>,
    file: *mut ffi::sqlite3_file,
}

#[cfg(test)]
impl HandleBoundSqliteAbiTestFile {
    pub(in crate::node_agent_compute_plugin_host::local_authority) fn install(
        operations: impl HandleBoundSqliteFileOperations,
    ) -> Self {
        let mut storage =
            Box::new(MaybeUninit::<super::types::InertHandleBoundSqliteFile>::uninit());
        let file = storage.as_mut_ptr().cast::<ffi::sqlite3_file>();
        // SAFETY: this owner supplies fresh aligned storage and serializes every callback.
        assert!(unsafe { raw_state::initialize_fresh_file(file) });
        let state = HandleBoundSqliteFileState::from_test(operations);
        // SAFETY: the fresh allocation is initialized and exclusively owned here.
        assert!(unsafe { raw_state::install_state(file, state) }.is_ok());
        Self { storage, file }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn file(
        &self,
    ) -> *mut ffi::sqlite3_file {
        self.file
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn is_cleared(&self) -> bool {
        // SAFETY: installation initialized this exact storage and self keeps it alive.
        let file = unsafe { self.storage.assume_init_ref() };
        file.base.pMethods.is_null() && file.state.is_null()
    }
}

#[cfg(test)]
impl Drop for HandleBoundSqliteAbiTestFile {
    fn drop(&mut self) {
        // SAFETY: this owner has exclusive access. Already-closed state is a no-op.
        let _ = unsafe { raw_state::abandon_installed_state(self.file) };
    }
}

pub(super) struct HandleBoundSqliteFileState {
    file: Option<Box<dyn HandleBoundSqliteFileOperations>>,
}

impl HandleBoundSqliteFileState {
    fn from_compute_plugin(file: ComputePluginHandleBoundSqliteAbiFile) -> Self {
        Self {
            file: Some(Box::new(file)),
        }
    }

    fn file_mut(&mut self) -> Result<&mut (dyn HandleBoundSqliteFileOperations + 'static), ()> {
        self.file.as_deref_mut().ok_or(())
    }

    pub(super) fn read_at_zero_filled(
        &mut self,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<usize, ()> {
        self.file_mut()?.read_at_zero_filled(offset, buffer)
    }

    pub(super) fn write_all_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), ()> {
        self.file_mut()?.write_all_at(offset, bytes)
    }

    pub(super) fn truncate(&mut self, size: u64) -> Result<(), ()> {
        self.file_mut()?.truncate(size)
    }

    pub(super) fn size(&mut self) -> Result<u64, ()> {
        self.file_mut()?.size()
    }

    pub(super) fn full_sync(&mut self) -> Result<(), ()> {
        self.file_mut()?.full_sync()
    }

    pub(super) fn lock_to(
        &mut self,
        level: HandleBoundSqliteAbiLockLevel,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        self.file_mut()?.lock_to(level)
    }

    pub(super) fn unlock_to(&mut self, level: HandleBoundSqliteAbiUnlockLevel) -> Result<(), ()> {
        self.file_mut()?.unlock_to(level)
    }

    pub(super) fn check_reserved_lock(&mut self) -> Result<bool, ()> {
        self.file_mut()?.check_reserved_lock()
    }

    pub(super) fn shm_map(
        &mut self,
        region: u32,
        region_size: NonZeroU32,
        extend: bool,
    ) -> Result<HandleBoundSqliteAbiShmMap, ()> {
        self.file_mut()?.shm_map(region, region_size, extend)
    }

    pub(super) fn shm_lock(
        &mut self,
        first: u8,
        count: NonZeroU8,
        action: HandleBoundSqliteAbiShmLockAction,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        self.file_mut()?.shm_lock(first, count, action)
    }

    pub(super) fn shm_barrier(&mut self) -> Result<(), ()> {
        self.file_mut()?.shm_barrier()
    }

    pub(super) fn shm_unmap(&mut self, delete: bool) -> Result<(), ()> {
        self.file_mut()?.shm_unmap(delete)
    }

    fn close(mut self) -> Result<(), ()> {
        self.file.take().ok_or(())?.close()
    }

    #[cfg(test)]
    fn from_test(file: impl HandleBoundSqliteFileOperations) -> Self {
        Self {
            file: Some(Box::new(file)),
        }
    }
}

/// Runs one typed callback. Any missing/mismatched raw state or Rust panic abandons the installed
/// state, which removes the callback table before Drop fail-closes concrete file custody.
pub(super) unsafe fn run_code(
    file: *mut ffi::sqlite3_file,
    fallback: c_int,
    operation: impl FnOnce(&mut HandleBoundSqliteFileState) -> c_int,
) -> c_int {
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the SQLite callback contract serializes this exact file allocation.
        unsafe { raw_state::with_installed_state(file, operation) }
    }));
    match result {
        Ok(Ok(code)) => code,
        Ok(Err(_)) | Err(_) => {
            unsafe { abandon_without_unwind(file) };
            fallback
        }
    }
}

pub(super) unsafe fn run_void(
    file: *mut ffi::sqlite3_file,
    operation: impl FnOnce(&mut HandleBoundSqliteFileState) -> Result<(), ()>,
) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the SQLite callback contract serializes this exact file allocation.
        unsafe { raw_state::with_installed_state(file, operation) }
    }));
    if !matches!(result, Ok(Ok(Ok(())))) {
        unsafe { abandon_without_unwind(file) };
    }
}

pub(super) unsafe fn close(file: *mut ffi::sqlite3_file, fallback: c_int) -> c_int {
    let state = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: xClose has exclusive consuming access to this exact file allocation.
        unsafe { raw_state::take_installed_state::<HandleBoundSqliteFileState>(file) }
    }));
    let state = match state {
        Ok(Ok(state)) => state,
        Ok(Err(_)) | Err(_) => {
            unsafe { abandon_without_unwind(file) };
            return fallback;
        }
    };
    match catch_unwind(AssertUnwindSafe(|| state.close())) {
        Ok(Ok(())) => ffi::SQLITE_OK,
        Ok(Err(())) | Err(_) => fallback,
    }
}

unsafe fn abandon_without_unwind(file: *mut ffi::sqlite3_file) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: this is the same serialized callback allocation; abandonment validates the
        // exact methods table before taking ownership.
        unsafe { raw_state::abandon_installed_state(file) }
    }));
}

#[cfg(test)]
mod tests;
