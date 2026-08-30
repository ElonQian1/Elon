//! Process-isolated Windows runners for the frozen JointClose selectors.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context};

use super::{
    a2_dynamic_evidence::{
        ChildLaunchIdentity, DynamicChildFailure, JointCloseFamilyCohort, SanitizedChildReport,
        ValidatedChildProcessReceipt, ValidatedJointCloseCandidateRecord,
        ValidatedJointCloseCleanCheckoutReceipt, ValidatedJointCloseFamily,
        ValidatedJointCloseFamilyMemberReceipt, ValidatedParentCleanupReceipt,
        WindowsDynamicEnvironment, A2_DYNAMIC_CHILD_NONCE_ENV,
    },
    a2b2_cases::{
        validate_joint_close_report_payload, JointCloseSelector, ValidatedJointCloseObservation,
    },
    a2b2_joint_close_cases as cases, joint_close_harness,
};

const CHILD_ROOT_ENV: &str = "ELON_SQLITE_A2_JOINT_CLOSE_CHILD_ROOT";

macro_rules! joint_close_case {
    ($test:ident, $exact:ident, $selector:ident) => {
        #[test]
        fn $test() -> anyhow::Result<()> {
            run_isolated_case(cases::$exact, JointCloseSelector::$selector)
        }
    };
}

joint_close_case!(
    raw_state_take_rejected,
    RAW_STATE_TAKE_REJECTED,
    RawStateTakeRejected
);
joint_close_case!(
    begin_connection_close_rejected,
    BEGIN_CONNECTION_CLOSE_REJECTED,
    BeginConnectionCloseRejected
);
joint_close_case!(
    callback_admission_rejected,
    CALLBACK_ADMISSION_REJECTED,
    CallbackAdmissionRejected
);
joint_close_case!(
    callback_wrapper_before,
    CALLBACK_WRAPPER_BEFORE,
    CallbackWrapperBefore
);
joint_close_case!(
    shm_view_unmap_before,
    SHM_VIEW_UNMAP_BEFORE,
    ShmViewUnmapBefore
);
joint_close_case!(
    shm_view_unmap_native_uncertain,
    SHM_VIEW_UNMAP_NATIVE_UNCERTAIN,
    ShmViewUnmapNativeUncertain
);
joint_close_case!(
    shm_view_unmap_after_known,
    SHM_VIEW_UNMAP_AFTER_KNOWN,
    ShmViewUnmapAfterKnown
);
joint_close_case!(
    shm_view_unmap_after_uncertain,
    SHM_VIEW_UNMAP_AFTER_UNCERTAIN,
    ShmViewUnmapAfterUncertain
);
joint_close_case!(
    shm_mapping_close_before,
    SHM_MAPPING_CLOSE_BEFORE,
    ShmMappingCloseBefore
);
joint_close_case!(
    shm_mapping_close_native_uncertain,
    SHM_MAPPING_CLOSE_NATIVE_UNCERTAIN,
    ShmMappingCloseNativeUncertain
);
joint_close_case!(
    shm_mapping_close_after_known,
    SHM_MAPPING_CLOSE_AFTER_KNOWN,
    ShmMappingCloseAfterKnown
);
joint_close_case!(
    shm_mapping_close_after_uncertain,
    SHM_MAPPING_CLOSE_AFTER_UNCERTAIN,
    ShmMappingCloseAfterUncertain
);
joint_close_case!(
    shm_dms_release_before,
    SHM_DMS_RELEASE_BEFORE,
    ShmDmsReleaseBefore
);
joint_close_case!(
    shm_dms_release_native_uncertain,
    SHM_DMS_RELEASE_NATIVE_UNCERTAIN,
    ShmDmsReleaseNativeUncertain
);
joint_close_case!(
    shm_dms_release_after_known,
    SHM_DMS_RELEASE_AFTER_KNOWN,
    ShmDmsReleaseAfterKnown
);
joint_close_case!(
    shm_dms_release_after_uncertain,
    SHM_DMS_RELEASE_AFTER_UNCERTAIN,
    ShmDmsReleaseAfterUncertain
);
joint_close_case!(
    shm_file_close_before,
    SHM_FILE_CLOSE_BEFORE,
    ShmFileCloseBefore
);
joint_close_case!(
    shm_file_close_native_retryable,
    SHM_FILE_CLOSE_NATIVE_RETRYABLE,
    ShmFileCloseNativeRetryable
);
joint_close_case!(
    shm_file_close_native_uncertain,
    SHM_FILE_CLOSE_NATIVE_UNCERTAIN,
    ShmFileCloseNativeUncertain
);
joint_close_case!(
    shm_file_close_after_known,
    SHM_FILE_CLOSE_AFTER_KNOWN,
    ShmFileCloseAfterKnown
);
joint_close_case!(
    shm_file_close_after_uncertain,
    SHM_FILE_CLOSE_AFTER_UNCERTAIN,
    ShmFileCloseAfterUncertain
);
joint_close_case!(shm_detach_before, SHM_DETACH_BEFORE, ShmDetachBefore);
joint_close_case!(
    shm_detach_after_known,
    SHM_DETACH_AFTER_KNOWN,
    ShmDetachAfterKnown
);
joint_close_case!(
    shm_detach_after_uncertain,
    SHM_DETACH_AFTER_UNCERTAIN,
    ShmDetachAfterUncertain
);
joint_close_case!(
    main_lock_release_before,
    MAIN_LOCK_RELEASE_BEFORE,
    MainLockReleaseBefore
);
joint_close_case!(
    main_lock_release_native_uncertain_shared,
    MAIN_LOCK_RELEASE_NATIVE_UNCERTAIN_SHARED,
    MainLockReleaseNativeUncertainShared
);
joint_close_case!(
    main_lock_release_native_uncertain_reserved,
    MAIN_LOCK_RELEASE_NATIVE_UNCERTAIN_RESERVED,
    MainLockReleaseNativeUncertainReserved
);
joint_close_case!(
    main_lock_release_after_known,
    MAIN_LOCK_RELEASE_AFTER_KNOWN,
    MainLockReleaseAfterKnown
);
joint_close_case!(
    main_file_close_before,
    MAIN_FILE_CLOSE_BEFORE,
    MainFileCloseBefore
);
joint_close_case!(
    main_file_close_native_retryable,
    MAIN_FILE_CLOSE_NATIVE_RETRYABLE,
    MainFileCloseNativeRetryable
);
joint_close_case!(
    main_file_close_native_uncertain,
    MAIN_FILE_CLOSE_NATIVE_UNCERTAIN,
    MainFileCloseNativeUncertain
);
joint_close_case!(
    main_file_close_after_known,
    MAIN_FILE_CLOSE_AFTER_KNOWN,
    MainFileCloseAfterKnown
);
joint_close_case!(physical_success, PHYSICAL_SUCCESS, PhysicalSuccess);
joint_close_case!(
    registry_wal_main_close_before,
    REGISTRY_WAL_MAIN_CLOSE_BEFORE,
    RegistryWalMainCloseBefore
);
joint_close_case!(
    registry_wal_main_close_native_uncertain,
    REGISTRY_WAL_MAIN_CLOSE_NATIVE_UNCERTAIN,
    RegistryWalMainCloseNativeUncertain
);
joint_close_case!(
    registry_wal_main_close_after_known,
    REGISTRY_WAL_MAIN_CLOSE_AFTER_KNOWN,
    RegistryWalMainCloseAfterKnown
);

#[test]
fn joint_close_windows_dynamic_family_36() -> anyhow::Result<()> {
    run_family()
}

fn run_isolated_case(exact_test: &'static str, selector: JointCloseSelector) -> anyhow::Result<()> {
    if let Some(root) = selected_child_root()? {
        return exercise_child(&root, selector);
    }
    run_parent(exact_test, selector)
}

fn run_parent(exact_test: &'static str, selector: JointCloseSelector) -> anyhow::Result<()> {
    let executable =
        std::env::current_exe().context("resolve current JointClose test executable")?;
    let record = capture_parent_case(
        &executable,
        exact_test,
        selector,
        |observation, environment, child, cleanup| {
            ValidatedJointCloseCandidateRecord::validate(observation, environment, child, cleanup)
                .map_err(anyhow::Error::msg)
        },
    )?;
    let report = record.report();
    if report.case_selector() != selector.report_name() || !report.parent_cleanup_deleted() {
        return Err(anyhow!("JointClose evidence report binding changed"));
    }
    verify_exact_clean_checkout(report.git_sha())?;
    println!(
        "A2_JOINT_CLOSE_IMPLEMENTATION_CANDIDATE_V1 case={} commit={} target={} child_exit={} parent_cleanup=deleted actual_commitment={}",
        report.case_selector(),
        report.git_sha(),
        report.target(),
        report.child_exit_code(),
        report.actual_payload_commitment(),
    );
    Ok(())
}

fn capture_family_member(
    executable: &Path,
    case: cases::ExactJointCloseCase,
    cohort: &JointCloseFamilyCohort,
) -> anyhow::Result<ValidatedJointCloseFamilyMemberReceipt> {
    capture_parent_case(
        executable,
        case.exact_test,
        case.selector,
        |observation, environment, child, cleanup| {
            ValidatedJointCloseFamilyMemberReceipt::validate(
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
    selector: JointCloseSelector,
    validate: impl FnOnce(
        ValidatedJointCloseObservation,
        WindowsDynamicEnvironment,
        ValidatedChildProcessReceipt,
        ValidatedParentCleanupReceipt,
    ) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let root = create_private_child_root()?;
    let launch = ChildLaunchIdentity::new();
    let spawned = match Command::new(executable)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_ROOT_ENV, &root)
        .env(A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(spawned) => spawned,
        Err(error) => {
            let error = anyhow!(error).context("spawn isolated JointClose child");
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
        let observation = validate_joint_close_report_payload(selector, child.actual_payload())
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

fn create_private_child_root() -> anyhow::Result<PathBuf> {
    let requested = std::env::temp_dir().join(format!(
        "elon-a2-joint-close-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir(&requested).with_context(|| {
        format!(
            "create fresh parent-owned JointClose root at {}",
            requested.display()
        )
    })?;
    match fs::canonicalize(&requested) {
        Ok(root) if root.is_absolute() => Ok(root),
        Ok(_) => Err(cleanup_failed_case(
            &requested,
            anyhow!("canonical JointClose root is not absolute"),
        )),
        Err(error) => Err(cleanup_failed_case(
            &requested,
            anyhow!(error).context("canonicalize fresh parent-owned JointClose root"),
        )),
    }
}

fn selected_child_root() -> anyhow::Result<Option<PathBuf>> {
    let Some(root) = std::env::var_os(CHILD_ROOT_ENV).map(PathBuf::from) else {
        return Ok(None);
    };
    if !root.is_absolute() {
        return Err(anyhow!("JointClose child root is not absolute"));
    }
    Ok(Some(root))
}

fn exercise_child(root: &Path, selector: JointCloseSelector) -> anyhow::Result<()> {
    let nonce =
        std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV).context("read parent-created A2 child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;
    let actual = joint_close_harness::exercise_joint_close(root, selector)?;
    let payload = actual.to_report_payload();
    let report = SanitizedChildReport::encode_for_current_child(
        &nonce,
        root,
        actual.identity.target.registration_id,
        &payload,
    )
    .map_err(anyhow::Error::msg)?;
    println!("{report}");
    if !root.is_dir() {
        return Err(anyhow!(
            "JointClose child root disappeared before parent observation"
        ));
    }
    Ok(())
}

fn run_family() -> anyhow::Result<()> {
    reject_ambient_child_environment()?;
    cases::validate_all().map_err(anyhow::Error::msg)?;
    let executable =
        std::env::current_exe().context("resolve JointClose family test executable")?;
    let cohort = JointCloseFamilyCohort::new();
    let mut members = Vec::with_capacity(cases::ALL.len());
    for case in cases::ALL {
        members.push(
            capture_family_member(&executable, case, &cohort).with_context(|| {
                format!(
                    "capture exact JointClose family member {}",
                    case.selector.report_name()
                )
            })?,
        );
    }
    let checkout =
        ValidatedJointCloseCleanCheckoutReceipt::capture(&cohort).map_err(anyhow::Error::msg)?;
    let rendered = ValidatedJointCloseFamily::reduce(cohort, members, checkout)
        .map_err(anyhow::Error::msg)?
        .render_atomic();
    let _clean_commit_fingerprint = rendered.clean_commit_fingerprint();
    println!("{}", rendered.as_str());
    Ok(())
}

fn reject_ambient_child_environment() -> anyhow::Result<()> {
    if std::env::var_os(CHILD_ROOT_ENV).is_some()
        || std::env::var_os(A2_DYNAMIC_CHILD_NONCE_ENV).is_some()
    {
        return Err(anyhow!("A2_JOINT_CLOSE_FAMILY_AMBIENT_CHILD_ENV"));
    }
    Ok(())
}

fn verify_exact_clean_checkout(expected_git_sha: &str) -> anyhow::Result<()> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .ok_or_else(|| anyhow!("resolve JointClose repository root from Cargo manifest"))?;
    let head = git_output(root, &["rev-parse", "HEAD"])?;
    if head.trim() != expected_git_sha {
        return Err(anyhow!("A2_JOINT_CLOSE_CHECKOUT_HEAD_MISMATCH"));
    }
    let status = git_output(root, &["status", "--porcelain=v1", "--untracked-files=all"])?;
    if !status.trim().is_empty() {
        return Err(anyhow!("A2_JOINT_CLOSE_CHECKOUT_NOT_CLEAN"));
    }
    Ok(())
}

fn git_output(root: &Path, arguments: &[&str]) -> anyhow::Result<String> {
    let output = crate::git_command_error::git_command()
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .context("run exact-checkout Git observation")?;
    if !output.status.success() {
        return Err(anyhow!("A2_JOINT_CLOSE_CHECKOUT_GIT_FAILED"));
    }
    String::from_utf8(output.stdout).map_err(|_| anyhow!("A2_JOINT_CLOSE_CHECKOUT_GIT_NON_UTF8"))
}

fn handle_child_failure(root: &Path, failure: DynamicChildFailure) -> anyhow::Error {
    let exit_confirmed = failure.exit_confirmed();
    let error = anyhow!(failure);
    if exit_confirmed {
        cleanup_failed_case(root, error)
    } else {
        error.context(format!(
            "retained JointClose case root {} because child exit is unconfirmed",
            root.display()
        ))
    }
}

fn cleanup_failed_case(root: &Path, error: anyhow::Error) -> anyhow::Error {
    match fs::remove_dir_all(root) {
        Ok(()) => error,
        Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup_error) => error.context(format!(
            "also failed to remove JointClose case root {}: {cleanup_error}",
            root.display()
        )),
    }
}
