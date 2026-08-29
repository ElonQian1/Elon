//! Process-isolated Windows runners for the frozen Unmap selectors.

use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context};

use super::{
    a2_dynamic_evidence::{
        ChildLaunchIdentity, DynamicChildFailure, UnmapFamilyCohort, ValidatedChildProcessReceipt,
        ValidatedParentCleanupReceipt, ValidatedUnmapCandidateRecord,
        ValidatedUnmapFamilyMemberReceipt, WindowsDynamicEnvironment, A2_DYNAMIC_CHILD_NONCE_ENV,
    },
    a2b2_cases::{validate_unmap_report_payload, UnmapSelector, ValidatedUnmapObservation},
};

mod cases;
mod checkout;
mod child;
mod family;

macro_rules! unmap_case {
    ($test:ident, $exact:ident, $selector:ident) => {
        #[test]
        fn $test() -> anyhow::Result<()> {
            run_isolated_case(cases::$exact, UnmapSelector::$selector)
        }
    };
}

unmap_case!(
    unmap_shared_delete_request_validation,
    SHARED_DELETE_REQUEST_VALIDATION,
    SharedDeleteRequestValidation
);
unmap_case!(
    unmap_shared_keep_callback_admission,
    SHARED_KEEP_CALLBACK_ADMISSION,
    SharedKeepCallbackAdmission
);
unmap_case!(
    unmap_shared_keep_callback_wrapper_before,
    SHARED_KEEP_CALLBACK_WRAPPER_BEFORE,
    SharedKeepCallbackWrapperBefore
);
unmap_case!(
    unmap_shared_keep_held_shared_lock,
    SHARED_KEEP_HELD_SHARED_LOCK,
    SharedKeepHeldSharedLock
);
unmap_case!(
    unmap_shared_keep_held_exclusive_lock,
    SHARED_KEEP_HELD_EXCLUSIVE_LOCK,
    SharedKeepHeldExclusiveLock
);
unmap_case!(
    unmap_shared_keep_detach_before,
    SHARED_KEEP_DETACH_BEFORE,
    SharedKeepDetachBefore
);
unmap_case!(
    unmap_shared_keep_detach_after_known,
    SHARED_KEEP_DETACH_AFTER_KNOWN,
    SharedKeepDetachAfterKnown
);
unmap_case!(
    unmap_shared_keep_detach_after_uncertain,
    SHARED_KEEP_DETACH_AFTER_UNCERTAIN,
    SharedKeepDetachAfterUncertain
);
unmap_case!(
    unmap_shared_keep_completion_native_uncertain,
    SHARED_KEEP_COMPLETION_NATIVE_UNCERTAIN,
    SharedKeepCompletionNativeUncertain
);
unmap_case!(
    unmap_shared_keep_success,
    SHARED_KEEP_SUCCESS,
    SharedKeepSuccess
);
unmap_case!(
    unmap_shared_delete_success,
    SHARED_DELETE_SUCCESS,
    SharedDeleteSuccess
);
include!("a2c_unmap_runner/final_tests.rs");

#[test]
fn unmap_windows_dynamic_family_49() -> anyhow::Result<()> {
    family::run()
}

fn run_isolated_case(exact_test: &'static str, selector: UnmapSelector) -> anyhow::Result<()> {
    if let Some(root) = child::selected_child_root()? {
        return child::exercise_child(&root, selector);
    }
    run_parent(exact_test, selector)
}

fn run_parent(exact_test: &'static str, selector: UnmapSelector) -> anyhow::Result<()> {
    let executable = std::env::current_exe().context("resolve current Unmap test executable")?;
    let record = capture_parent_case(
        &executable,
        exact_test,
        selector,
        |observation, environment, child, cleanup| {
            ValidatedUnmapCandidateRecord::validate(observation, environment, child, cleanup)
                .map_err(anyhow::Error::msg)
        },
    )?;
    let report = record.report();
    if report.case_selector() != selector.report_name() || !report.parent_cleanup_deleted() {
        return Err(anyhow!("Unmap evidence report binding changed"));
    }
    checkout::verify_exact_clean_checkout(report.git_sha())?;
    println!(
        "A2_UNMAP_IMPLEMENTATION_CANDIDATE_V1 case={} commit={} target={} child_exit={} parent_cleanup=deleted actual_commitment={}",
        report.case_selector(),
        report.git_sha(),
        report.target(),
        report.child_exit_code(),
        report.actual_payload_commitment(),
    );
    Ok(())
}

pub(super) fn capture_family_member(
    executable: &Path,
    case: cases::ExactUnmapCase,
    cohort: &UnmapFamilyCohort,
) -> anyhow::Result<ValidatedUnmapFamilyMemberReceipt> {
    capture_parent_case(
        executable,
        case.exact_test,
        case.selector,
        |observation, environment, child, cleanup| {
            ValidatedUnmapFamilyMemberReceipt::validate(
                observation,
                environment,
                child,
                cleanup,
                cohort,
            )
            .map_err(anyhow::Error::msg)
        },
    )
}

fn capture_parent_case<T>(
    executable: &Path,
    exact_test: &'static str,
    selector: UnmapSelector,
    validate: impl FnOnce(
        ValidatedUnmapObservation,
        WindowsDynamicEnvironment,
        ValidatedChildProcessReceipt,
        ValidatedParentCleanupReceipt,
    ) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
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
            let error = anyhow!(error).context("spawn isolated Unmap child");
            return Err(cleanup_failed_case(&root, error));
        }
    };

    let bound = match launch.bind(spawned) {
        Ok(bound) => bound,
        Err(error) => return Err(handle_child_failure(&root, error)),
    };
    let child = match bound.wait_for_successful_report() {
        Ok(child) => child,
        Err(error) => return Err(handle_child_failure(&root, error)),
    };
    let result = (|| {
        let observation = validate_unmap_report_payload(selector, child.actual_payload())
            .map_err(anyhow::Error::msg)?;
        let environment =
            WindowsDynamicEnvironment::capture(&root, &child).map_err(anyhow::Error::msg)?;
        let cleanup = ValidatedParentCleanupReceipt::remove_after_child_exit(&child, &environment)
            .map_err(anyhow::Error::msg)?;
        validate(observation, environment, child, cleanup)
    })();
    match result {
        Ok(value) => Ok(value),
        Err(error) => Err(cleanup_failed_case(&root, error)),
    }
}

fn create_private_child_root(selector: UnmapSelector) -> anyhow::Result<std::path::PathBuf> {
    let requested = std::env::temp_dir().join(format!(
        "elon-a2-unmap-{}-{}-{}",
        selector.report_name(),
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir(&requested).with_context(|| {
        format!(
            "create fresh parent-owned Unmap root at {}",
            requested.display()
        )
    })?;
    match fs::canonicalize(&requested) {
        Ok(root) if root.is_absolute() => Ok(root),
        Ok(_) => Err(cleanup_failed_case(
            &requested,
            anyhow!("canonical Unmap root is not absolute"),
        )),
        Err(error) => Err(cleanup_failed_case(
            &requested,
            anyhow!(error).context("canonicalize fresh parent-owned Unmap root"),
        )),
    }
}

fn handle_child_failure(root: &Path, failure: DynamicChildFailure) -> anyhow::Error {
    let exit_confirmed = failure.exit_confirmed();
    let error = anyhow!(failure);
    if exit_confirmed {
        cleanup_failed_case(root, error)
    } else {
        error.context(format!(
            "retained Unmap case root {} because child exit is unconfirmed",
            root.display()
        ))
    }
}

fn cleanup_failed_case(root: &Path, error: anyhow::Error) -> anyhow::Error {
    match fs::remove_dir_all(root) {
        Ok(()) => error,
        Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup_error) => error.context(format!(
            "also failed to remove Unmap case root {}: {cleanup_error}",
            root.display()
        )),
    }
}
