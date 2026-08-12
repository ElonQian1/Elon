//! Windows-only process-isolated VFS unregister fault runners.
//!
//! These tests exercise the real SQLite unregister callback boundary, but do not publish dynamic
//! evidence until they are compiled and run on Windows by the later acceptance batch.

use std::{ffi::CString, fs, path::Path, process::Command, sync::Arc};

use anyhow::Context;
use rusqlite::ffi;

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
    assert!(!unsafe { ffi::sqlite3_vfs_find(vfs_name.as_ptr()) }.is_null());
    let error = registration
        .unregister()
        .expect_err("selected VFS unregister fault must reject shutdown");

    let observations = lifecycle
        .observations()
        .map_err(anyhow::Error::msg)?
        .into_iter()
        .filter(|observation| {
            observation.route.is_none()
                && observation.phase == ManagedTestLifecycleFaultPhase::VfsUnregister
        })
        .collect::<Vec<_>>();
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
            // SAFETY: a before-call fault retains the still-registered table and its name.
            assert!(!unsafe { ffi::sqlite3_vfs_find(vfs_name.as_ptr()) }.is_null());
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
            // SAFETY: SQLite completed unregister before the injected after-success failure.
            assert!(unsafe { ffi::sqlite3_vfs_find(vfs_name.as_ptr()) }.is_null());
        }
    }

    assert!(lifecycle.is_terminal());
    assert_eq!(lifecycle.pending_count().map_err(anyhow::Error::msg)?, 0);
    assert!(routes.upgrade().is_some());
    assert!(runtime.upgrade().is_some());
    assert!(
        root.is_dir(),
        "retained unregister custody keeps the child root"
    );
    Ok(())
}
