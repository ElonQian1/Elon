//! One live final-connection WAL fixture whose allocation is retained after real xClose.

use std::{mem::ManuallyDrop, os::raw::c_int, path::Path, time::Duration};

use anyhow::{anyhow, Context};

use super::super::{
    a2b2_cases::{JointCloseActualTopology, JointCloseSelector},
    connection::{ManagedTestCapturedMainCloseCall, ManagedTestRegistryLifecycleRouteObserver},
    ManagedSqliteMultiConnectionFixture, ManagedTestCallbackFaultObservation,
    ManagedTestLifecycleFaultBinding, ManagedTestLifecycleFaultObservation,
    ManagedTestRouteOrdinal, ManagedTestShmFaultPlanBinding, ManagedTestShmTargetWitness,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::{
        HandleBoundSqliteAbiRawCloseWitness, HandleBoundSqliteAbiRawCloseWitnessSnapshot,
    },
    node_agent_managed_fs::ManagedSqliteShmTestTargetSnapshot,
};

pub(super) const SELECTED: usize = 0;
const PROBE_VALUE: i64 = 2_051;

pub(super) struct JointCloseFixture {
    // This is the actual owner of the rusqlite Connection and its sqlite3_file allocation. The
    // child never drops it after direct xClose; a bare pointer is never the lifetime authority.
    pub(super) fixture: ManuallyDrop<ManagedSqliteMultiConnectionFixture>,
    close: ManagedTestCapturedMainCloseCall,
    invocation: InvocationState,
    pub(super) binding: ManagedTestShmFaultPlanBinding,
    pub(super) lifecycle: ManagedTestLifecycleFaultBinding,
    pub(super) route_observer: ManagedTestRegistryLifecycleRouteObserver,
    pub(super) route: ManagedTestRouteOrdinal,
    pub(super) target: ManagedTestShmTargetWitness,
    pub(super) pre_physical: ManagedSqliteShmTestTargetSnapshot,
    pub(super) raw_witness: HandleBoundSqliteAbiRawCloseWitness,
    pub(super) callback_baseline: Vec<ManagedTestCallbackFaultObservation>,
    pub(super) lifecycle_baseline: Vec<ManagedTestLifecycleFaultObservation>,
    pub(super) prepared_names: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InvocationState {
    Fresh,
    InvokedOnce,
    InvokedTwice,
}

impl JointCloseFixture {
    pub(super) fn prepare(root: &Path, selector: JointCloseSelector) -> anyhow::Result<Self> {
        let fixture = ManagedSqliteMultiConnectionFixture::open_single(root, [0xd7; 16])?;
        fixture.connection(SELECTED)?.busy_timeout(Duration::ZERO)?;
        let mode: String =
            fixture
                .connection(SELECTED)?
                .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(anyhow!("JointClose fixture did not enter WAL mode"));
        }
        fixture.route(SELECTED)?.into_schema_migration()?;
        fixture.connection(SELECTED)?.execute_batch(
            "CREATE TABLE joint_close_probe (
                 probe_id INTEGER PRIMARY KEY,
                 value INTEGER NOT NULL
             );",
        )?;
        fixture.route(SELECTED)?.into_runtime()?;
        fixture.connection(SELECTED)?.execute(
            "INSERT INTO joint_close_probe(probe_id, value) VALUES (1, ?1)",
            [PROBE_VALUE],
        )?;

        let route = fixture.route_ordinal(SELECTED)?;
        let selected = fixture.route(SELECTED)?;
        let close = selected
            .capture_main_close_call()
            .map_err(anyhow::Error::msg)?;
        if selector == JointCloseSelector::MainLockReleaseNativeUncertainShared {
            // SAFETY: `fixture` still uniquely owns this live allocation and no xClose has run.
            unsafe { close.acquire_main_lock_prestate(false) }.map_err(anyhow::Error::msg)?;
        } else if selector == JointCloseSelector::MainLockReleaseNativeUncertainReserved {
            // SAFETY: `fixture` still uniquely owns this live allocation and no xClose has run.
            unsafe { close.acquire_main_lock_prestate(true) }.map_err(anyhow::Error::msg)?;
        }
        // SAFETY: the same still-live fixture owns and serializes the installed allocation.
        let raw_witness = unsafe { close.raw_close_witness() }.map_err(anyhow::Error::msg)?;
        validate_pristine_raw(raw_witness.snapshot())?;
        let binding = selected
            .installed_shm_fault_witness()
            .map_err(anyhow::Error::msg)?;
        let target = binding.target_witness().map_err(anyhow::Error::msg)?;
        let pre_physical = binding
            .observer()
            .map_err(anyhow::Error::msg)?
            .snapshot()
            .context("observe JointClose exact physical prestate")?;
        let lifecycle = selected.registry_lifecycle_binding()?;
        let route_observer = fixture.registry_lifecycle_route_observer(SELECTED)?;
        let callback_baseline = fixture
            .callback_fault_observations()
            .map_err(anyhow::Error::msg)?;
        let lifecycle_baseline = fixture
            .lifecycle_fault_observations()
            .map_err(anyhow::Error::msg)?;
        if !fixture.unmap_runtime_trace(SELECTED)?.is_empty() {
            return Err(anyhow!(
                "JointClose outer runtime observation was not pristine"
            ));
        }
        fixture.enable_unmap_runtime_observation(SELECTED)?;
        let prepared_names = selected
            .barrier_logical_route_snapshot()?
            .exact_route_names();
        validate_pre(&fixture, root, target, pre_physical, prepared_names)?;

        Ok(Self {
            fixture: ManuallyDrop::new(fixture),
            close,
            invocation: InvocationState::Fresh,
            binding,
            lifecycle,
            route_observer,
            route,
            target,
            pre_physical,
            raw_witness,
            callback_baseline,
            lifecycle_baseline,
            prepared_names,
        })
    }

    pub(super) fn owner(&self) -> &ManagedSqliteMultiConnectionFixture {
        &self.fixture
    }

    pub(super) fn arm_raw_state_take_rejection(&self) -> anyhow::Result<()> {
        if self.invocation != InvocationState::Fresh {
            return Err(anyhow!("JointClose raw-state control armed after xClose"));
        }
        // SAFETY: `self.fixture` is the typed owner, is intentionally ManuallyDrop-retained, and
        // this state machine proves no xClose invocation has occurred.
        unsafe { self.close.arm_raw_state_take_rejection() }
            .map(drop)
            .map_err(anyhow::Error::msg)
    }

    pub(super) fn invoke_first(&mut self) -> anyhow::Result<c_int> {
        if self.invocation != InvocationState::Fresh {
            return Err(anyhow!("JointClose first xClose invocation is not fresh"));
        }
        // SAFETY: the ManuallyDrop fixture remains the allocation owner, and Fresh proves this is
        // the first serialized use of the saved callback.
        let code = unsafe { self.close.invoke() };
        self.invocation = InvocationState::InvokedOnce;
        Ok(code)
    }

    pub(super) fn invoke_second(&mut self) -> anyhow::Result<c_int> {
        if self.invocation != InvocationState::InvokedOnce {
            return Err(anyhow!(
                "JointClose second xClose invocation is not exact-once"
            ));
        }
        // SAFETY: every selector reaches this single serialized retry; the typed fixture still
        // owns either the preserved or cleared allocation, and InvokedOnce bounds it to one call.
        let code = unsafe { self.close.invoke() };
        self.invocation = InvocationState::InvokedTwice;
        Ok(code)
    }

    pub(super) fn observe_raw_slots(
        &self,
    ) -> anyhow::Result<crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::HandleBoundSqliteAbiRawSlotSnapshot>
    {
        if self.invocation == InvocationState::Fresh {
            return Err(anyhow!("JointClose raw slots observed before xClose"));
        }
        // SAFETY: the ManuallyDrop fixture continues to own the exact allocation after its raw
        // slots clear, and the invocation state excludes concurrent callback use.
        unsafe { self.close.observe_raw_slots() }.map_err(anyhow::Error::msg)
    }

    pub(super) fn pre_topology(&self) -> JointCloseActualTopology {
        JointCloseActualTopology {
            sqlite_connections: 1,
            shm_connections: 1,
            registry_routes: 1,
            logical_names: 3,
        }
    }
}

fn validate_pristine_raw(raw: HandleBoundSqliteAbiRawCloseWitnessSnapshot) -> anyhow::Result<()> {
    if raw.raw_close_entries != 0
        || raw.state_take_attempts != 0
        || raw.methods_clears != 0
        || raw.state_take_successes != 0
        || raw.state_close_custody_retentions != 0
        || raw.state_close_attempts != 0
        || raw.state_abandons != 0
    {
        return Err(anyhow!("JointClose raw-close witness was not pristine"));
    }
    Ok(())
}

fn validate_pre(
    fixture: &ManagedSqliteMultiConnectionFixture,
    root: &Path,
    target: ManagedTestShmTargetWitness,
    physical: ManagedSqliteShmTestTargetSnapshot,
    prepared_names: usize,
) -> anyhow::Result<()> {
    let registration = fixture.live_registration_snapshot()?;
    let route = fixture
        .route(SELECTED)?
        .route_custody_snapshot()
        .map_err(anyhow::Error::msg)?;
    let (routes, names) = fixture.logical_route_counts()?;
    if !root.is_absolute()
        || !root.is_dir()
        || !root.join("db").is_dir()
        || fixture.live_connection_count() != 1
        || routes != 1
        || names != 3
        || prepared_names != 3
        || fixture.registration_id() != 1
        || target.registration_id() != 1
        || target.route_ordinal() != 1
        || target.runtime_generation() != 1
        || target.shm_connection_id() != 1
        || !physical.target_attached
        || physical.shared_mask != 0
        || physical.exclusive_mask != 0
        || physical.topology.shm_connections != 1
        || !physical.topology.node_present
        || physical.topology.views != 1
        || physical.topology.mappings != 1
        || !physical.topology.shm_file_present
        || route.phase() != super::super::ManagedSqliteTestVfsRoutePhase::Active
        || !route.connection_owner()
        || !route.main_file_lock_owner_lease()
        || !route.shm_lease()
        || route.callbacks_in_flight() != 0
        || !route.access_callback_allowed()
        || !registration.registered()
        || !registration.table_present()
        || !registration.name_present()
        || !registration.context_present()
    {
        return Err(anyhow!("JointClose final-WAL precondition is not exact"));
    }
    Ok(())
}
