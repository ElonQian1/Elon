use std::ffi::CStr;

use super::super::{ManagedSqliteLogicalFileRole, ManagedSqliteLogicalNameRejection};

/// Borrowed bytes copied out of SQLite's authorizer callback arguments by a future raw boundary.
///
/// `argument_three` corresponds to the fifth callback parameter. It is usually a database name,
/// but bundled SQLite 3.45 uses it as a column name for `ALTER TABLE ... DROP COLUMN`.
#[derive(Clone, Copy)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteRawAuthorizerRequest<
    'a,
> {
    pub(super) action_code: i32,
    pub(super) argument_one: Option<&'a [u8]>,
    pub(super) argument_two: Option<&'a [u8]>,
    pub(super) argument_three: Option<&'a [u8]>,
    pub(super) accessor: Option<&'a [u8]>,
}

impl<'a> ManagedSqliteRawAuthorizerRequest<'a> {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn new(
        action_code: i32,
        argument_one: Option<&'a [u8]>,
        argument_two: Option<&'a [u8]>,
        argument_three: Option<&'a [u8]>,
        accessor: Option<&'a [u8]>,
    ) -> Self {
        Self {
            action_code,
            argument_one,
            argument_two,
            argument_three,
            accessor,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) enum ManagedSqliteAuthorizerRawField
{
    ArgumentOne,
    ArgumentTwo,
    ArgumentThree,
    Accessor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) enum ManagedSqliteAuthorizerAbiRejection
{
    UnknownActionCode(i32),
    InvalidUtf8(ManagedSqliteAuthorizerRawField),
    InvalidArgumentShape(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteVfsAccessRequest
{
    role: ManagedSqliteLogicalFileRole,
}

impl ManagedSqliteVfsAccessRequest {
    pub(super) fn new(role: ManagedSqliteLogicalFileRole) -> Self {
        Self { role }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn role(
        self,
    ) -> ManagedSqliteLogicalFileRole {
        self.role
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) enum ManagedSqliteVfsAccessRequestRejection
{
    LogicalName(ManagedSqliteLogicalNameRejection),
    UnsupportedAccessFlag(i32),
    UnsupportedRole(ManagedSqliteLogicalFileRole),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteVfsDeleteRequest
{
    role: ManagedSqliteLogicalFileRole,
    sync_parent: bool,
}

impl ManagedSqliteVfsDeleteRequest {
    pub(super) fn new(role: ManagedSqliteLogicalFileRole, sync_parent: bool) -> Self {
        Self { role, sync_parent }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn role(
        self,
    ) -> ManagedSqliteLogicalFileRole {
        self.role
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn sync_parent(
        self,
    ) -> bool {
        self.sync_parent
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) enum ManagedSqliteVfsDeleteRequestRejection
{
    LogicalName(ManagedSqliteLogicalNameRejection),
    InvalidSyncDirectoryFlag(i32),
    UnsupportedRole(ManagedSqliteLogicalFileRole),
    InvalidRoleSyncMatrix,
}

#[derive(Clone, Copy)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteVfsFullPathnameRequest<
    'a,
> {
    output: &'a CStr,
}

impl<'a> ManagedSqliteVfsFullPathnameRequest<'a> {
    pub(super) fn new(output: &'a CStr) -> Self {
        Self { output }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn output(
        self,
    ) -> &'a CStr {
        self.output
    }

    pub(super) fn required_output_bytes(self) -> usize {
        self.output.to_bytes_with_nul().len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) enum ManagedSqliteVfsFullPathnameRequestRejection
{
    LogicalName(ManagedSqliteLogicalNameRejection),
    UnsupportedRole(ManagedSqliteLogicalFileRole),
    InvalidOutputCapacity(i32),
}
