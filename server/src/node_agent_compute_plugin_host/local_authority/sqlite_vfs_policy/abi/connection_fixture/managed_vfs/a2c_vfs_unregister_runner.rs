//! Eight process-isolated RegistrationShutdown Windows dynamic runners.

use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context};

use super::{
    a2_dynamic_evidence::{
        ChildLaunchIdentity, DynamicChildFailure, ValidatedParentCleanupReceipt,
        ValidatedWindowsDynamicRecord, WindowsDynamicEnvironment, A2_DYNAMIC_CHILD_NONCE_ENV,
    },
    a2b2_cases::{validate_registration_shutdown_report_payload, RegistrationShutdownSelector},
};

mod cases;
mod child;
pub(super) mod observe;
mod outcome;

pub(super) use outcome::ObservedRegistrationShutdownOutcome;

#[test]
fn registration_shutdown_outstanding_callback_gate() -> anyhow::Result<()> {
    run_isolated_case(
        cases::OUTSTANDING_CALLBACK,
        RegistrationShutdownSelector::OutstandingCallbackGate,
    )
}

#[test]
fn registration_shutdown_live_route_gate() -> anyhow::Result<()> {
    run_isolated_case(
        cases::LIVE_ROUTE,
        RegistrationShutdownSelector::LiveRouteGate,
    )
}

#[test]
fn registration_shutdown_quarantined_custody_gate() -> anyhow::Result<()> {
    run_isolated_case(
        cases::QUARANTINED_CUSTODY,
        RegistrationShutdownSelector::QuarantinedCustodyGate,
    )
}

#[test]
fn registration_shutdown_route_index_observation() -> anyhow::Result<()> {
    run_isolated_case(
        cases::ROUTE_INDEX_OBSERVATION,
        RegistrationShutdownSelector::RouteIndexObservation,
    )
}

#[test]
fn registration_shutdown_vfs_unregister_before_call() -> anyhow::Result<()> {
    run_isolated_case(
        cases::VFS_UNREGISTER_BEFORE,
        RegistrationShutdownSelector::VfsUnregisterBeforeCall,
    )
}

#[test]
fn registration_shutdown_vfs_unregister_injected_pre_native_retryable_observation(
) -> anyhow::Result<()> {
    run_isolated_case(
        cases::VFS_UNREGISTER_INJECTED_PRE_NATIVE_RETRYABLE,
        RegistrationShutdownSelector::VfsUnregisterNativeRetryable,
    )
}

#[test]
fn registration_shutdown_vfs_unregister_after_success_known() -> anyhow::Result<()> {
    run_isolated_case(
        cases::VFS_UNREGISTER_AFTER,
        RegistrationShutdownSelector::VfsUnregisterAfterSuccessKnown,
    )
}

#[test]
fn registration_shutdown_success() -> anyhow::Result<()> {
    run_isolated_case(cases::SUCCESS, RegistrationShutdownSelector::Success)
}

fn run_isolated_case(
    exact_test: &'static str,
    selector: RegistrationShutdownSelector,
) -> anyhow::Result<()> {
    if let Some(root) = child::selected_child_root()? {
        return child::exercise_child(&root, selector);
    }
    run_parent(exact_test, selector)
}

fn run_parent(
    exact_test: &'static str,
    selector: RegistrationShutdownSelector,
) -> anyhow::Result<()> {
    let root = private_child_root(selector);
    let launch = ChildLaunchIdentity::new();
    let spawned = Command::new(
        std::env::current_exe().context("resolve current registration shutdown test executable")?,
    )
    .args(["--exact", exact_test, "--nocapture"])
    .env(child::CHILD_ROOT_ENV, &root)
    .env(A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .context("spawn isolated registration shutdown child")?;

    let bound = match launch.bind(spawned) {
        Ok(bound) => bound,
        Err(error) => {
            return Err(handle_child_failure(&root, error));
        }
    };
    let child = match bound.wait_for_successful_report() {
        Ok(child) => child,
        Err(error) => return Err(handle_child_failure(&root, error)),
    };
    let result = (|| {
        let observation =
            validate_registration_shutdown_report_payload(selector, child.actual_payload())
                .map_err(anyhow::Error::msg)?;
        let environment =
            WindowsDynamicEnvironment::capture(&root, &child).map_err(anyhow::Error::msg)?;
        let cleanup = ValidatedParentCleanupReceipt::remove_after_child_exit(&child, &environment)
            .map_err(anyhow::Error::msg)?;
        let record =
            ValidatedWindowsDynamicRecord::validate(observation, environment, child, cleanup)
                .map_err(anyhow::Error::msg)?;
        let report = record.report();
        if report.case_selector() != selector.report_name() || !report.parent_cleanup_deleted() {
            return Err(anyhow!(
                "registration shutdown evidence report binding changed"
            ));
        }
        println!("{report}");
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(cleanup_failed_case(&root, error)),
    }
}

fn private_child_root(selector: RegistrationShutdownSelector) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "elon-a2-registration-shutdown-{}-{}-{}",
        selector.report_name(),
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ))
}

fn handle_child_failure(root: &Path, failure: DynamicChildFailure) -> anyhow::Error {
    let exit_confirmed = failure.exit_confirmed();
    let error = anyhow!(failure);
    if exit_confirmed {
        cleanup_failed_case(root, error)
    } else {
        error.context(format!(
            "retained registration shutdown case root {} because child exit is unconfirmed",
            root.display()
        ))
    }
}

fn cleanup_failed_case(root: &Path, error: anyhow::Error) -> anyhow::Error {
    match fs::remove_dir_all(root) {
        Ok(()) => error,
        Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup_error) => error.context(format!(
            "also failed to remove registration shutdown case root {}: {cleanup_error}",
            root.display()
        )),
    }
}
