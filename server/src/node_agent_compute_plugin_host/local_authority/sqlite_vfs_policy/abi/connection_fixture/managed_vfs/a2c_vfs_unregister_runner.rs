//! Windows-only process-isolated VFS unregister fault runners.
//!
//! These tests exercise the real SQLite unregister callback boundary, but do not publish dynamic
//! evidence until they are compiled and run on Windows by the later acceptance batch.

use std::{ffi::CString, fs, path::Path, process::Command, sync::Arc};

use anyhow::Context;
use rusqlite::ffi;

use super::a2b2_cases::{
    validate_dynamic_registration, DynamicRegistrationActual,
    DynamicRegistrationRetainedDisposition, DynamicRegistrationTiming,
};
use super::*;

const CHILD_ROOT_ENV: &str = "ELON_SQLITE_A2C_VFS_UNREGISTER_CHILD_ROOT";
const BEFORE_EXACT_TEST: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_vfs_unregister_runner::vfs_unregister_before_call_retains_registered_custody";
const AFTER_EXACT_TEST: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_vfs_unregister_runner::vfs_unregister_after_success_retains_unregistered_custody";

#[test]
fn vfs_unregister_before_call_retains_registered_custody() -> anyhow::Result<()> {
    run_isolated_case(BEFORE_EXACT_TEST, BeforeOrAfter::BeforeCall)
}

#[test]
fn vfs_unregister_after_success_retains_unregistered_custody() -> anyhow::Result<()> {
    run_isolated_case(AFTER_EXACT_TEST, BeforeOrAfter::AfterSuccess)
}

#[derive(Clone, Copy)]
enum BeforeOrAfter {
    BeforeCall,
    AfterSuccess,
}

#[derive(Clone, Copy)]
struct RegistrationOnlyTopology {
    sqlite_connections: u8,
    shm_connections: u8,
    registry_routes: u8,
    logical_names: u8,
}

fn run_isolated_case(exact_test: &str, timing: BeforeOrAfter) -> anyhow::Result<()> {
    if let Some(root) = std::env::var_os(CHILD_ROOT_ENV).map(std::path::PathBuf::from) {
        exercise_unregister_fault(&root, timing)?;
        return Ok(());
    }

    let root = std::env::temp_dir().join(format!(
        "elon-managed-vfs-a2c-unregister-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let status = Command::new(std::env::current_exe().context("resolve current test executable")?)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_ROOT_ENV, &root)
        .status()
        .context("run isolated VFS unregister fault child")?;
    let cleanup = fs::remove_dir_all(&root).with_context(|| {
        format!(
            "remove VFS unregister child namespace after process exit at {}",
            root.display()
        )
    });
    assert!(status.success(), "VFS unregister fault child must pass");
    cleanup?;
    Ok(())
}

fn exercise_unregister_fault(root: &Path, timing: BeforeOrAfter) -> anyhow::Result<()> {
    let registration = ManagedTestVfsRegistration::register(root, [0xac; 16])?;
    assert_eq!(registration.live_route_count()?, 0);

    let context = registration
        .context
        .as_ref()
        .expect("registered VFS context");
    let routes = Arc::downgrade(&context.routes);
    let runtime = Arc::downgrade(&context.runtime);
    let pre_topology = registration_only_topology(&context.routes)?;
    let retained_parts = registration.retained_parts_witness();
    assert_eq!(retained_parts.snapshot(), None);
    let vfs_name = CString::new(registration.name()?)?;
    let lifecycle = registration.lifecycle();
    let selected_timing = match timing {
        BeforeOrAfter::BeforeCall => ManagedTestLifecycleFaultTiming::BeforeCall,
        BeforeOrAfter::AfterSuccess => ManagedTestLifecycleFaultTiming::AfterSuccess,
    };
    let step = ManagedTestLifecycleFaultStep::registration(
        ManagedTestLifecycleFaultPhase::VfsUnregister,
        1,
        selected_timing,
    )
    .map_err(anyhow::Error::msg)?;
    lifecycle.install(&[step]).map_err(anyhow::Error::msg)?;

    // SAFETY: `vfs_name` is a live CString and SQLite permits lookup by registered VFS name.
    let lookup_present_before = !unsafe { ffi::sqlite3_vfs_find(vfs_name.as_ptr()) }.is_null();
    assert!(lookup_present_before);
    let error = registration
        .unregister()
        .expect_err("selected VFS unregister fault must reject shutdown");

    let all_observations = lifecycle.observations().map_err(anyhow::Error::msg)?;
    let total_observation_count = all_observations.len();
    let observations = all_observations
        .into_iter()
        .filter(|observation| {
            observation.route.is_none()
                && observation.phase == ManagedTestLifecycleFaultPhase::VfsUnregister
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observations.len(),
        total_observation_count,
        "isolated unregister runner must not hide unrelated lifecycle observations"
    );
    // SAFETY: `vfs_name` remains a live CString throughout the isolated child case.
    let lookup_present_after = !unsafe { ffi::sqlite3_vfs_find(vfs_name.as_ptr()) }.is_null();
    let before = ManagedTestLifecycleFaultObservation {
        route: None,
        phase: ManagedTestLifecycleFaultPhase::VfsUnregister,
        occurrence: 1,
        timing: ManagedTestLifecycleFaultTiming::BeforeCall,
        triggered: matches!(timing, BeforeOrAfter::BeforeCall),
    };

    match timing {
        BeforeOrAfter::BeforeCall => {
            assert_eq!(
                error.to_string(),
                "injected before managed test VFS unregister"
            );
            assert_eq!(observations, vec![before]);
            // A before-call fault retains the still-registered table and its name.
            assert!(lookup_present_after);
        }
        BeforeOrAfter::AfterSuccess => {
            assert_eq!(
                error.to_string(),
                "injected after managed test VFS unregister"
            );
            let after = ManagedTestLifecycleFaultObservation {
                route: None,
                phase: ManagedTestLifecycleFaultPhase::VfsUnregister,
                occurrence: 1,
                timing: ManagedTestLifecycleFaultTiming::AfterSuccess,
                triggered: true,
            };
            assert_eq!(observations, vec![before, after]);
            // SQLite completed unregister before the injected after-success failure.
            assert!(!lookup_present_after);
        }
    }

    let lifecycle_terminal = lifecycle.is_terminal();
    let lifecycle_pending = checked_u8(
        lifecycle.pending_count().map_err(anyhow::Error::msg)?,
        "lifecycle pending count",
    )?;
    let retained_routes = routes.upgrade();
    let retained_runtime = runtime.upgrade();
    let post_topology = registration_only_topology(
        retained_routes
            .as_deref()
            .context("retained VFS route collection witness")?,
    )?;
    let retained_snapshot = retained_parts
        .snapshot()
        .context("retained VFS parts witness snapshot")?;
    let retained_disposition = match retained_snapshot.disposition {
        ManagedTestVfsRegistrationDisposition::Registered => {
            DynamicRegistrationRetainedDisposition::Registered
        }
        ManagedTestVfsRegistrationDisposition::Unregistered => {
            DynamicRegistrationRetainedDisposition::Unregistered
        }
    };
    let root_exists_after = root.is_dir();
    let custody_retained = retained_routes.is_some()
        && retained_runtime.is_some()
        && retained_parts.snapshot() == Some(retained_snapshot);
    let actual = DynamicRegistrationActual {
        timing: match timing {
            BeforeOrAfter::BeforeCall => DynamicRegistrationTiming::BeforeCall,
            BeforeOrAfter::AfterSuccess => DynamicRegistrationTiming::AfterSuccessKnown,
        },
        pre_sqlite_connections: pre_topology.sqlite_connections,
        pre_shm_connections: pre_topology.shm_connections,
        pre_registry_routes: pre_topology.registry_routes,
        pre_logical_names: pre_topology.logical_names,
        post_sqlite_connections: post_topology.sqlite_connections,
        post_shm_connections: post_topology.shm_connections,
        post_registry_routes: post_topology.registry_routes,
        post_logical_names: post_topology.logical_names,
        lookup_present_before,
        lookup_present_after,
        before_call_observations: observation_count(
            &observations,
            ManagedTestLifecycleFaultTiming::BeforeCall,
            None,
        )?,
        before_call_triggers: observation_count(
            &observations,
            ManagedTestLifecycleFaultTiming::BeforeCall,
            Some(true),
        )?,
        after_success_observations: observation_count(
            &observations,
            ManagedTestLifecycleFaultTiming::AfterSuccess,
            None,
        )?,
        after_success_triggers: observation_count(
            &observations,
            ManagedTestLifecycleFaultTiming::AfterSuccess,
            Some(true),
        )?,
        lifecycle_pending,
        lifecycle_terminal,
        retained_routes: post_topology.registry_routes,
        retained_logical_names: post_topology.logical_names,
        retained_vfs_table: retained_snapshot.table_present,
        retained_vfs_name: retained_snapshot.name_present,
        retained_vfs_context: retained_snapshot.context_present,
        retained_disposition,
        custody_retained,
        root_present_after_failure: root_exists_after,
    };
    validate_dynamic_registration(actual).map_err(anyhow::Error::msg)?;

    assert!(lifecycle_terminal);
    assert_eq!(lifecycle_pending, 0);
    assert!(retained_runtime.is_some());
    assert!(
        root_exists_after,
        "retained unregister custody keeps the child root"
    );
    Ok(())
}

fn registration_only_topology(
    routes: &ManagedTestVfsRouteCollection,
) -> anyhow::Result<RegistrationOnlyTopology> {
    // This runner calls registration directly and never constructs a rusqlite connection or asks
    // the VFS for shared memory. Those two zeros are construction facts; live_route_count also
    // locks and verifies that the exact-name index contains three names per registry route.
    let registry_routes = checked_u8(routes.live_route_count()?, "live registry route count")?;
    let logical_names = registry_routes
        .checked_mul(3)
        .context("live logical-name count overflow")?;
    Ok(RegistrationOnlyTopology {
        sqlite_connections: 0,
        shm_connections: 0,
        registry_routes,
        logical_names,
    })
}

fn observation_count(
    observations: &[ManagedTestLifecycleFaultObservation],
    timing: ManagedTestLifecycleFaultTiming,
    triggered: Option<bool>,
) -> anyhow::Result<u8> {
    checked_u8(
        observations
            .iter()
            .filter(|observation| {
                observation.timing == timing
                    && triggered.map_or(true, |expected| observation.triggered == expected)
            })
            .count(),
        "lifecycle observation count",
    )
}

fn checked_u8(value: usize, label: &'static str) -> anyhow::Result<u8> {
    u8::try_from(value).with_context(|| format!("{label} exceeds u8"))
}
