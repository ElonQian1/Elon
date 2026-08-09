use std::ffi::CString;

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
    abi::{
        ManagedSqliteVfsAccessRequest, ManagedSqliteVfsAccessRequestRejection,
        ManagedSqliteVfsDeleteRequest, ManagedSqliteVfsDeleteRequestRejection,
        ManagedSqliteVfsFullPathnameRequestRejection,
    },
    ManagedSqliteVfsOpenFlagRejection, ManagedSqliteVfsOpenRequest,
};

impl<Custody, NonceSource> ManagedSqliteRegistryProcessOwner<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn main_logical_name_owned(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<CString, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.main_logical_name_owned(route))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn project_x_open(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        candidate_name: Option<&[u8]>,
        raw_flags: i32,
    ) -> Result<
        Result<ManagedSqliteVfsOpenRequest, ManagedSqliteVfsOpenFlagRejection>,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        self.apply_route(route, |routes| {
            routes.project_x_open(route, candidate_name, raw_flags)
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn project_x_access(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        candidate_name: Option<&[u8]>,
        raw_flag: i32,
    ) -> Result<
        Result<ManagedSqliteVfsAccessRequest, ManagedSqliteVfsAccessRequestRejection>,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        self.apply_route(route, |routes| {
            routes.project_x_access(route, candidate_name, raw_flag)
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn project_x_delete(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        candidate_name: Option<&[u8]>,
        raw_sync_directory: i32,
    ) -> Result<
        Result<ManagedSqliteVfsDeleteRequest, ManagedSqliteVfsDeleteRequestRejection>,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        self.apply_route(route, |routes| {
            routes.project_x_delete(route, candidate_name, raw_sync_directory)
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn project_x_full_pathname(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        candidate_name: Option<&[u8]>,
        raw_output_capacity: i32,
    ) -> Result<
        Result<CString, ManagedSqliteVfsFullPathnameRequestRejection>,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        self.apply_route(route, |routes| {
            routes.project_x_full_pathname(route, candidate_name, raw_output_capacity)
        })
    }
}
