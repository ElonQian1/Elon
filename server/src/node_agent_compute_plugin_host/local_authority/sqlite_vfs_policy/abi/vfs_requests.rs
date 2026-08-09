use rusqlite::ffi;

use super::super::{
    ManagedSqliteLogicalFileRole as Role, ManagedSqliteVfsOpenFlagRejection,
    ManagedSqliteVfsOpenRequest, SealedHandleBoundSqlitePolicy,
};
use super::types::{
    ManagedSqliteVfsAccessRequest, ManagedSqliteVfsAccessRequestRejection,
    ManagedSqliteVfsDeleteRequest, ManagedSqliteVfsDeleteRequestRejection,
    ManagedSqliteVfsFullPathnameRequest, ManagedSqliteVfsFullPathnameRequestRejection,
};

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteVfsRequestAbiAdapter;

impl ManagedSqliteVfsRequestAbiAdapter {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn project_x_open(
        policy: &SealedHandleBoundSqlitePolicy,
        candidate_name: Option<&[u8]>,
        raw_flags: i32,
    ) -> Result<ManagedSqliteVfsOpenRequest, ManagedSqliteVfsOpenFlagRejection> {
        policy.authorize_vfs_open(candidate_name, raw_flags)
    }

    /// SQLite 3.45 uses `READWRITE` only for `temp_store_directory` and never uses `READ`.
    /// The handle-bound authority therefore accepts existence checks for exact recoverable
    /// sidecars only.
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn project_x_access(
        policy: &SealedHandleBoundSqlitePolicy,
        candidate_name: Option<&[u8]>,
        raw_flag: i32,
    ) -> Result<ManagedSqliteVfsAccessRequest, ManagedSqliteVfsAccessRequestRejection> {
        if raw_flag != ffi::SQLITE_ACCESS_EXISTS {
            return Err(ManagedSqliteVfsAccessRequestRejection::UnsupportedAccessFlag(raw_flag));
        }
        let role = policy
            .classify_logical_name(candidate_name)
            .map_err(ManagedSqliteVfsAccessRequestRejection::LogicalName)?;
        match role {
            Role::Journal | Role::Wal => Ok(ManagedSqliteVfsAccessRequest::new(role)),
            Role::Main => Err(ManagedSqliteVfsAccessRequestRejection::UnsupportedRole(
                role,
            )),
        }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn project_x_delete(
        policy: &SealedHandleBoundSqlitePolicy,
        candidate_name: Option<&[u8]>,
        raw_sync_directory: i32,
    ) -> Result<ManagedSqliteVfsDeleteRequest, ManagedSqliteVfsDeleteRequestRejection> {
        let sync_parent = match raw_sync_directory {
            0 => false,
            1 => true,
            other => {
                return Err(
                    ManagedSqliteVfsDeleteRequestRejection::InvalidSyncDirectoryFlag(other),
                );
            }
        };
        let role = policy
            .classify_logical_name(candidate_name)
            .map_err(ManagedSqliteVfsDeleteRequestRejection::LogicalName)?;
        match (role, sync_parent) {
            (Role::Journal, _) | (Role::Wal, false) => {
                Ok(ManagedSqliteVfsDeleteRequest::new(role, sync_parent))
            }
            (Role::Wal, true) => Err(ManagedSqliteVfsDeleteRequestRejection::InvalidRoleSyncMatrix),
            (Role::Main, _) => Err(ManagedSqliteVfsDeleteRequestRejection::UnsupportedRole(
                role,
            )),
        }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn project_x_full_pathname<
        'a,
    >(
        policy: &'a SealedHandleBoundSqlitePolicy,
        candidate_name: Option<&[u8]>,
        raw_output_capacity: i32,
    ) -> Result<ManagedSqliteVfsFullPathnameRequest<'a>, ManagedSqliteVfsFullPathnameRequestRejection>
    {
        let role = policy
            .classify_logical_name(candidate_name)
            .map_err(ManagedSqliteVfsFullPathnameRequestRejection::LogicalName)?;
        if role != Role::Main {
            return Err(ManagedSqliteVfsFullPathnameRequestRejection::UnsupportedRole(role));
        }
        let request = ManagedSqliteVfsFullPathnameRequest::new(policy.logical_name(Role::Main));
        let required = request.required_output_bytes();
        let capacity = usize::try_from(raw_output_capacity).map_err(|_| {
            ManagedSqliteVfsFullPathnameRequestRejection::InvalidOutputCapacity(raw_output_capacity)
        })?;
        if capacity < required {
            return Err(
                ManagedSqliteVfsFullPathnameRequestRejection::InvalidOutputCapacity(
                    raw_output_capacity,
                ),
            );
        }
        Ok(request)
    }
}
