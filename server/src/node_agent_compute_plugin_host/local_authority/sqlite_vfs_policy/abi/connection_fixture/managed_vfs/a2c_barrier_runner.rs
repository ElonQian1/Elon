//! Eight process-isolated Barrier Windows dynamic runners.

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
    a2b2_cases::{validate_barrier_report_payload, BarrierSelector},
};

mod cases;
mod child;

#[test]
fn barrier_admission_rejected() -> anyhow::Result<()> {
    run_isolated_case(
        cases::ADMISSION_REJECTED,
        BarrierSelector::AdmissionRejected,
    )
}

#[test]
fn barrier_wrapper_before() -> anyhow::Result<()> {
    run_isolated_case(cases::WRAPPER_BEFORE, BarrierSelector::WrapperBefore)
}

#[test]
fn barrier_fence_before() -> anyhow::Result<()> {
    run_isolated_case(cases::FENCE_BEFORE, BarrierSelector::FenceBefore)
}

#[test]
fn barrier_fence_after() -> anyhow::Result<()> {
    run_isolated_case(cases::FENCE_AFTER, BarrierSelector::FenceAfter)
}

#[test]
fn barrier_completion_before() -> anyhow::Result<()> {
    run_isolated_case(cases::COMPLETION_BEFORE, BarrierSelector::CompletionBefore)
}

#[test]
fn barrier_completion_native_uncertain() -> anyhow::Result<()> {
    run_isolated_case(
        cases::COMPLETION_NATIVE_UNCERTAIN,
        BarrierSelector::CompletionNativeUncertain,
    )
}

#[test]
fn barrier_completion_after_success_known() -> anyhow::Result<()> {
    run_isolated_case(
        cases::COMPLETION_AFTER_SUCCESS_KNOWN,
        BarrierSelector::CompletionAfterSuccessKnown,
    )
}

#[test]
fn barrier_success() -> anyhow::Result<()> {
    run_isolated_case(cases::SUCCESS, BarrierSelector::Success)
}

fn run_isolated_case(exact_test: &'static str, selector: BarrierSelector) -> anyhow::Result<()> {
    if let Some(root) = child::selected_child_root()? {
        return child::exercise_child(&root, selector);
    }
    run_parent(exact_test, selector)
}

fn run_parent(exact_test: &'static str, selector: BarrierSelector) -> anyhow::Result<()> {
    let executable = std::env::current_exe().context("resolve current Barrier test executable")?;
    let root = create_private_child_root(selector)?;
    let launch = ChildLaunchIdentity::new();
    let spawned = match Command::new(executable)
        .args(["--exact", exact_test, "--nocapture"])
        .env(child::CHILD_ROOT_ENV, &root)
        .env(A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(spawned) => spawned,
        Err(error) => {
            let error = anyhow!(error).context("spawn isolated Barrier child");
            return Err(cleanup_failed_case(&root, error));
        }
    };

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
        let observation = validate_barrier_report_payload(selector, child.actual_payload())
            .map_err(anyhow::Error::msg)?;
        let environment =
            WindowsDynamicEnvironment::capture(&root, &child).map_err(anyhow::Error::msg)?;
        let cleanup = ValidatedParentCleanupReceipt::remove_after_child_exit(&child, &environment)
            .map_err(anyhow::Error::msg)?;
        let record = ValidatedWindowsDynamicRecord::validate_barrier(
            observation,
            environment,
            child,
            cleanup,
        )
        .map_err(anyhow::Error::msg)?;
        let report = record.report();
        if report.case_selector() != selector.report_name() || !report.parent_cleanup_deleted() {
            return Err(anyhow!("Barrier evidence report binding changed"));
        }
        println!("{report}");
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(cleanup_failed_case(&root, error)),
    }
}

fn create_private_child_root(selector: BarrierSelector) -> anyhow::Result<std::path::PathBuf> {
    let requested = std::env::temp_dir().join(format!(
        "elon-a2-barrier-{}-{}-{}",
        selector.report_name(),
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir(&requested).with_context(|| {
        format!(
            "create fresh parent-owned Barrier root at {}",
            requested.display()
        )
    })?;
    match fs::canonicalize(&requested) {
        Ok(root) if root.is_absolute() => Ok(root),
        Ok(_) => {
            let error = anyhow!("canonical Barrier root is not absolute");
            Err(cleanup_failed_case(&requested, error))
        }
        Err(error) => {
            let error = anyhow!(error).context("canonicalize fresh parent-owned Barrier root");
            Err(cleanup_failed_case(&requested, error))
        }
    }
}

fn handle_child_failure(root: &Path, failure: DynamicChildFailure) -> anyhow::Error {
    let exit_confirmed = failure.exit_confirmed();
    let error = anyhow!(failure);
    if exit_confirmed {
        cleanup_failed_case(root, error)
    } else {
        error.context(format!(
            "retained Barrier case root {} because child exit is unconfirmed",
            root.display()
        ))
    }
}

fn cleanup_failed_case(root: &Path, error: anyhow::Error) -> anyhow::Error {
    match fs::remove_dir_all(root) {
        Ok(()) => error,
        Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup_error) => error.context(format!(
            "also failed to remove Barrier case root {}: {cleanup_error}",
            root.display()
        )),
    }
}
