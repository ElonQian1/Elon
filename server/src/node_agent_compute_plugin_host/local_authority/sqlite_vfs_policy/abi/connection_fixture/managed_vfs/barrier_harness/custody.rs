//! Post-call custody and isolation witnesses for one selected Barrier route.

use std::{fs, path::Path};

use anyhow::{anyhow, Context};

use super::{checked_u8, ObservedBarrierOutcome, RetainedBarrierFixture, SELECTED, SIBLING};
use crate::{
    node_agent_compute_plugin_host::local_authority::{
        sqlite_vfs_abi::HandleBoundSqliteAbiRawSlotSnapshot,
        sqlite_vfs_policy::{
            registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot,
            ManagedSqliteLogicalFileRole,
        },
    },
    node_agent_managed_fs::{ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestTargetSnapshot},
};

use super::super::{
    a2b2_cases::{
        BarrierActualCustody, BarrierActualTopology, BarrierDmsCustody, BarrierLogicalRoutePhase,
        BarrierRegistrationPhase, BarrierRegistryRoutePhase,
    },
    ManagedSqliteTestVfsRouteCustodySnapshot, ManagedSqliteTestVfsRoutePhase,
    ManagedTestShmTargetWitness, ManagedTestVfsLiveRegistrationSnapshot,
};

pub(super) struct BarrierPostWitness {
    pub(super) registry_route_phase: BarrierRegistryRoutePhase,
    pub(super) logical_route_phase: BarrierLogicalRoutePhase,
    pub(super) registration_phase: BarrierRegistrationPhase,
    pub(super) later_callback_allowed: bool,
    pub(super) retained: BarrierActualCustody,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn observe_post_witness(
    root: &Path,
    outcome: ObservedBarrierOutcome,
    fixture: &RetainedBarrierFixture,
    selected_target: ManagedTestShmTargetWitness,
    sibling_target: ManagedTestShmTargetWitness,
    pre_sibling_physical: ManagedSqliteShmTestTargetSnapshot,
    post_sibling_physical: ManagedSqliteShmTestTargetSnapshot,
    pre_sibling_raw: HandleBoundSqliteAbiRawSlotSnapshot,
    post_sibling_raw: HandleBoundSqliteAbiRawSlotSnapshot,
    selected_physical: ManagedSqliteShmTestTargetSnapshot,
    terminal: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    registration: ManagedTestVfsLiveRegistrationSnapshot,
    topology: BarrierActualTopology,
) -> anyhow::Result<BarrierPostWitness> {
    require_registration(registration)?;
    validate_sibling(
        fixture,
        selected_target,
        sibling_target,
        pre_sibling_physical,
        post_sibling_physical,
        pre_sibling_raw,
        post_sibling_raw,
        selected_physical,
    )?;
    let logical = fixture.route(SELECTED)?.barrier_logical_route_snapshot()?;
    if logical.live_routes() != 2
        || logical.logical_names() != 6
        || logical.exact_route_names() != 3
        || topology.registry_routes != 2
        || topology.logical_names != 6
    {
        return Err(anyhow!("Barrier retained logical topology is not exact"));
    }

    let live_route = fixture.route(SELECTED)?.route_custody_snapshot().ok();
    let terminal_route = terminal.terminal_route();
    let (
        registry_route_phase,
        logical_route_phase,
        later_callback_allowed,
        main_file,
        main_owner,
        shm_lease,
        callback_leases,
        registry_entry,
    ) = if outcome.is_success() {
        let live_route = live_route.context("successful Barrier lost its exact live route")?;
        require_live_route(live_route, "selected")?;
        if terminal.retention_count() != 0
            || terminal.route_removal_count() != 0
            || terminal.terminal_route_observation_count() != 0
            || terminal.wal_main_physical_custody_retention_count() != 0
            || terminal_route.is_some()
            || !terminal.active_route_present()
        {
            return Err(anyhow!(
                "successful Barrier unexpectedly produced terminal route custody"
            ));
        }
        (
            BarrierRegistryRoutePhase::Active,
            BarrierLogicalRoutePhase::Indexed,
            live_route.access_callback_allowed(),
            live_route.main_file_lock_owner_lease(),
            live_route.main_file_lock_owner_lease(),
            live_route.shm_lease(),
            checked_u8(
                live_route.callbacks_in_flight() as usize,
                "Barrier live callback leases",
            )?,
            true,
        )
    } else {
        if live_route.is_some()
            || terminal.active_route_present()
            || terminal.route_removal_count() != 1
            || terminal.terminal_route_observation_count() != 1
            || terminal.wal_main_physical_custody_retention_count() != 1
        {
            return Err(anyhow!(
                "failed Barrier lacks one exact removed terminal route"
            ));
        }
        let terminal_route = terminal_route.context("failed Barrier lost terminal route state")?;
        if !terminal_route.terminal_reason_is_failure_custody_retained()
            || !terminal_route.connection_owner()
            || !terminal_route.main_file_lock_owner_lease()
            || !terminal_route.shm_lease()
            || terminal_route.callbacks_in_flight() as usize
                != terminal.callback_lease_retention_count()
        {
            return Err(anyhow!(
                "failed Barrier terminal reason or exact route leases changed"
            ));
        }
        (
            BarrierRegistryRoutePhase::TerminalQuarantine,
            BarrierLogicalRoutePhase::Retained,
            false,
            terminal.wal_main_physical_custody_retention_count() == 1,
            terminal_route.main_file_lock_owner_lease(),
            terminal_route.shm_lease(),
            checked_u8(
                terminal_route.callbacks_in_flight() as usize,
                "Barrier terminal callback leases",
            )?,
            true,
        )
    };

    let root_release = observe_root_release(
        root,
        selected_physical,
        main_file,
        main_owner,
        shm_lease,
        registry_entry,
        logical.exact_route_names(),
        registration,
    )?;
    Ok(BarrierPostWitness {
        registry_route_phase,
        logical_route_phase,
        registration_phase: BarrierRegistrationPhase::Registered,
        later_callback_allowed,
        retained: BarrierActualCustody {
            node: selected_physical.topology.node_present,
            views: checked_u8(
                usize::from(selected_physical.topology.views),
                "Barrier retained views",
            )?,
            mappings: checked_u8(
                usize::from(selected_physical.topology.mappings),
                "Barrier retained mappings",
            )?,
            dms: observe_dms(selected_physical.topology.dms)?,
            shm_file: selected_physical.topology.shm_file_present,
            main_file,
            main_lock_owner: main_owner,
            main_lease: main_owner,
            shm_lease,
            callback_leases,
            registry_entry,
            logical_names: checked_u8(
                logical.exact_route_names(),
                "Barrier exact retained logical names",
            )?,
            vfs_table: registration.table_present(),
            vfs_name: registration.name_present(),
            vfs_context: registration.context_present(),
            root_deletable: root_release.is_some(),
        },
    })
}

pub(super) fn require_live_route(
    route: ManagedSqliteTestVfsRouteCustodySnapshot,
    label: &'static str,
) -> anyhow::Result<()> {
    if route.phase() != ManagedSqliteTestVfsRoutePhase::Active
        || !route.connection_owner()
        || !route.main_file_lock_owner_lease()
        || !route.shm_lease()
        || route.callbacks_in_flight() != 0
        || !route.access_callback_allowed()
    {
        return Err(anyhow!(
            "Barrier {label} route custody is not live and idle"
        ));
    }
    Ok(())
}

pub(super) fn require_registration(
    registration: ManagedTestVfsLiveRegistrationSnapshot,
) -> anyhow::Result<()> {
    if !registration.registered()
        || !registration.table_present()
        || !registration.name_present()
        || !registration.context_present()
    {
        return Err(anyhow!("Barrier live VFS registration custody changed"));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_sibling(
    fixture: &RetainedBarrierFixture,
    selected_target: ManagedTestShmTargetWitness,
    sibling_target: ManagedTestShmTargetWitness,
    pre_physical: ManagedSqliteShmTestTargetSnapshot,
    post_physical: ManagedSqliteShmTestTargetSnapshot,
    pre_raw: HandleBoundSqliteAbiRawSlotSnapshot,
    post_raw: HandleBoundSqliteAbiRawSlotSnapshot,
    selected_physical: ManagedSqliteShmTestTargetSnapshot,
) -> anyhow::Result<()> {
    let selected_route = fixture.route_ordinal(SELECTED)?.counter_value();
    let sibling_route = fixture.route_ordinal(SIBLING)?.counter_value();
    require_live_route(
        fixture
            .route(SIBLING)?
            .route_custody_snapshot()
            .map_err(anyhow::Error::msg)?,
        "sibling",
    )?;
    if selected_target.route_ordinal() != selected_route
        || sibling_target.route_ordinal() != sibling_route
        || sibling_route == selected_route
        || sibling_target.registration_id() != selected_target.registration_id()
        || sibling_target.runtime_generation() != selected_target.runtime_generation()
        || sibling_target.shm_connection_id() == selected_target.shm_connection_id()
        || selected_target.role() != ManagedSqliteLogicalFileRole::Main
        || sibling_target.role() != ManagedSqliteLogicalFileRole::Main
    {
        return Err(anyhow!("Barrier sibling exact physical identity changed"));
    }
    if !pre_raw.methods_installed
        || !pre_raw.state_installed
        || !post_raw.methods_installed
        || !post_raw.state_installed
        || !pre_physical.target_attached
        || !post_physical.target_attached
        || pre_physical.shared_mask != 0
        || pre_physical.exclusive_mask != 0
        || post_physical.shared_mask != 0
        || post_physical.exclusive_mask != 0
        || pre_physical.topology.shm_connections != 2
        || post_physical.topology != selected_physical.topology
    {
        return Err(anyhow!(
            "Barrier selected failure escaped into sibling raw or SHM custody"
        ));
    }
    Ok(())
}

struct BarrierRootReleaseWitness;

fn observe_root_release(
    root: &Path,
    physical: ManagedSqliteShmTestTargetSnapshot,
    main_file: bool,
    main_owner: bool,
    shm_lease: bool,
    registry_entry: bool,
    logical_names: usize,
    registration: ManagedTestVfsLiveRegistrationSnapshot,
) -> anyhow::Result<Option<BarrierRootReleaseWitness>> {
    let canonical = fs::canonicalize(root).context("canonicalize live Barrier evidence root")?;
    if canonical != root
        || !root.is_dir()
        || !root.join("db").is_dir()
        || !physical.target_attached
        || !physical.topology.node_present
        || physical.topology.views != 1
        || physical.topology.mappings != 1
        || !physical.topology.shm_file_present
        || !main_file
        || !main_owner
        || !shm_lease
        || !registry_entry
        || logical_names != 3
        || !registration.table_present()
        || !registration.name_present()
        || !registration.context_present()
    {
        return Err(anyhow!(
            "Barrier root retention is not bound to complete live custody"
        ));
    }
    Ok(None)
}

fn observe_dms(value: ManagedSqliteShmTestDmsCustody) -> anyhow::Result<BarrierDmsCustody> {
    match value {
        ManagedSqliteShmTestDmsCustody::Absent => Ok(BarrierDmsCustody::Absent),
        ManagedSqliteShmTestDmsCustody::Shared => Ok(BarrierDmsCustody::Shared),
        ManagedSqliteShmTestDmsCustody::Released => Ok(BarrierDmsCustody::Released),
        ManagedSqliteShmTestDmsCustody::ExclusiveOutcomeUncertain => {
            Ok(BarrierDmsCustody::OutcomeUncertain)
        }
        ManagedSqliteShmTestDmsCustody::ExclusiveKnown => {
            Err(anyhow!("Barrier cannot erase known-exclusive DMS custody"))
        }
    }
}
