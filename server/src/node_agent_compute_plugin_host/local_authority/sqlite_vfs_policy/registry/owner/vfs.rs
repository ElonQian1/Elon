use std::ffi::CString;

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
    abi::{
        ManagedSqliteVfsAccessRequest, ManagedSqliteVfsAccessRequestRejection,
        ManagedSqliteVfsDeleteRequest, ManagedSqliteVfsDeleteRequestRejection,
        ManagedSqliteVfsFullPathnameRequestRejection, ManagedSqliteVfsRequestAbiAdapter,
    },
    ManagedSqliteVfsOpenFlagRejection, ManagedSqliteVfsOpenRequest,
};

impl<Custody: ManagedSqliteRegistryCustody> ManagedSqliteRegistryOwner<Custody> {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn main_logical_name_owned(
        &self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<CString, ManagedSqliteRegistryRouteRejection> {
        Ok(self
            .exact_entry(handle)?
            .policy
            .logical_name(ManagedSqliteLogicalFileRole::Main)
            .to_owned())
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn project_x_open(
        &self,
        handle: ManagedSqliteRegistryRouteHandle,
        candidate_name: Option<&[u8]>,
        raw_flags: i32,
    ) -> Result<
        Result<ManagedSqliteVfsOpenRequest, ManagedSqliteVfsOpenFlagRejection>,
        ManagedSqliteRegistryRouteRejection,
    > {
        Ok(ManagedSqliteVfsRequestAbiAdapter::project_x_open(
            &self.exact_entry(handle)?.policy,
            candidate_name,
            raw_flags,
        ))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn project_x_access(
        &self,
        handle: ManagedSqliteRegistryRouteHandle,
        candidate_name: Option<&[u8]>,
        raw_flag: i32,
    ) -> Result<
        Result<ManagedSqliteVfsAccessRequest, ManagedSqliteVfsAccessRequestRejection>,
        ManagedSqliteRegistryRouteRejection,
    > {
        Ok(ManagedSqliteVfsRequestAbiAdapter::project_x_access(
            &self.exact_entry(handle)?.policy,
            candidate_name,
            raw_flag,
        ))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn project_x_delete(
        &self,
        handle: ManagedSqliteRegistryRouteHandle,
        candidate_name: Option<&[u8]>,
        raw_sync_directory: i32,
    ) -> Result<
        Result<ManagedSqliteVfsDeleteRequest, ManagedSqliteVfsDeleteRequestRejection>,
        ManagedSqliteRegistryRouteRejection,
    > {
        Ok(ManagedSqliteVfsRequestAbiAdapter::project_x_delete(
            &self.exact_entry(handle)?.policy,
            candidate_name,
            raw_sync_directory,
        ))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn project_x_full_pathname(
        &self,
        handle: ManagedSqliteRegistryRouteHandle,
        candidate_name: Option<&[u8]>,
        raw_output_capacity: i32,
    ) -> Result<
        Result<CString, ManagedSqliteVfsFullPathnameRequestRejection>,
        ManagedSqliteRegistryRouteRejection,
    > {
        Ok(ManagedSqliteVfsRequestAbiAdapter::project_x_full_pathname(
            &self.exact_entry(handle)?.policy,
            candidate_name,
            raw_output_capacity,
        )
        .map(|request| request.output().to_owned()))
    }
}
