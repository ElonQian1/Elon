//! Sealed RegistryLifecycle controls and post-close witnesses for one selected route.

use anyhow::{anyhow, Context};

use super::{ManagedSqliteMultiConnectionFixture, ManagedTestRouteOrdinal};
use crate::node_agent_managed_fs::ManagedSqliteShmTestTopologySnapshot;

use super::super::{
    connection::{
        ManagedTestRegistryLifecycleCloseOutcome, ManagedTestRegistryLifecycleRouteObserver,
    },
    lifecycle_faults::{
        ManagedTestRegistryLifecycleControl, ManagedTestRegistryLifecycleTraceSnapshot,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct ManagedTestRegistryLifecycleRouteSnapshot {
    live_routes: usize,
    logical_names: usize,
}

impl ManagedTestRegistryLifecycleRouteSnapshot {
    pub(in super::super) fn live_routes(&self) -> usize {
        self.live_routes
    }

    pub(in super::super) fn logical_names(&self) -> usize {
        self.logical_names
    }
}

impl ManagedSqliteMultiConnectionFixture {
    pub(in super::super) fn registry_lifecycle_sqlite_connection_count(
        &self,
    ) -> anyhow::Result<u8> {
        u8::try_from(
            self.connections
                .iter()
                .filter(|connection| connection.is_some())
                .count(),
        )
        .context("RegistryLifecycle live SQLite connection count exceeds u8")
    }

    pub(in super::super) fn registration_id(&self) -> u64 {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .registration_id()
            .counter_value()
    }

    pub(in super::super) fn install_registry_lifecycle_control(
        &self,
        index: usize,
        control: ManagedTestRegistryLifecycleControl,
    ) -> anyhow::Result<()> {
        self.route(index)?
            .registry_lifecycle_binding()?
            .install_registry_lifecycle_control(control)
            .map_err(anyhow::Error::msg)
    }

    pub(in super::super) fn registry_lifecycle_route_observer(
        &self,
        index: usize,
    ) -> anyhow::Result<ManagedTestRegistryLifecycleRouteObserver> {
        self.route(index)?.registry_lifecycle_route_observer()
    }

    pub(in super::super) fn registry_lifecycle_trace(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> anyhow::Result<ManagedTestRegistryLifecycleTraceSnapshot> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .lifecycle()
            .registry_trace(route)
            .map_err(anyhow::Error::msg)
    }

    pub(in super::super) fn retain_outstanding_journal_sidecar(
        &self,
        index: usize,
    ) -> anyhow::Result<()> {
        let selected = self.route(index)?;
        let registration = self
            .registration
            .as_ref()
            .expect("managed VFS registration");
        let context = registration
            .context
            .as_ref()
            .context("managed VFS context")?;
        selected.retain_outstanding_journal_sidecar(&context.runtime)
    }

    pub(in super::super) fn close_registry_lifecycle_once(
        &mut self,
        index: usize,
    ) -> anyhow::Result<ManagedTestRegistryLifecycleCloseOutcome> {
        let selected = self
            .connections
            .get_mut(index)
            .and_then(Option::take)
            .ok_or_else(|| anyhow!("managed SQLite connection {index} is not live"))?;
        selected.close_registry_lifecycle_once()
    }

    pub(in super::super) fn registry_lifecycle_route_snapshot(
        &self,
    ) -> anyhow::Result<ManagedTestRegistryLifecycleRouteSnapshot> {
        let routes = self
            .registration
            .as_ref()
            .expect("managed VFS registration")
            .routes();
        let (live_routes, logical_names) = routes.live_route_index_counts()?;
        Ok(ManagedTestRegistryLifecycleRouteSnapshot {
            live_routes,
            logical_names,
        })
    }

    pub(in super::super) fn registry_lifecycle_runtime_snapshot(
        &self,
    ) -> anyhow::Result<ManagedSqliteShmTestTopologySnapshot> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .context
            .as_ref()
            .context("managed VFS context")?
            .runtime
            .test_topology_snapshot()
            .map_err(|failure| anyhow!("observe RegistryLifecycle runtime: {failure:?}"))
    }
}
