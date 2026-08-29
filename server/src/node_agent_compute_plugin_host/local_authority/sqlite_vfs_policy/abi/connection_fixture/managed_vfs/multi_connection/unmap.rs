//! Exact-route Unmap calls and consolidated read-only observations.

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use super::super::connection::ManagedTestUnmapCallbackObservation;
use super::*;
use crate::{
    node_agent_compute_plugin_host::local_authority::{
        sqlite_vfs_abi::HandleBoundSqliteAbiRawSlotSnapshot,
        sqlite_vfs_policy::registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    },
    node_agent_managed_fs::ManagedSqliteShmTestTargetSnapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestUnmapRouteObservation {
    pub(in super::super) target: ManagedTestShmTargetWitness,
    pub(in super::super) physical: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) raw: HandleBoundSqliteAbiRawSlotSnapshot,
    pub(in super::super) active_custody: Option<ManagedSqliteTestVfsRouteCustodySnapshot>,
    pub(in super::super) terminal_custody: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
}

impl ManagedSqliteMultiConnectionFixture {
    pub(in super::super) fn enable_unmap_runtime_observation(
        &self,
        index: usize,
    ) -> anyhow::Result<()> {
        let route = self.route_ordinal(index)?;
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .lifecycle()
            .enable_unmap_runtime_observation(route)
            .map_err(anyhow::Error::msg)
    }

    pub(in super::super) fn call_unmap_raw(
        &self,
        index: usize,
        raw_delete: std::os::raw::c_int,
    ) -> anyhow::Result<ManagedTestUnmapCallbackObservation> {
        self.route(index)?
            .call_main_shm_unmap_raw(raw_delete)
            .map_err(anyhow::Error::msg)
    }

    pub(in super::super) fn acquire_unmap_shm_lock(
        &self,
        index: usize,
        exclusive: bool,
    ) -> anyhow::Result<()> {
        let ownership = if exclusive {
            ffi::SQLITE_SHM_EXCLUSIVE
        } else {
            ffi::SQLITE_SHM_SHARED
        };
        let result = self
            .route(index)?
            .call_main_shm_lock_raw(0, 1, ffi::SQLITE_SHM_LOCK | ownership)
            .map_err(anyhow::Error::msg)?;
        if result == ffi::SQLITE_OK {
            Ok(())
        } else {
            Err(anyhow!(
                "installed xShmLock rejected Unmap precondition with SQLite code {result}"
            ))
        }
    }

    pub(in super::super) fn quarantine_unmap_admission(&self, index: usize) -> anyhow::Result<()> {
        self.route(index)?
            .quarantine_for_unmap_admission_test()
            .map_err(anyhow::Error::msg)
    }

    pub(in super::super) fn observe_unmap_route(
        &self,
        index: usize,
    ) -> anyhow::Result<ManagedTestUnmapRouteObservation> {
        let route = self.route(index)?;
        let binding = route
            .installed_shm_fault_witness()
            .map_err(anyhow::Error::msg)?;
        let terminal_custody = route
            .terminal_custody_test_snapshot()
            .map_err(anyhow::Error::msg)?;
        let active_custody = if terminal_custody.active_route_present() {
            Some(route.route_custody_snapshot().map_err(anyhow::Error::msg)?)
        } else {
            None
        };
        Ok(ManagedTestUnmapRouteObservation {
            target: binding.target_witness().map_err(anyhow::Error::msg)?,
            physical: binding
                .observer()
                .map_err(anyhow::Error::msg)?
                .snapshot()
                .context("observe exact managed SHM Unmap target")?,
            raw: route.observe_main_raw_slots().map_err(anyhow::Error::msg)?,
            active_custody,
            terminal_custody,
        })
    }

    pub(in super::super) fn verify_unmap_sibling_sql(
        &self,
        sibling: usize,
        expected: i64,
    ) -> anyhow::Result<()> {
        let observed: i64 = self.connection(sibling)?.query_row(
            "SELECT value FROM unmap_probe WHERE probe_id=1",
            [],
            |row| row.get(0),
        )?;
        if observed == expected {
            Ok(())
        } else {
            Err(anyhow!(
                "Unmap sibling SQL witness changed: expected {expected}, observed {observed}"
            ))
        }
    }

    pub(in super::super) fn unmap_runtime_trace(
        &self,
        index: usize,
    ) -> anyhow::Result<
        Vec<crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryUnmapRuntimeEvent>,
    >{
        let route = self.route_ordinal(index)?;
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .lifecycle()
            .unmap_runtime_trace(route)
            .map_err(anyhow::Error::msg)
    }

    pub(in super::super) fn finish_unmap_runtime_observation(
        &self,
        index: usize,
    ) -> anyhow::Result<
        Vec<crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryUnmapRuntimeEvent>,
    >{
        let route = self.route_ordinal(index)?;
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .lifecycle()
            .finish_unmap_runtime_observation(route)
            .map_err(anyhow::Error::msg)
    }
}
