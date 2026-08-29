//! Final-route topology, registry and retained-custody observations.

use std::path::Path;

use anyhow::{anyhow, Context};

use super::super::super::{
    a2b2_cases::{
        UnmapActualCustody, UnmapActualTopology, UnmapDmsCustody, UnmapLogicalRoutePhase,
        UnmapRegistrationPhase, UnmapRegistryRoutePhase, UnmapSelector,
    },
    multi_connection::ManagedTestUnmapRouteObservation,
    ManagedSqliteMultiConnectionFixture, ManagedSqliteTestVfsRoutePhase,
};
use super::{outcome, prepare::SELECTED};
use crate::node_agent_managed_fs::ManagedSqliteShmTestDmsCustody;

pub(super) struct FinalPostWitness {
    pub(super) topology: UnmapActualTopology,
    pub(super) registry_route_phase: UnmapRegistryRoutePhase,
    pub(super) logical_route_phase: UnmapLogicalRoutePhase,
    pub(super) registration_phase: UnmapRegistrationPhase,
    pub(super) later_callback_allowed: bool,
    pub(super) retained: UnmapActualCustody,
}

pub(super) fn observe_topology(
    fixture: &ManagedSqliteMultiConnectionFixture,
    selected: ManagedTestUnmapRouteObservation,
    prepared_names: usize,
) -> anyhow::Result<UnmapActualTopology> {
    let logical = fixture.route(SELECTED)?.barrier_logical_route_snapshot()?;
    let live_routes = logical.live_routes();
    let indexed_names = logical.logical_names();
    let retained = selected.terminal_custody.terminal_route().is_some();
    if selected.terminal_custody.terminal_route_observation_count() != usize::from(retained) {
        return Err(anyhow!(
            "final Unmap terminal route observation is not exact"
        ));
    }
    let selected_indexed_names = logical.exact_route_names();
    if selected_indexed_names != 0 && selected_indexed_names != prepared_names {
        return Err(anyhow!("final Unmap logical-name index partially changed"));
    }
    if !retained && selected_indexed_names == 0 {
        return Err(anyhow!(
            "active final Unmap route disappeared from its index"
        ));
    }
    let removed_from_index = retained && selected_indexed_names == 0;
    Ok(UnmapActualTopology {
        sqlite_connections: checked_u8(
            fixture.live_connection_count(),
            "final Unmap SQLite connection count",
        )?,
        shm_connections: selected.physical.topology.shm_connections,
        registry_routes: checked_u8(
            live_routes + usize::from(removed_from_index),
            "final Unmap registry route count",
        )?,
        logical_names: checked_u8(
            indexed_names
                + if removed_from_index {
                    prepared_names
                } else {
                    0
                },
            "final Unmap logical-name count",
        )?,
    })
}

pub(super) fn observe_post_witness(
    root: &Path,
    selector: UnmapSelector,
    fixture: &ManagedSqliteMultiConnectionFixture,
    prepared_names: usize,
    pre: ManagedTestUnmapRouteObservation,
    post: ManagedTestUnmapRouteObservation,
) -> anyhow::Result<FinalPostWitness> {
    let registration = fixture.live_registration_snapshot()?;
    if !registration.registered()
        || !registration.table_present()
        || !registration.name_present()
        || !registration.context_present()
    {
        return Err(anyhow!("final Unmap registration custody changed"));
    }
    if !root.is_dir() || !root.join("db").is_dir() || pre.target != post.target {
        return Err(anyhow!("final Unmap exact target or root custody changed"));
    }
    if !pre.raw.methods_installed
        || !pre.raw.state_installed
        || !post.raw.methods_installed
        || !post.raw.state_installed
        || !pre.physical.target_attached
        || pre.physical.topology.shm_connections != 1
    {
        return Err(anyhow!(
            "final Unmap raw or physical pre-custody is incomplete"
        ));
    }

    let terminal_expected = outcome::route_terminal(selector);
    let terminal = post.terminal_custody;
    let (callback_leases, logical_names, later_callback_allowed, main_owner, shm_lease) =
        if terminal_expected {
            let route = terminal
                .terminal_route()
                .context("terminal final Unmap route lost retained custody")?;
            let expected_other = usize::from(outcome::domain_terminal(selector));
            if terminal.active_route_present()
                || post.active_custody.is_some()
                || terminal.route_removal_count() != 1
                || terminal.terminal_route_observation_count() != 1
                || terminal.wal_main_physical_custody_retention_count() != 0
                || terminal.callback_lease_retention_count() != 1
                || terminal.other_terminal_custody_retention_count() != expected_other
                || terminal.completion_evidence_retention_count() != 0
                || terminal.retention_count() != 1 + expected_other
                || terminal.explicit_failure_custody_retained_count() != 1
                || !route.terminal_reason_is_failure_custody_retained()
                || !route.connection_owner()
                || !route.main_file_lock_owner_lease()
                || !route.shm_lease()
                || route.callbacks_in_flight() != 1
            {
                return Err(anyhow!("terminal final Unmap route custody is not exact"));
            }
            (
                1,
                prepared_names,
                false,
                route.main_file_lock_owner_lease(),
                route.shm_lease(),
            )
        } else {
            if !terminal.active_route_present()
                || terminal.route_removal_count() != 0
                || terminal.retention_count() != 0
                || terminal.terminal_route_observation_count() != 0
            {
                return Err(anyhow!("active final Unmap route entered terminal custody"));
            }
            let route = post
                .active_custody
                .context("active final Unmap route lost live custody")?;
            if route.phase() != ManagedSqliteTestVfsRoutePhase::Active
                || !route.connection_owner()
                || !route.main_file_lock_owner_lease()
                || !route.shm_lease()
                || route.callbacks_in_flight() != 0
                || !route.access_callback_allowed()
            {
                return Err(anyhow!("active final Unmap route is not live and idle"));
            }
            (
                0,
                prepared_names,
                route.access_callback_allowed(),
                route.main_file_lock_owner_lease(),
                route.shm_lease(),
            )
        };
    let indexed_names = fixture
        .route(SELECTED)?
        .barrier_logical_route_snapshot()?
        .exact_route_names();
    if (!terminal_expected && indexed_names != prepared_names)
        || (terminal_expected && indexed_names != 0 && indexed_names != prepared_names)
    {
        return Err(anyhow!("final Unmap logical route custody is inconsistent"));
    }
    let topology = observe_topology(fixture, post, prepared_names)?;
    let physical = post.physical.topology;
    Ok(FinalPostWitness {
        topology,
        registry_route_phase: if terminal_expected {
            UnmapRegistryRoutePhase::TerminalQuarantine
        } else {
            UnmapRegistryRoutePhase::Active
        },
        logical_route_phase: if terminal_expected {
            UnmapLogicalRoutePhase::Retained
        } else {
            UnmapLogicalRoutePhase::Indexed
        },
        registration_phase: UnmapRegistrationPhase::Registered,
        later_callback_allowed,
        retained: UnmapActualCustody {
            node: physical.node_present,
            views: checked_u8(physical.views as usize, "final Unmap retained views")?,
            mappings: checked_u8(physical.mappings as usize, "final Unmap retained mappings")?,
            dms: observe_dms(physical.dms)?,
            shm_file: physical.shm_file_present,
            main_file: main_owner,
            main_lock_owner: main_owner,
            main_lease: main_owner,
            shm_lease,
            callback_leases,
            registry_entry: true,
            logical_names: checked_u8(logical_names, "final Unmap retained logical names")?,
            vfs_table: registration.table_present(),
            vfs_name: registration.name_present(),
            vfs_context: registration.context_present(),
            root_deletable: false,
        },
    })
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
        ManagedSqliteShmTestDmsCustody::ExclusiveKnown => {
            Err(anyhow!("final Unmap retained known-exclusive DMS custody"))
        }
    }
}

fn checked_u8(value: usize, label: &'static str) -> anyhow::Result<u8> {
    u8::try_from(value).with_context(|| format!("{label} exceeds u8"))
}
