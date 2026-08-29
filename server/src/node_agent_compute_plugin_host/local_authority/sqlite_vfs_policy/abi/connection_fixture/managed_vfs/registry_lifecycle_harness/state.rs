//! Exact post-close topology, route phase and retained-custody projection.

use std::{fs, path::Path};

use anyhow::{anyhow, Context};

use super::{
    super::{
        a2b2_cases::{
            RegistryLifecycleActualCustody, RegistryLifecycleActualTopology,
            RegistryLifecycleDmsCustody, RegistryLifecycleLogicalRoutePhase,
            RegistryLifecycleRegistrationPhase, RegistryLifecycleRegistryRoutePhase,
        },
        ManagedSqliteTestVfsRouteCustodySnapshot, ManagedSqliteTestVfsRoutePhase,
        ManagedTestRegistryLifecycleTraceSnapshot, ManagedTestVfsLiveRegistrationSnapshot,
    },
    outcome::ObservedRegistryLifecycleOutcome,
    prepare::RegistryLifecycleRouteSnapshot,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    node_agent_managed_fs::{
        ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestTargetSnapshot,
        ManagedSqliteShmTestTopologySnapshot,
    },
};

pub(super) struct RegistryLifecycleState {
    pub(super) mutation_may_have_occurred: bool,
    pub(super) lock_outcome_uncertain: bool,
    pub(super) domain_terminal: bool,
    pub(super) registry_route_phase: RegistryLifecycleRegistryRoutePhase,
    pub(super) logical_route_phase: RegistryLifecycleLogicalRoutePhase,
    pub(super) registration_phase: RegistryLifecycleRegistrationPhase,
    pub(super) later_callback_allowed: bool,
    pub(super) pre: RegistryLifecycleActualTopology,
    pub(super) post: RegistryLifecycleActualTopology,
    pub(super) retained: RegistryLifecycleActualCustody,
    pub(super) custody_retain: u8,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn observe_state(
    root: &Path,
    outcome: ObservedRegistryLifecycleOutcome,
    pre_target: ManagedSqliteShmTestTargetSnapshot,
    post_target: ManagedSqliteShmTestTargetSnapshot,
    pre_routes: &RegistryLifecycleRouteSnapshot,
    post_routes: &RegistryLifecycleRouteSnapshot,
    pre_sqlite_connections: u8,
    post_sqlite_connections: u8,
    post_runtime: ManagedSqliteShmTestTopologySnapshot,
    terminal: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    registration: ManagedTestVfsLiveRegistrationSnapshot,
    sibling: Option<ManagedSqliteTestVfsRouteCustodySnapshot>,
    trace: &ManagedTestRegistryLifecycleTraceSnapshot,
) -> anyhow::Result<RegistryLifecycleState> {
    validate_registration(root, registration)?;
    validate_physical(outcome, pre_target, post_target, post_runtime)?;
    let expected_pre_routes = if outcome.is_shared() { 2 } else { 1 };
    if pre_routes.live_routes != expected_pre_routes
        || pre_routes.logical_names != expected_pre_routes * 3
        || usize::from(pre_sqlite_connections) != expected_pre_routes
    {
        return Err(anyhow!(
            "RegistryLifecycle pre-close route topology differs from selected family"
        ));
    }
    let expected_logical_routes = if outcome.is_shared() {
        1
    } else if outcome.logical_route_removed() {
        0
    } else {
        1
    };
    if post_routes.live_routes != expected_logical_routes
        || post_routes.logical_names != expected_logical_routes * 3
    {
        return Err(anyhow!(
            "RegistryLifecycle post-close logical index differs from observed removal"
        ));
    }
    validate_selected_terminal(outcome, terminal)?;
    let custody_retain = validate_receipt_custody(outcome, terminal, trace)?;
    validate_sibling(outcome, sibling)?;

    let expected_post_sqlite_connections = u8::from(outcome.is_shared());
    if post_sqlite_connections != expected_post_sqlite_connections {
        return Err(anyhow!(
            "RegistryLifecycle post-close live SQLite connection slots differ from observed topology"
        ));
    }

    let callback_leases = terminal
        .terminal_route()
        .map(|route| route.callbacks_in_flight())
        .unwrap_or(0);
    let registry_routes = if outcome.is_shared() {
        1
    } else {
        u8::from(!outcome.registry_route_removed())
    };
    Ok(RegistryLifecycleState {
        mutation_may_have_occurred: !outcome.is_success(),
        lock_outcome_uncertain: false,
        domain_terminal: false,
        registry_route_phase: if outcome.registry_route_removed() {
            RegistryLifecycleRegistryRoutePhase::Removed
        } else {
            RegistryLifecycleRegistryRoutePhase::TerminalQuarantine
        },
        logical_route_phase: if outcome.logical_route_removed() {
            RegistryLifecycleLogicalRoutePhase::Removed
        } else {
            RegistryLifecycleLogicalRoutePhase::Retained
        },
        registration_phase: RegistryLifecycleRegistrationPhase::Registered,
        later_callback_allowed: false,
        pre: RegistryLifecycleActualTopology {
            sqlite_connections: pre_sqlite_connections,
            shm_connections: pre_target.topology.shm_connections,
            registry_routes: expected_pre_routes as u8,
            logical_names: (expected_pre_routes * 3) as u8,
        },
        post: RegistryLifecycleActualTopology {
            sqlite_connections: post_sqlite_connections,
            shm_connections: post_runtime.shm_connections,
            registry_routes,
            logical_names: u8::try_from(post_routes.logical_names)
                .context("RegistryLifecycle logical-name count exceeds u8")?,
        },
        retained: RegistryLifecycleActualCustody {
            node: false,
            views: 0,
            mappings: 0,
            dms: RegistryLifecycleDmsCustody::Absent,
            shm_file: false,
            main_file: false,
            main_lock_owner: false,
            main_lease: false,
            shm_lease: false,
            callback_leases: u8::try_from(callback_leases)
                .context("RegistryLifecycle callback lease count exceeds u8")?,
            registry_entry: !outcome.registry_route_removed(),
            logical_names: if outcome.logical_route_removed() {
                0
            } else {
                3
            },
            vfs_table: registration.table_present(),
            vfs_name: registration.name_present(),
            vfs_context: registration.context_present(),
            root_deletable: false,
        },
        custody_retain,
    })
}

fn validate_receipt_custody(
    outcome: ObservedRegistryLifecycleOutcome,
    terminal: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    trace: &ManagedTestRegistryLifecycleTraceSnapshot,
) -> anyhow::Result<u8> {
    use ObservedRegistryLifecycleOutcome as O;

    let expected = match outcome {
        O::RegistryRouteRemovalAfterSuccessKnown
        | O::RegistryRouteRemovalPublishNative
        | O::LogicalRouteRemovalBefore
        | O::LogicalRouteRemovalIndexNative => (1, 0, 0),
        O::LogicalRouteRemovalClaimNative => (0, 1, 0),
        O::LogicalRouteRemovalAfterSuccessKnown => (0, 0, 1),
        _ => (0, 0, 0),
    };
    let actual = (
        trace.retained_registry_retirement_count(),
        trace.published_registry_retirement_count(),
        trace.retained_logical_removal_count(),
    );
    let expected_terminal_kinds = match outcome {
        O::CallbackCompletionBefore | O::CallbackCompletionNativeUncertain => (1, 0, 0, 0),
        O::ConnectionObservationOutstandingSidecar => (0, 0, 0, 1),
        O::CallbackCompletionAfterSuccessKnown
        | O::ConnectionObservationBefore
        | O::ConnectionObservationAfterSuccessKnown
        | O::RegistryRouteRemovalBefore
        | O::RegistryRouteRemovalOwnerNative => (0, 1, 0, 0),
        _ => (0, 0, 0, 0),
    };
    let actual_terminal_kinds = (
        terminal.callback_lease_retention_count(),
        terminal.completion_evidence_retention_count(),
        terminal.wal_main_physical_custody_retention_count(),
        terminal.other_terminal_custody_retention_count(),
    );
    let total = terminal
        .retention_count()
        .checked_add(trace.receipt_custody_count())
        .context("RegistryLifecycle custody count overflow")?;
    let expected_total = usize::from(!outcome.is_success());
    let expected_explicit = usize::from(
        !outcome.is_success()
            && !outcome.registry_route_removed()
            && outcome != O::ConnectionObservationOutstandingSidecar,
    );
    if actual != expected
        || actual_terminal_kinds != expected_terminal_kinds
        || terminal.explicit_failure_custody_retained_count() != expected_explicit
        || total != expected_total
    {
        return Err(anyhow!(
            "RegistryLifecycle real terminal/receipt custody is not exactly the observed branch"
        ));
    }
    u8::try_from(total).context("RegistryLifecycle custody count exceeds u8")
}

fn validate_registration(
    root: &Path,
    registration: ManagedTestVfsLiveRegistrationSnapshot,
) -> anyhow::Result<()> {
    let canonical = fs::canonicalize(root).context("canonicalize RegistryLifecycle child root")?;
    if canonical != root
        || !root.is_dir()
        || !registration.registered()
        || !registration.table_present()
        || !registration.name_present()
        || !registration.context_present()
    {
        return Err(anyhow!(
            "RegistryLifecycle actual was not captured before VFS unregister"
        ));
    }
    Ok(())
}

fn validate_physical(
    outcome: ObservedRegistryLifecycleOutcome,
    pre: ManagedSqliteShmTestTargetSnapshot,
    post: ManagedSqliteShmTestTargetSnapshot,
    runtime: ManagedSqliteShmTestTopologySnapshot,
) -> anyhow::Result<()> {
    if !pre.target_attached
        || post.target_attached
        || pre.shared_mask != 0
        || pre.exclusive_mask != 0
        || post.shared_mask != 0
        || post.exclusive_mask != 0
        || post.topology != runtime
    {
        return Err(anyhow!(
            "RegistryLifecycle selected physical target did not detach exactly once"
        ));
    }
    if outcome.is_shared() {
        if runtime.shm_connections != 1
            || !runtime.node_present
            || runtime.views != 1
            || runtime.mappings != 1
            || runtime.dms != ManagedSqliteShmTestDmsCustody::Shared
            || !runtime.shm_file_present
        {
            return Err(anyhow!(
                "RegistryLifecycle shared success damaged its physical sibling"
            ));
        }
    } else if runtime.shm_connections != 0
        || runtime.node_present
        || runtime.views != 0
        || runtime.mappings != 0
        || runtime.dms != ManagedSqliteShmTestDmsCustody::Absent
        || runtime.shm_file_present
    {
        return Err(anyhow!(
            "RegistryLifecycle final selected physical custody was retained"
        ));
    }
    Ok(())
}

fn validate_selected_terminal(
    outcome: ObservedRegistryLifecycleOutcome,
    terminal: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
) -> anyhow::Result<()> {
    if terminal.active_route_present() {
        return Err(anyhow!(
            "RegistryLifecycle selected registry route remained active after close"
        ));
    }
    if !outcome.registry_route_removed() {
        let route = terminal
            .terminal_route()
            .context("RegistryLifecycle route-only failure lacks terminal route witness")?;
        let outstanding_sidecar =
            outcome == ObservedRegistryLifecycleOutcome::ConnectionObservationOutstandingSidecar;
        if terminal.route_removal_count() != 1
            || terminal.terminal_route_observation_count() != 1
            || terminal.retention_count() != 1
            || (outstanding_sidecar
                && (!route.terminal_reason_is_connection_close_unproven()
                    || route.sidecar_lease_count() != 1))
            || (!outstanding_sidecar
                && (!route.terminal_reason_is_failure_custody_retained()
                    || route.sidecar_lease_count() != 0))
            || route.main_file_lock_owner_lease()
            || route.shm_lease()
        {
            return Err(anyhow!(
                "RegistryLifecycle selected terminal registry custody is not exact"
            ));
        }
    } else if terminal.terminal_route().is_some()
        || terminal.terminal_route_observation_count() != 0
        || terminal.retention_count() != 0
        || terminal.route_removal_count() != 0
    {
        return Err(anyhow!(
            "RegistryLifecycle removed route was relabeled terminal quarantine"
        ));
    }
    Ok(())
}

fn validate_sibling(
    outcome: ObservedRegistryLifecycleOutcome,
    sibling: Option<ManagedSqliteTestVfsRouteCustodySnapshot>,
) -> anyhow::Result<()> {
    match (outcome.is_shared(), sibling) {
        (false, None) => Ok(()),
        (true, Some(sibling))
            if sibling.phase() == ManagedSqliteTestVfsRoutePhase::Active
                && sibling.connection_owner()
                && sibling.main_file_lock_owner_lease()
                && sibling.shm_lease()
                && sibling.callbacks_in_flight() == 0
                && sibling.access_callback_allowed() =>
        {
            Ok(())
        }
        _ => Err(anyhow!(
            "RegistryLifecycle shared sibling route custody is missing or changed"
        )),
    }
}
