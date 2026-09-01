//! Real same-FileId, distinct-HANDLE DMS holder for the Q18 initialization path.

use std::{
    fs::{File, OpenOptions},
    os::windows::io::AsRawHandle,
};

use crate::node_agent_managed_fs::{
    platform, same_file_identity, ManagedSqliteAccess, ManagedSqliteFileKind,
    PinnedManagedSqliteFile, PlatformManagedSqliteLockAttempt,
};

use super::super::types::SHM_DMS_OFFSET;

pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) struct ManagedSqliteShmTestQ18DmsHolderLeaseV1
{
    file: Option<File>,
    runtime_generation: u64,
    shm_connection_id: u64,
    target_identity_verified: bool,
    holder_identity_verified: bool,
    same_file_id: bool,
    distinct_handle: bool,
    acquired: bool,
    held_during_target_shared: bool,
    held_during_target_close: bool,
    unlock_attempts: u8,
    unlock_succeeded: bool,
}

impl ManagedSqliteShmTestQ18DmsHolderLeaseV1 {
    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn acquire(
        target: (u64, u64),
        file: &PinnedManagedSqliteFile,
    ) -> Result<Self, &'static str> {
        if target.0 == 0 || target.1 == 0 {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_TARGET_ZERO");
        }
        if file.kind != ManagedSqliteFileKind::Shm
            || file.access != ManagedSqliteAccess::ReadWrite
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_TARGET_FILE_INVALID");
        }
        let target_identity = platform::inspect(&file.file)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_Q18_TARGET_INSPECT_FAILED")?;
        if target_identity.is_directory
            || target_identity.is_reparse_point
            || !same_file_identity(target_identity, file.identity)
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_TARGET_IDENTITY_CHANGED");
        }
        let path = platform::canonical_path(&file.file)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_Q18_TARGET_PATH_FAILED")?;
        let holder = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_REOPEN_FAILED")?;
        let holder_identity = platform::inspect(&holder)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_INSPECT_FAILED")?;
        if holder_identity.is_directory
            || holder_identity.is_reparse_point
            || !same_file_identity(target_identity, holder_identity)
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_FILE_ID_MISMATCH");
        }
        if holder.as_raw_handle() == file.file.as_raw_handle() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_HANDLE_NOT_DISTINCT");
        }
        match platform::try_lock_sqlite_byte_range(&holder, SHM_DMS_OFFSET, 1, true)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_ACQUIRE_FAILED")?
        {
            PlatformManagedSqliteLockAttempt::Acquired => {}
            PlatformManagedSqliteLockAttempt::Contended => {
                return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_ALREADY_CONTENDED")
            }
        }
        Ok(Self {
            file: Some(holder),
            runtime_generation: target.0,
            shm_connection_id: target.1,
            target_identity_verified: true,
            holder_identity_verified: true,
            same_file_id: true,
            distinct_handle: true,
            acquired: true,
            held_during_target_shared: false,
            held_during_target_close: false,
            unlock_attempts: 0,
            unlock_succeeded: false,
        })
    }

    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn mark_held_during_target_shared(
        &mut self,
    ) {
        self.held_during_target_shared = true;
    }

    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn mark_held_during_target_close(
        &mut self,
    ) {
        self.held_during_target_close = true;
    }

    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn release_explicit(
        mut self,
    ) -> Result<[u64; 15], &'static str> {
        if self.unlock_attempts != 0 || !self.acquired {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_RELEASE_SEQUENCE_INVALID");
        }
        self.unlock_attempts = 1;
        let file = self
            .file
            .as_ref()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_MISSING")?;
        platform::unlock_sqlite_byte_range(file, SHM_DMS_OFFSET, 1)
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_RELEASE_FAILED")?;
        self.unlock_succeeded = true;
        let values = self.ordered_values();
        self.file.take();
        Ok(values)
    }

    fn ordered_values(&self) -> [u64; 15] {
        [
            self.runtime_generation,
            self.shm_connection_id,
            SHM_DMS_OFFSET,
            1,
            u64::from(self.target_identity_verified),
            u64::from(self.holder_identity_verified),
            u64::from(self.same_file_id),
            u64::from(self.distinct_handle),
            1,
            1,
            u64::from(self.acquired),
            u64::from(self.held_during_target_shared),
            u64::from(self.held_during_target_close),
            u64::from(self.unlock_attempts),
            u64::from(self.unlock_succeeded),
        ]
    }
}

impl Drop for ManagedSqliteShmTestQ18DmsHolderLeaseV1 {
    fn drop(&mut self) {
        if self.acquired && !self.unlock_succeeded {
            if let Some(file) = self.file.as_ref() {
                let _ = platform::unlock_sqlite_byte_range(file, SHM_DMS_OFFSET, 1);
            }
        }
    }
}
