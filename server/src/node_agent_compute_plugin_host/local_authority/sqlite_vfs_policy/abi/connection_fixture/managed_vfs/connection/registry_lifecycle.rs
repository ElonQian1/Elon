//! Registry-lifecycle closure for one managed routed SQLite connection fixture.

use std::{mem, sync::Arc};

use anyhow::{anyhow, Error};

use super::{
    ManagedSqliteRoutedConnectionFixture, ManagedTestLifecycleFaultBinding,
    ManagedTestRegistryLifecycleTraceSnapshot, ManagedTestVfsCounts, PinnedManagedSqliteWalRuntime,
    TestRoute,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::{
        ManagedSqliteRegistryLifecycleStage, ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    },
    node_agent_managed_fs::{ManagedSqliteAccess, ManagedSqliteFileKind, ManagedSqliteOpenMode},
};

#[derive(Debug)]
pub(in super::super) enum ManagedTestRegistryLifecycleCloseOutcome {
    XCloseRejected(Error),
    LogicalRetirementRejected(Error),
    Success(ManagedTestVfsCounts),
}

#[derive(Clone)]
pub(in super::super) struct ManagedTestRegistryLifecycleRouteObserver {
    route: Arc<TestRoute>,
    lifecycle: ManagedTestLifecycleFaultBinding,
}

impl ManagedTestRegistryLifecycleRouteObserver {
    pub(in super::super) fn terminal_custody(
        &self,
    ) -> anyhow::Result<ManagedSqliteRegistryTerminalCustodyTestSnapshot> {
        self.route
            .terminal_custody_test_snapshot()
            .map_err(|()| anyhow!("observe selected RegistryLifecycle terminal custody"))
    }

    pub(in super::super) fn trace(
        &self,
    ) -> anyhow::Result<ManagedTestRegistryLifecycleTraceSnapshot> {
        self.lifecycle
            .registry_lifecycle_trace()
            .map_err(anyhow::Error::msg)
    }
}

impl ManagedTestRegistryLifecycleCloseOutcome {
    fn into_result(self) -> anyhow::Result<ManagedTestVfsCounts> {
        match self {
            Self::XCloseRejected(error) | Self::LogicalRetirementRejected(error) => Err(error),
            Self::Success(counts) => Ok(counts),
        }
    }
}

pub(super) fn lifecycle_binding(
    fixture: &ManagedSqliteRoutedConnectionFixture,
) -> anyhow::Result<ManagedTestLifecycleFaultBinding> {
    Ok(fixture
        .route_entry
        .as_ref()
        .ok_or_else(|| anyhow!("managed RegistryLifecycle route entry is not live"))?
        .lifecycle())
}

pub(super) fn route_observer(
    fixture: &ManagedSqliteRoutedConnectionFixture,
) -> anyhow::Result<ManagedTestRegistryLifecycleRouteObserver> {
    Ok(ManagedTestRegistryLifecycleRouteObserver {
        route: Arc::clone(&fixture.route),
        lifecycle: lifecycle_binding(fixture)?,
    })
}

pub(super) fn retain_outstanding_journal_sidecar(
    fixture: &ManagedSqliteRoutedConnectionFixture,
    runtime: &Arc<PinnedManagedSqliteWalRuntime>,
) -> anyhow::Result<()> {
    let opened = runtime
        .namespace()
        .open(
            ManagedSqliteFileKind::Journal,
            ManagedSqliteAccess::ReadWrite,
            ManagedSqliteOpenMode::OpenOrCreate,
        )
        .map_err(|failure| anyhow!("open RegistryLifecycle Journal sentinel: {failure:?}"))?;
    lifecycle_binding(fixture)?
        .install_connection_observation_sidecar(opened)
        .map_err(anyhow::Error::msg)
}

pub(super) fn close(
    mut fixture: ManagedSqliteRoutedConnectionFixture,
) -> anyhow::Result<ManagedTestVfsCounts> {
    let counts = close_connection(&mut fixture)?;
    if let Some(registration) = fixture.registration.take() {
        registration.unregister()?;
    }
    Ok(counts)
}

pub(super) fn close_connection(
    fixture: &mut ManagedSqliteRoutedConnectionFixture,
) -> anyhow::Result<ManagedTestVfsCounts> {
    close_connection_detailed(fixture)?.into_result()
}

pub(super) fn close_connection_detailed(
    fixture: &mut ManagedSqliteRoutedConnectionFixture,
) -> anyhow::Result<ManagedTestRegistryLifecycleCloseOutcome> {
    if let Err(error) = fixture.uninstall_authorizer() {
        if let Some(connection) = fixture.connection.take() {
            mem::forget(connection);
        }
        return Err(error);
    }
    drop(fixture.authorizer.take());
    let Some(connection) = fixture.connection.take() else {
        return Err(anyhow!("managed routed SQLite connection already consumed"));
    };
    if let Err((connection, error)) = connection.close() {
        mem::forget(connection);
        return Err(anyhow!(
            "close managed routed SQLite connection: {error}; connection retained"
        ));
    }
    let lifecycle = lifecycle_binding(fixture)?;
    let trace = lifecycle
        .registry_lifecycle_trace()
        .map_err(anyhow::Error::msg)?;
    if trace.count(ManagedSqliteRegistryLifecycleStage::RetirementPublishSucceeded) != 1 {
        return Ok(ManagedTestRegistryLifecycleCloseOutcome::XCloseRejected(
            anyhow!("managed routed SQLite xClose rejected before retirement publication"),
        ));
    }
    let route_entry = fixture
        .route_entry
        .as_ref()
        .expect("managed fixture route entry");
    let logical_removal = match fixture.routes.retire_closed_route(route_entry) {
        Ok(receipt) => receipt,
        Err(error) => {
            return Ok(ManagedTestRegistryLifecycleCloseOutcome::LogicalRetirementRejected(error))
        }
    };
    drop(logical_removal);
    fixture.route_entry.take();
    Ok(ManagedTestRegistryLifecycleCloseOutcome::Success(
        fixture.counters.snapshot(),
    ))
}
