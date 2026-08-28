//! Sealed process-local executor for the eight RegistrationShutdown source cases.

use std::{ffi::CString, mem, path::Path, sync::Arc};

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use super::{
    a2b2_cases::{
        RegistrationShutdownActual, RegistrationShutdownActualCustody,
        RegistrationShutdownActualTopology, RegistrationShutdownDmsCustody,
        RegistrationShutdownLogicalRoutePhase, RegistrationShutdownRegistrationPhase,
        RegistrationShutdownRegistryRoutePhase, RegistrationShutdownSelector,
    },
    a2c_vfs_unregister_runner::{
        observe::RegistrationShutdownActions, ObservedRegistrationShutdownOutcome,
    },
    shared_namespace::{
        ManagedTestRegistrationShutdownRouteSnapshot, ManagedTestVfsRouteCollection,
    },
    ManagedSqliteRoutedConnectionFixture, ManagedTestLifecycleFaultPhase,
    ManagedTestLifecycleFaultStep, ManagedTestLifecycleFaultTiming, ManagedTestVfsRegistration,
    ManagedTestVfsRegistrationDisposition, ManagedTestVfsRetainedPartsSnapshot,
    PinnedManagedSqliteWalRuntime, TestRoute,
};
use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestTopologySnapshot,
};

pub(super) fn exercise_registration_shutdown(
    root: &Path,
    selector: RegistrationShutdownSelector,
) -> anyhow::Result<RegistrationShutdownActual> {
    let mut registration = ManagedTestVfsRegistration::register(root, [0xa2; 16])?;
    let registration_id = registration.registration_id().counter_value();
    if registration_id != 1 {
        return Err(anyhow!(
            "RegistrationShutdown exact child did not create registration identity 1"
        ));
    }
    let vfs_name = CString::new(registration.name()?)?;
    let routes = registration.routes();
    let runtime = Arc::clone(
        &registration
            .context
            .as_ref()
            .expect("registered VFS context")
            .runtime,
    );
    let weak_routes = Arc::downgrade(&routes);
    let weak_runtime = Arc::downgrade(&runtime);
    let lifecycle = registration.lifecycle();
    let retained_parts = registration.retained_parts_witness();
    let actions = RegistrationShutdownActions::new(selector);

    let mut connection = if selector_has_live_route(selector) {
        let fixture = ManagedSqliteRoutedConnectionFixture::open_registered(&registration)?;
        prepare_live_wal(&fixture)?;
        Some(fixture)
    } else {
        None
    };
    let pre = observe_topology(&routes, &runtime)?;

    if selector == RegistrationShutdownSelector::OutstandingCallbackGate {
        let route = only_route(&routes)?;
        actions.retain_preexisting_callback(&route)?;
    }
    let quarantine = if selector == RegistrationShutdownSelector::QuarantinedCustodyGate {
        Some(
            lifecycle
                .arm_registration_shutdown_quarantine()
                .map_err(anyhow::Error::msg)?,
        )
    } else {
        None
    };
    install_unregister_fault(selector, &lifecycle)?;
    let lifecycle_baseline = lifecycle.observations().map_err(anyhow::Error::msg)?;

    let lookup_present_before = vfs_is_registered(&vfs_name);
    let shutdown = registration.unregister_in_place_with(
        |route_index| actions.observe_route_index(route_index),
        |table| actions.injected_pre_native_retryable_or_call_sqlite_unregister(table),
    );
    let target = registration.registration_shutdown_target_witness()?;
    let lookup_present_after = vfs_is_registered(&vfs_name);
    let post = observe_topology(&routes, &runtime)?;
    let post_routes = routes.registration_shutdown_snapshot()?;
    let post_runtime = runtime
        .test_topology_snapshot()
        .map_err(|failure| anyhow!("observe registration shutdown runtime: {failure:?}"))?;
    let retained_snapshot = retained_parts.snapshot();

    actions.observe_lifecycle(&lifecycle, &lifecycle_baseline)?;
    if let Some(witness) = quarantine.as_ref() {
        actions.observe_quarantined_custody(witness)?;
    }
    if retained_snapshot.is_some() {
        actions.observe_custody_retained()?;
    }
    let outcome = actions.take_outcome()?;
    verify_selected_outcome(selector, &outcome, &shutdown)?;
    if !lookup_present_before {
        return Err(anyhow!("parent-selected managed VFS was not registered"));
    }

    let registry_route_phase = observe_registry_route_phase(&post_routes)?;
    let later_callback_allowed = post_routes
        .only_route_custody()
        .is_some_and(|custody| custody.access_callback_allowed());
    let logical_route_phase = if post_routes.live_routes() == 0 {
        RegistrationShutdownLogicalRoutePhase::Removed
    } else if retained_snapshot.is_some() {
        RegistrationShutdownLogicalRoutePhase::Retained
    } else {
        RegistrationShutdownLogicalRoutePhase::Indexed
    };
    let registration_phase = observe_registration_phase(lookup_present_after, retained_snapshot)?;
    let retained = observe_custody(&post_routes, post_runtime, retained_snapshot, false)?;

    if shutdown.is_err() {
        if let Some(fixture) = connection.take() {
            mem::forget(fixture);
        }
    }
    drop(routes);
    drop(runtime);
    drop(registration);
    let root_release = observe_root_release(
        &outcome,
        &shutdown,
        lookup_present_after,
        retained_snapshot,
        &post_routes,
        post_runtime,
        weak_routes.upgrade().is_none(),
        weak_runtime.upgrade().is_none(),
    )?;
    let retained = RegistrationShutdownActualCustody {
        root_deletable: root_release.is_some(),
        ..retained
    };
    let observed_selector = outcome.selector();
    let identity = outcome.into_identity(target, retained_snapshot);

    Ok(RegistrationShutdownActual {
        selector: observed_selector,
        identity,
        mutation_may_have_occurred: post_runtime.mutation_may_have_occurred
            || (shutdown.is_err() && actions.native_success() == 1),
        lock_outcome_uncertain: post_runtime.lock_outcome_uncertain,
        domain_terminal: post_runtime.domain_terminal,
        registry_route_phase,
        logical_route_phase,
        registration_phase,
        later_callback_allowed,
        pre,
        post,
        retained,
        counts: actions.snapshot(),
    })
}

fn prepare_live_wal(fixture: &ManagedSqliteRoutedConnectionFixture) -> anyhow::Result<()> {
    let mode: String = fixture
        .connection()
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!(
            "managed registration shutdown fixture did not enter WAL"
        ));
    }
    fixture.into_schema_migration()?;
    fixture.connection().execute_batch(
        "CREATE TABLE registration_shutdown_probe (
             probe_id INTEGER PRIMARY KEY,
             value INTEGER NOT NULL
         );",
    )?;
    fixture.into_runtime()?;
    fixture.connection().execute(
        "INSERT INTO registration_shutdown_probe(probe_id, value) VALUES (1, 280)",
        [],
    )?;
    Ok(())
}

fn install_unregister_fault(
    selector: RegistrationShutdownSelector,
    lifecycle: &Arc<super::ManagedTestLifecycleFaultController>,
) -> anyhow::Result<()> {
    let timing = match selector {
        RegistrationShutdownSelector::VfsUnregisterBeforeCall => {
            Some(ManagedTestLifecycleFaultTiming::BeforeCall)
        }
        RegistrationShutdownSelector::VfsUnregisterAfterSuccessKnown => {
            Some(ManagedTestLifecycleFaultTiming::AfterSuccess)
        }
        _ => None,
    };
    if let Some(timing) = timing {
        let step = ManagedTestLifecycleFaultStep::registration(
            ManagedTestLifecycleFaultPhase::VfsUnregister,
            1,
            timing,
        )
        .map_err(anyhow::Error::msg)?;
        lifecycle.install(&[step]).map_err(anyhow::Error::msg)?;
    }
    Ok(())
}

fn observe_topology(
    routes: &ManagedTestVfsRouteCollection,
    runtime: &PinnedManagedSqliteWalRuntime,
) -> anyhow::Result<RegistrationShutdownActualTopology> {
    let routes = routes.registration_shutdown_snapshot()?;
    let runtime = runtime
        .test_topology_snapshot()
        .map_err(|failure| anyhow!("observe registration shutdown topology: {failure:?}"))?;
    let sqlite_connections = match routes.only_route_custody() {
        Some(custody) if routes.live_routes() == 1 => u8::from(custody.connection_owner()),
        None if routes.live_routes() == 0 => 0,
        _ => return Err(anyhow!("registration route and connection owner disagree")),
    };
    Ok(RegistrationShutdownActualTopology {
        sqlite_connections,
        shm_connections: runtime.shm_connections,
        registry_routes: checked_u8(routes.live_routes(), "registry route count")?,
        logical_names: checked_u8(routes.logical_names(), "logical name count")?,
    })
}

fn observe_custody(
    routes: &ManagedTestRegistrationShutdownRouteSnapshot,
    runtime: ManagedSqliteShmTestTopologySnapshot,
    retained: Option<ManagedTestVfsRetainedPartsSnapshot>,
    root_deletable: bool,
) -> anyhow::Result<RegistrationShutdownActualCustody> {
    let route_custody = match routes.only_route_custody() {
        Some(custody) if routes.live_routes() == 1 => Some(custody),
        None if routes.live_routes() == 0 => None,
        _ => return Err(anyhow!("registration route custody is not singular")),
    };
    let callback_leases = route_custody
        .map(|custody| {
            checked_u8(
                custody.callbacks_in_flight() as usize,
                "callback lease count",
            )
        })
        .transpose()?
        .unwrap_or(0);
    let main_owner = route_custody.is_some_and(|custody| custody.main_file_lock_owner_lease());
    Ok(RegistrationShutdownActualCustody {
        node: runtime.node_present,
        views: checked_u8(usize::from(runtime.views), "retained view count")?,
        mappings: checked_u8(usize::from(runtime.mappings), "retained mapping count")?,
        dms: observe_dms(runtime.dms)?,
        shm_file: runtime.shm_file_present,
        main_file: main_owner,
        main_lock_owner: main_owner,
        main_lease: main_owner,
        shm_lease: route_custody.is_some_and(|custody| custody.shm_lease()),
        callback_leases,
        registry_entry: route_custody.is_some(),
        logical_names: checked_u8(routes.logical_names(), "retained logical name count")?,
        vfs_table: retained.is_some_and(|snapshot| snapshot.table_present()),
        vfs_name: retained.is_some_and(|snapshot| snapshot.name_present()),
        vfs_context: retained.is_some_and(|snapshot| snapshot.context_present()),
        root_deletable,
    })
}

fn observe_registry_route_phase(
    routes: &ManagedTestRegistrationShutdownRouteSnapshot,
) -> anyhow::Result<RegistrationShutdownRegistryRoutePhase> {
    match (routes.live_routes(), routes.only_route_custody()) {
        (0, None) => Ok(RegistrationShutdownRegistryRoutePhase::Removed),
        (1, Some(custody)) => match custody.phase() {
            super::ManagedSqliteTestVfsRoutePhase::Active => {
                Ok(RegistrationShutdownRegistryRoutePhase::Active)
            }
            super::ManagedSqliteTestVfsRoutePhase::Closing => {
                Ok(RegistrationShutdownRegistryRoutePhase::Closing)
            }
            super::ManagedSqliteTestVfsRoutePhase::AwaitingRouteRetirement => {
                Ok(RegistrationShutdownRegistryRoutePhase::AwaitingRetirement)
            }
            super::ManagedSqliteTestVfsRoutePhase::TerminalQuarantine => {
                Ok(RegistrationShutdownRegistryRoutePhase::TerminalQuarantine)
            }
            super::ManagedSqliteTestVfsRoutePhase::PendingMain
            | super::ManagedSqliteTestVfsRoutePhase::Opening
            | super::ManagedSqliteTestVfsRoutePhase::Retired => Err(anyhow!(
                "registration route session phase is not reportable at shutdown"
            )),
        },
        _ => Err(anyhow!("registration shutdown route index is not singular")),
    }
}

fn observe_registration_phase(
    lookup_present: bool,
    retained: Option<ManagedTestVfsRetainedPartsSnapshot>,
) -> anyhow::Result<RegistrationShutdownRegistrationPhase> {
    match (
        lookup_present,
        retained.map(ManagedTestVfsRetainedPartsSnapshot::disposition),
    ) {
        (true, Some(ManagedTestVfsRegistrationDisposition::Registered)) => {
            Ok(RegistrationShutdownRegistrationPhase::RetainedRegistered)
        }
        (false, Some(ManagedTestVfsRegistrationDisposition::Unregistered)) => {
            Ok(RegistrationShutdownRegistrationPhase::RetainedAfterUnregister)
        }
        (false, None) => Ok(RegistrationShutdownRegistrationPhase::Unregistered),
        (true, None) => Ok(RegistrationShutdownRegistrationPhase::Registered),
        _ => Err(anyhow!("VFS lookup and retained disposition disagree")),
    }
}

fn observe_dms(
    dms: ManagedSqliteShmTestDmsCustody,
) -> anyhow::Result<RegistrationShutdownDmsCustody> {
    match dms {
        ManagedSqliteShmTestDmsCustody::Absent => Ok(RegistrationShutdownDmsCustody::Absent),
        ManagedSqliteShmTestDmsCustody::Shared => Ok(RegistrationShutdownDmsCustody::Shared),
        ManagedSqliteShmTestDmsCustody::Released => Ok(RegistrationShutdownDmsCustody::Released),
        ManagedSqliteShmTestDmsCustody::ExclusiveOutcomeUncertain => {
            Ok(RegistrationShutdownDmsCustody::OutcomeUncertain)
        }
        ManagedSqliteShmTestDmsCustody::ExclusiveKnown => Err(anyhow!(
            "RegistrationShutdown model cannot erase known-exclusive DMS custody"
        )),
    }
}

fn selector_has_live_route(selector: RegistrationShutdownSelector) -> bool {
    matches!(
        selector,
        RegistrationShutdownSelector::OutstandingCallbackGate
            | RegistrationShutdownSelector::LiveRouteGate
            | RegistrationShutdownSelector::QuarantinedCustodyGate
            | RegistrationShutdownSelector::RouteIndexObservation
    )
}

fn only_route(routes: &ManagedTestVfsRouteCollection) -> anyhow::Result<Arc<TestRoute>> {
    routes
        .registration_shutdown_snapshot()?
        .only_route()
        .cloned()
        .context("registration shutdown live route witness")
}

fn vfs_is_registered(name: &CString) -> bool {
    // SAFETY: `name` is a live NUL-terminated VFS name and SQLite permits read-only lookup.
    !unsafe { ffi::sqlite3_vfs_find(name.as_ptr()) }.is_null()
}

fn verify_selected_outcome(
    selector: RegistrationShutdownSelector,
    outcome: &ObservedRegistrationShutdownOutcome,
    shutdown: &anyhow::Result<()>,
) -> anyhow::Result<()> {
    if outcome.selector() != selector || outcome.is_success() != shutdown.is_ok() {
        return Err(anyhow!(
            "parent-selected RegistrationShutdown case differs from sealed observed outcome"
        ));
    }
    Ok(())
}

struct ManagedTestRegistrationShutdownRootReleaseWitness;

#[allow(clippy::too_many_arguments)]
fn observe_root_release(
    outcome: &ObservedRegistrationShutdownOutcome,
    shutdown: &anyhow::Result<()>,
    lookup_present_after: bool,
    retained: Option<ManagedTestVfsRetainedPartsSnapshot>,
    routes: &ManagedTestRegistrationShutdownRouteSnapshot,
    runtime: ManagedSqliteShmTestTopologySnapshot,
    route_owner_released: bool,
    runtime_owner_released: bool,
) -> anyhow::Result<Option<ManagedTestRegistrationShutdownRootReleaseWitness>> {
    if !outcome.is_success() {
        if shutdown.is_ok() {
            return Err(anyhow!(
                "failed registration shutdown released its root owner"
            ));
        }
        return Ok(None);
    }
    if shutdown.is_err()
        || lookup_present_after
        || retained.is_some()
        || routes.live_routes() != 0
        || routes.logical_names() != 0
        || routes.only_route().is_some()
        || routes.only_route_custody().is_some()
        || runtime.shm_connections != 0
        || runtime.node_present
        || runtime.views != 0
        || runtime.mappings != 0
        || runtime.dms != ManagedSqliteShmTestDmsCustody::Absent
        || runtime.shm_file_present
        || !route_owner_released
        || !runtime_owner_released
    {
        return Err(anyhow!(
            "successful registration shutdown retained root-bound ownership"
        ));
    }
    Ok(Some(ManagedTestRegistrationShutdownRootReleaseWitness))
}

fn checked_u8(value: usize, label: &'static str) -> anyhow::Result<u8> {
    u8::try_from(value).with_context(|| format!("{label} exceeds u8"))
}
