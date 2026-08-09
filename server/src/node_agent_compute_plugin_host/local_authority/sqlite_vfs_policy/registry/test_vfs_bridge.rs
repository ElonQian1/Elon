//! Test-only bridge from a registered SQLite VFS into the exact registry and file custody.
//!
//! The bridge supports main, rollback-journal and WAL files. The first main-file SHM map consumes
//! ordinary main custody into a same-route WAL-main + SHM pair; production registration remains
//! unavailable.

use std::{ffi::CString, sync::Arc};

use super::{
    file_custody::{HandleBoundSqliteAbiFile, ManagedSqliteRegistryPinnedFile},
    owner::{ManagedSqliteRegistryCustody, ManagedSqliteRegistryRouteHandle},
    process_owner::{
        ManagedSqliteRegistryNonceSource, ManagedSqliteRegistryProcessOwner,
        ManagedSqliteRegistryRoutedCallbackLease,
    },
    types::{ManagedSqliteRegistryCallbackKind, ManagedSqliteRegistryTerminalReason},
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
        abi::{ManagedSqliteVfsAccessRequest, ManagedSqliteVfsDeleteRequest},
        ManagedSqliteAuthorizerDecision, ManagedSqliteAuthorizerRequest,
        ManagedSqliteLogicalFileRole, ManagedSqliteVfsOpenRequest,
    },
    node_agent_managed_fs::{
        PinnedManagedSqliteFile, PinnedManagedSqliteMainFile, PinnedManagedSqliteWalRuntime,
    },
};

mod file;

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) use file::ManagedSqliteTestVfsFile;

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteTestVfsRoute<
    Custody,
    NonceSource,
> where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
    route: ManagedSqliteRegistryRouteHandle,
}

impl<Custody, NonceSource> ManagedSqliteTestVfsRoute<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn register(
        owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
        custody: Custody,
    ) -> Result<Self, ()> {
        let route = owner.register(custody).map_err(drop)?;
        owner.begin_open_attempt(route).map_err(drop)?;
        Ok(Self { owner, route })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn main_logical_name(
        &self,
    ) -> Result<CString, ()> {
        self.owner.main_logical_name_owned(self.route).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn begin_open_callback(
        &self,
    ) -> Result<ManagedSqliteTestVfsCallback<Custody, NonceSource>, ()> {
        self.begin_callback(ManagedSqliteRegistryCallbackKind::Open)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn begin_access_callback(
        &self,
    ) -> Result<ManagedSqliteTestVfsCallback<Custody, NonceSource>, ()> {
        self.begin_callback(ManagedSqliteRegistryCallbackKind::Access)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn begin_delete_callback(
        &self,
    ) -> Result<ManagedSqliteTestVfsCallback<Custody, NonceSource>, ()> {
        self.begin_callback(ManagedSqliteRegistryCallbackKind::Delete)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn begin_full_pathname_callback(
        &self,
    ) -> Result<ManagedSqliteTestVfsCallback<Custody, NonceSource>, ()> {
        self.begin_callback(ManagedSqliteRegistryCallbackKind::FullPathname)
    }

    fn begin_callback(
        &self,
        kind: ManagedSqliteRegistryCallbackKind,
    ) -> Result<ManagedSqliteTestVfsCallback<Custody, NonceSource>, ()> {
        Ok(ManagedSqliteTestVfsCallback {
            lease: self.owner.begin_callback(self.route, kind).map_err(drop)?,
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn project_x_open(
        &self,
        candidate_name: Option<&[u8]>,
        raw_flags: i32,
    ) -> Result<ManagedSqliteVfsOpenRequest, ()> {
        self.owner
            .project_x_open(self.route, candidate_name, raw_flags)
            .map_err(drop)?
            .map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn project_x_access(
        &self,
        candidate_name: Option<&[u8]>,
        raw_flag: i32,
    ) -> Result<ManagedSqliteVfsAccessRequest, ()> {
        self.owner
            .project_x_access(self.route, candidate_name, raw_flag)
            .map_err(drop)?
            .map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn project_x_delete(
        &self,
        candidate_name: Option<&[u8]>,
        raw_sync_directory: i32,
    ) -> Result<ManagedSqliteVfsDeleteRequest, ()> {
        self.owner
            .project_x_delete(self.route, candidate_name, raw_sync_directory)
            .map_err(drop)?
            .map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn project_x_full_pathname(
        &self,
        candidate_name: Option<&[u8]>,
        raw_output_capacity: i32,
    ) -> Result<CString, ()> {
        self.owner
            .project_x_full_pathname(self.route, candidate_name, raw_output_capacity)
            .map_err(drop)?
            .map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn bind_main(
        &self,
        file: PinnedManagedSqliteMainFile,
        wal_runtime: Arc<PinnedManagedSqliteWalRuntime>,
    ) -> Result<ManagedSqliteTestVfsFile<Custody, NonceSource>, ()> {
        let lease = match self.owner.claim_main(self.route) {
            Ok(lease) => lease,
            Err(_) => {
                let _ = self.retain_failure(file);
                return Err(());
            }
        };
        let pinned =
            ManagedSqliteRegistryPinnedFile::bind_main(self.owner, self.route, file, lease)
                .map_err(drop)?;
        Ok(ManagedSqliteTestVfsFile::new(
            HandleBoundSqliteAbiFile::from_pinned(pinned),
            self.owner,
            self.route,
            ManagedSqliteLogicalFileRole::Main,
            Some(wal_runtime),
        ))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn bind_sidecar(
        &self,
        file: PinnedManagedSqliteFile,
        role: ManagedSqliteLogicalFileRole,
    ) -> Result<ManagedSqliteTestVfsFile<Custody, NonceSource>, ()> {
        let lease = match self.owner.claim_sidecar(self.route, role) {
            Ok(lease) => lease,
            Err(_) => {
                let _ = self.retain_failure(file);
                return Err(());
            }
        };
        let pinned =
            ManagedSqliteRegistryPinnedFile::bind_sidecar(self.owner, self.route, file, lease)
                .map_err(drop)?;
        Ok(ManagedSqliteTestVfsFile::new(
            HandleBoundSqliteAbiFile::from_pinned(pinned),
            self.owner,
            self.route,
            role,
            None,
        ))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn activate_after_main_open(
        &self,
    ) -> Result<(), ()> {
        self.owner.activate_connection(self.route).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn authorize_sql(
        &self,
        request: ManagedSqliteAuthorizerRequest<'_>,
    ) -> Result<ManagedSqliteAuthorizerDecision, ()> {
        self.owner.authorize_sql(self.route, request).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn enter_schema_migration(
        &self,
    ) -> Result<(), ()> {
        self.owner.enter_schema_migration(self.route).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn enter_runtime(
        &self,
    ) -> Result<(), ()> {
        self.owner.enter_runtime(self.route).map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn retain_failure<
        Retained: 'static,
    >(
        &self,
        custody: Retained,
    ) -> Result<(), ()> {
        self.owner
            .retain_terminal_custody(
                self.route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                custody,
            )
            .map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn abort_unopened_for_test(
        &self,
    ) {
        if self.owner.begin_connection_close(self.route).is_ok()
            && self.owner.observe_connection_closed(self.route).is_ok()
        {
            let _ = self.owner.retire_closed(self.route);
        }
    }
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteTestVfsCallback<
    Custody,
    NonceSource,
> where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    lease: ManagedSqliteRegistryRoutedCallbackLease<Custody, NonceSource>,
}

impl<Custody, NonceSource> ManagedSqliteTestVfsCallback<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn complete(
        self,
    ) -> Result<(), ()> {
        self.lease.complete().map_err(drop)
    }
}
