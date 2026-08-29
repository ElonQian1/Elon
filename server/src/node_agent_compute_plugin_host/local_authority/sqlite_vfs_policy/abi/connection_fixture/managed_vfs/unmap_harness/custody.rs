//! Post-call sibling, route, SHM, registration, and retained-custody witnesses.

use std::path::Path;

use anyhow::{anyhow, Context};

use super::super::{
    a2b2_cases::{
        UnmapActualCustody, UnmapActualTopology, UnmapDmsCustody, UnmapLogicalRoutePhase,
        UnmapRegistrationPhase, UnmapRegistryRoutePhase,
    },
    multi_connection::ManagedTestUnmapRouteObservation,
    ManagedSqliteTestVfsRoutePhase, ManagedTestVfsLiveRegistrationSnapshot,
};
use super::{checked_u8, outcome::ObservedSharedUnmapOutcome, prepare::RetainedUnmapFixture};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole,
    node_agent_managed_fs::ManagedSqliteShmTestDmsCustody,
};

pub(super) struct UnmapPostWitness {
    pub(super) registry_route_phase: UnmapRegistryRoutePhase,
    pub(super) logical_route_phase: UnmapLogicalRoutePhase,
    pub(super) registration_phase: UnmapRegistrationPhase,
    pub(super) later_callback_allowed: bool,
    pub(super) retained: UnmapActualCustody,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn observe_post_witness(
    root: &Path,
    outcome: ObservedSharedUnmapOutcome,
    fixture: &RetainedUnmapFixture,
    prepared_selected_names: usize,
    pre_selected: ManagedTestUnmapRouteObservation,
    post_selected: ManagedTestUnmapRouteObservation,
    pre_sibling: ManagedTestUnmapRouteObservation,
    post_sibling: ManagedTestUnmapRouteObservation,
    registration: ManagedTestVfsLiveRegistrationSnapshot,
    topology: UnmapActualTopology,
) -> anyhow::Result<UnmapPostWitness> {
    require_registration(registration)?;
    validate_sibling(
        fixture,
        pre_selected,
        post_selected,
        pre_sibling,
        post_sibling,
    )?;
    if topology.registry_routes == 0 || !root.is_dir() || !root.join("db").is_dir() {
        return Err(anyhow!("Unmap retained logical/root custody is incomplete"));
    }

    let terminal = post_selected.terminal_custody;
    let (
        registry_route_phase,
        logical_route_phase,
        later_callback_allowed,
        main_file,
        main_owner,
        shm_lease,
        callback_leases,
        logical_names,
    ) = if outcome.route_terminal() {
        let route = terminal
            .terminal_route()
            .context("terminal Unmap route lost its redacted custody snapshot")?;
        if !route.terminal_reason_is_failure_custody_retained()
            || !route.connection_owner()
            || !route.main_file_lock_owner_lease()
            || !route.shm_lease()
            || route.callbacks_in_flight() as usize != terminal.callback_lease_retention_count()
        {
            return Err(anyhow!("terminal Unmap route leases are inconsistent"));
        }
        (
            UnmapRegistryRoutePhase::TerminalQuarantine,
            UnmapLogicalRoutePhase::Retained,
            false,
            route.main_file_lock_owner_lease(),
            route.main_file_lock_owner_lease(),
            route.shm_lease(),
            checked_u8(
                route.callbacks_in_flight() as usize,
                "Unmap terminal callback leases",
            )?,
            checked_u8(prepared_selected_names, "Unmap terminal logical names")?,
        )
    } else {
        let route = post_selected
            .active_custody
            .context("active Unmap outcome lost its route custody")?;
        require_live_route(route, "selected")?;
        (
            UnmapRegistryRoutePhase::Active,
            UnmapLogicalRoutePhase::Indexed,
            route.access_callback_allowed(),
            route.main_file_lock_owner_lease(),
            route.main_file_lock_owner_lease(),
            route.shm_lease(),
            checked_u8(
                route.callbacks_in_flight() as usize,
                "Unmap live callback leases",
            )?,
            checked_u8(prepared_selected_names, "Unmap live logical names")?,
        )
    };
    if logical_names == 0 {
        return Err(anyhow!("Unmap selected route lost all logical names"));
    }
    let post_indexed_names = fixture
        .route(super::prepare::SELECTED)?
        .barrier_logical_route_snapshot()?
        .exact_route_names();
    if (!outcome.route_terminal() && post_indexed_names != prepared_selected_names)
        || (outcome.route_terminal()
            && post_indexed_names != 0
            && post_indexed_names != prepared_selected_names)
    {
        return Err(anyhow!(
            "Unmap selected logical-name custody disagrees with its pre-stimulus binding"
        ));
    }
    let physical = post_selected.physical.topology;
    Ok(UnmapPostWitness {
        registry_route_phase,
        logical_route_phase,
        registration_phase: UnmapRegistrationPhase::Registered,
        later_callback_allowed,
        retained: UnmapActualCustody {
            node: physical.node_present,
            views: checked_u8(physical.views as usize, "Unmap retained views")?,
            mappings: checked_u8(physical.mappings as usize, "Unmap retained mappings")?,
            dms: observe_dms(physical.dms)?,
            shm_file: physical.shm_file_present,
            main_file,
            main_lock_owner: main_owner,
            main_lease: main_owner,
            shm_lease,
            callback_leases,
            registry_entry: true,
            logical_names,
            vfs_table: registration.table_present(),
            vfs_name: registration.name_present(),
            vfs_context: registration.context_present(),
            root_deletable: false,
        },
    })
}

fn validate_sibling(
    fixture: &RetainedUnmapFixture,
    pre_selected: ManagedTestUnmapRouteObservation,
    post_selected: ManagedTestUnmapRouteObservation,
    pre_sibling: ManagedTestUnmapRouteObservation,
    post_sibling: ManagedTestUnmapRouteObservation,
) -> anyhow::Result<()> {
    require_live_route(
        post_sibling
            .active_custody
            .context("Unmap sibling route left active custody")?,
        "sibling",
    )?;
    if pre_selected.target.registration_id() != pre_sibling.target.registration_id()
        || pre_selected.target.runtime_generation() != pre_sibling.target.runtime_generation()
        || pre_selected.target.route_ordinal() == pre_sibling.target.route_ordinal()
        || pre_selected.target.shm_connection_id() == pre_sibling.target.shm_connection_id()
        || pre_selected.target.role() != ManagedSqliteLogicalFileRole::Main
        || pre_sibling.target.role() != ManagedSqliteLogicalFileRole::Main
        || pre_selected.target != post_selected.target
        || pre_sibling.target != post_sibling.target
        || fixture
            .route_ordinal(super::prepare::SELECTED)?
            .counter_value()
            != pre_selected.target.route_ordinal()
        || fixture
            .route_ordinal(super::prepare::SIBLING)?
            .counter_value()
            != pre_sibling.target.route_ordinal()
    {
        return Err(anyhow!("Unmap exact selected/sibling identity changed"));
    }
    if !pre_sibling.raw.methods_installed
        || !pre_sibling.raw.state_installed
        || !post_sibling.raw.methods_installed
        || !post_sibling.raw.state_installed
        || !pre_sibling.physical.target_attached
        || !post_sibling.physical.target_attached
        || pre_sibling.physical.topology.shm_connections != 2
        || post_sibling.physical.topology != post_selected.physical.topology
        || pre_sibling.physical.shared_mask != 0
        || pre_sibling.physical.exclusive_mask != 0
        || post_sibling.physical.shared_mask != 0
        || post_sibling.physical.exclusive_mask != 0
    {
        return Err(anyhow!(
            "Unmap selected transition escaped into sibling custody"
        ));
    }
    Ok(())
}

fn require_live_route(
    route: super::super::ManagedSqliteTestVfsRouteCustodySnapshot,
    label: &'static str,
) -> anyhow::Result<()> {
    if route.phase() != ManagedSqliteTestVfsRoutePhase::Active
        || !route.connection_owner()
        || !route.main_file_lock_owner_lease()
        || !route.shm_lease()
        || route.callbacks_in_flight() != 0
        || !route.access_callback_allowed()
    {
        return Err(anyhow!("Unmap {label} route is not live and idle"));
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
        return Err(anyhow!("Unmap live VFS registration custody changed"));
    }
    Ok(())
}

fn observe_dms(value: ManagedSqliteShmTestDmsCustody) -> anyhow::Result<UnmapDmsCustody> {
    match value {
        ManagedSqliteShmTestDmsCustody::Absent => Ok(UnmapDmsCustody::Absent),
        ManagedSqliteShmTestDmsCustody::Shared => Ok(UnmapDmsCustody::Shared),
        ManagedSqliteShmTestDmsCustody::Released => Ok(UnmapDmsCustody::Released),
        ManagedSqliteShmTestDmsCustody::SharedOutcomeUncertain
        | ManagedSqliteShmTestDmsCustody::ExclusiveOutcomeUncertain => {
            Ok(UnmapDmsCustody::OutcomeUncertain)
        }
        ManagedSqliteShmTestDmsCustody::ExclusiveKnown => Err(anyhow!(
            "SharedNonFinal Unmap retained known-exclusive DMS custody"
        )),
    }
}
