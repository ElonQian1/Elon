//! Real one/two-Connection WAL fixture retained until RegistryLifecycle child exit.

use std::{mem, path::Path, time::Duration};

use anyhow::{anyhow, Context};

use super::super::{
    connection::{
        ManagedTestRegistryLifecycleCloseOutcome, ManagedTestRegistryLifecycleRouteObserver,
    },
    lifecycle_faults::{
        ManagedTestRegistryLifecycleControl, ManagedTestRegistryLifecycleTraceSnapshot,
    },
    ManagedSqliteMultiConnectionFixture, ManagedSqliteTestVfsRouteCustodySnapshot,
    ManagedTestLifecycleFaultObservation, ManagedTestLifecycleFaultStep, ManagedTestRouteOrdinal,
    ManagedTestShmFaultPlanBinding, ManagedTestVfsLiveRegistrationSnapshot,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::{
        sqlite_vfs_abi::{
            HandleBoundSqliteAbiRawCloseWitness, HandleBoundSqliteAbiRawCloseWitnessSnapshot,
        },
        sqlite_vfs_policy::registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    },
    node_agent_managed_fs::ManagedSqliteShmTestTopologySnapshot,
};

const SELECTED: usize = 0;
const SIBLING: usize = 1;
const PROBE_VALUE: i64 = 829;

pub(super) struct RegistryLifecycleRouteSnapshot {
    pub(super) live_routes: usize,
    pub(super) logical_names: usize,
}

pub(super) struct RetainedRegistryLifecycleFixture {
    fixture: Option<ManagedSqliteMultiConnectionFixture>,
    selected_observer: ManagedTestRegistryLifecycleRouteObserver,
    raw_close_witness: HandleBoundSqliteAbiRawCloseWitness,
    selected_ordinal: ManagedTestRouteOrdinal,
    selected_close_attempts: u8,
}

impl RetainedRegistryLifecycleFixture {
    pub(super) fn prepare(root: &Path, shared: bool) -> anyhow::Result<Self> {
        let fixture = if shared {
            ManagedSqliteMultiConnectionFixture::open(root, [0xc3; 16])?
        } else {
            ManagedSqliteMultiConnectionFixture::open_single(root, [0xc3; 16])?
        };
        let selected_ordinal = fixture.route_ordinal(SELECTED)?;
        let selected_observer = fixture.registry_lifecycle_route_observer(SELECTED)?;
        let raw_close_witness = fixture
            .route(SELECTED)?
            .observe_main_raw_close_witness()
            .map_err(anyhow::Error::msg)?;
        let mut retained = Self {
            fixture: Some(fixture),
            selected_observer,
            raw_close_witness,
            selected_ordinal,
            selected_close_attempts: 0,
        };
        retained.prepare_wal(shared)?;
        retained.validate_pre_topology(shared)?;
        Ok(retained)
    }

    fn prepare_wal(&mut self, shared: bool) -> anyhow::Result<()> {
        self.fixture()?
            .connection(SELECTED)?
            .busy_timeout(Duration::ZERO)?;
        if shared {
            self.fixture()?
                .connection(SIBLING)?
                .busy_timeout(Duration::ZERO)?;
        }
        let mode: String = self.fixture()?.connection(SELECTED)?.query_row(
            "PRAGMA journal_mode=WAL",
            [],
            |row| row.get(0),
        )?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(anyhow!("RegistryLifecycle fixture did not enter WAL mode"));
        }
        self.fixture()?.route(SELECTED)?.into_schema_migration()?;
        self.fixture()?.connection(SELECTED)?.execute_batch(
            "CREATE TABLE registry_lifecycle_probe (
                 probe_id INTEGER PRIMARY KEY,
                 value INTEGER NOT NULL
             );",
        )?;
        self.fixture()?.route(SELECTED)?.into_runtime()?;
        self.fixture()?.connection(SELECTED)?.execute(
            "INSERT INTO registry_lifecycle_probe(probe_id, value) VALUES (1, 829)",
            [],
        )?;
        if shared {
            self.fixture()?.route(SIBLING)?.into_schema_migration()?;
            self.fixture()?.route(SIBLING)?.into_runtime()?;
            self.verify_sibling_sql()?;
        }
        Ok(())
    }

    fn validate_pre_topology(&self, shared: bool) -> anyhow::Result<()> {
        let target = self
            .target_binding()?
            .observer()
            .map_err(anyhow::Error::msg)?
            .snapshot()?;
        let expected = if shared { 2 } else { 1 };
        let routes = self.route_snapshot()?;
        let registration = self.live_registration_snapshot()?;
        let raw = self
            .fixture()?
            .route(SELECTED)?
            .observe_main_raw_slots()
            .map_err(anyhow::Error::msg)?;
        if !target.target_attached
            || target.topology.shm_connections != expected
            || target.shared_mask != 0
            || target.exclusive_mask != 0
            || routes.live_routes != usize::from(expected)
            || routes.logical_names != usize::from(expected) * 3
            || self.fixture()?.registration_id() != 1
            || !registration.registered()
            || !registration.table_present()
            || !registration.name_present()
            || !registration.context_present()
            || !raw.methods_installed
            || !raw.state_installed
        {
            return Err(anyhow!("RegistryLifecycle pre-close topology is not exact"));
        }
        Ok(())
    }

    pub(super) fn target_binding(&self) -> anyhow::Result<ManagedTestShmFaultPlanBinding> {
        self.fixture()?
            .route(SELECTED)?
            .installed_shm_fault_witness()
            .map_err(anyhow::Error::msg)
    }

    pub(super) fn selected_ordinal(&self) -> ManagedTestRouteOrdinal {
        self.selected_ordinal
    }

    pub(super) fn raw_close_snapshot(&self) -> HandleBoundSqliteAbiRawCloseWitnessSnapshot {
        self.raw_close_witness.snapshot()
    }

    pub(super) fn install_lifecycle_fault(
        &self,
        step: ManagedTestLifecycleFaultStep,
    ) -> anyhow::Result<()> {
        self.fixture()?
            .install_lifecycle_fault_script(&[step])
            .map_err(anyhow::Error::msg)
    }

    pub(super) fn install_control(
        &self,
        control: ManagedTestRegistryLifecycleControl,
    ) -> anyhow::Result<()> {
        self.fixture()?
            .install_registry_lifecycle_control(SELECTED, control)
    }

    pub(super) fn retain_outstanding_sidecar(&self) -> anyhow::Result<()> {
        self.fixture()?.retain_outstanding_journal_sidecar(SELECTED)
    }

    pub(super) fn lifecycle_observations(
        &self,
    ) -> anyhow::Result<Vec<ManagedTestLifecycleFaultObservation>> {
        self.fixture()?
            .lifecycle_fault_observations()
            .map_err(anyhow::Error::msg)
    }

    pub(super) fn lifecycle_pending(&self) -> anyhow::Result<usize> {
        self.fixture()?
            .pending_lifecycle_fault_count()
            .map_err(anyhow::Error::msg)
    }

    pub(super) fn registry_trace(
        &self,
    ) -> anyhow::Result<ManagedTestRegistryLifecycleTraceSnapshot> {
        self.selected_observer.trace()
    }

    pub(super) fn terminal_custody(
        &self,
    ) -> anyhow::Result<ManagedSqliteRegistryTerminalCustodyTestSnapshot> {
        self.selected_observer.terminal_custody()
    }

    pub(super) fn close_selected_once(
        &mut self,
    ) -> anyhow::Result<ManagedTestRegistryLifecycleCloseOutcome> {
        self.selected_close_attempts = self
            .selected_close_attempts
            .checked_add(1)
            .context("RegistryLifecycle selected close attempt overflow")?;
        if self.selected_close_attempts != 1 {
            return Err(anyhow!(
                "RegistryLifecycle selected connection was closed more than once"
            ));
        }
        self.fixture_mut()?.close_registry_lifecycle_once(SELECTED)
    }

    pub(super) fn verify_sibling_sql(&self) -> anyhow::Result<()> {
        let value: i64 = self.fixture()?.connection(SIBLING)?.query_row(
            "SELECT value FROM registry_lifecycle_probe WHERE probe_id=1",
            [],
            |row| row.get(0),
        )?;
        if value != PROBE_VALUE {
            return Err(anyhow!(
                "RegistryLifecycle sibling SQL witness changed after selected close"
            ));
        }
        Ok(())
    }

    pub(super) fn sibling_custody(
        &self,
    ) -> anyhow::Result<ManagedSqliteTestVfsRouteCustodySnapshot> {
        self.fixture()?
            .route(SIBLING)?
            .route_custody_snapshot()
            .map_err(anyhow::Error::msg)
    }

    pub(super) fn sibling_is_live(&self) -> bool {
        self.fixture()
            .and_then(|fixture| fixture.connection(SIBLING))
            .is_ok()
    }

    pub(super) fn route_snapshot(&self) -> anyhow::Result<RegistryLifecycleRouteSnapshot> {
        let snapshot = self.fixture()?.registry_lifecycle_route_snapshot()?;
        Ok(RegistryLifecycleRouteSnapshot {
            live_routes: snapshot.live_routes(),
            logical_names: snapshot.logical_names(),
        })
    }

    pub(super) fn sqlite_connection_count(&self) -> anyhow::Result<u8> {
        self.fixture()?.registry_lifecycle_sqlite_connection_count()
    }

    pub(super) fn runtime_snapshot(&self) -> anyhow::Result<ManagedSqliteShmTestTopologySnapshot> {
        self.fixture()?.registry_lifecycle_runtime_snapshot()
    }

    pub(super) fn live_registration_snapshot(
        &self,
    ) -> anyhow::Result<ManagedTestVfsLiveRegistrationSnapshot> {
        self.fixture()?.live_registration_snapshot()
    }

    pub(super) fn selected_close_attempts(&self) -> u8 {
        self.selected_close_attempts
    }

    fn fixture(&self) -> anyhow::Result<&ManagedSqliteMultiConnectionFixture> {
        self.fixture
            .as_ref()
            .context("RegistryLifecycle fixture is not live")
    }

    fn fixture_mut(&mut self) -> anyhow::Result<&mut ManagedSqliteMultiConnectionFixture> {
        self.fixture
            .as_mut()
            .context("RegistryLifecycle fixture is not live")
    }
}

impl Drop for RetainedRegistryLifecycleFixture {
    fn drop(&mut self) {
        if let Some(fixture) = self.fixture.take() {
            // No child teardown may manufacture a second xClose or unregister evidence.
            mem::forget(fixture);
        }
    }
}
