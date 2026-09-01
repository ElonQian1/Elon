//! Process-isolated native receipts for executable Lock request-validation programs.

mod abi_scalar_rejection;
mod callback_completion_route_unknown;
mod created_first_exclusive_release_error;
mod created_first_shared_busy_close_succeeded;
mod created_first_truncate_error_release_failed;
mod created_first_truncate_error_release_succeeded;
mod existing_first_exclusive_release_error;
mod existing_first_truncate_error_release_failed;
mod existing_first_truncate_error_release_succeeded;
mod lifecycle;
mod local_protocol_rejection;
mod local_sibling_contention;
mod native_acquire_busy;
mod pre_managed_rejection;
mod raw_state_rejection;
mod request_validation;
#[cfg(all(test, windows))]
mod selector_test_support;
mod stored_poison;
mod stored_poison_dispatch;
mod stored_poison_model;
mod stored_poison_route_unknown;
pub(in super::super) use abi_scalar_rejection::*;
#[cfg(all(test, windows))]
pub(in super::super) use callback_completion_route_unknown::{
    lock_callback_route_unknown_selector_for_test,
    selected_lock_callback_route_unknown_selector_for_test,
};
pub(in super::super) use callback_completion_route_unknown::{
    run_lock_callback_route_unknown_program_isolated, LockRunnerCallbackRouteUnknownBindingV1,
    LockRunnerCallbackRouteUnknownPathV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use created_first_exclusive_release_error::{
    lock_native_acquire_created_first_exclusive_release_error_selector_for_test,
    selected_lock_native_acquire_created_first_exclusive_release_error_selector_for_test,
};
pub(in super::super) use created_first_exclusive_release_error::{
    run_lock_native_acquire_created_first_exclusive_release_error_program_isolated,
    LockRunnerCreatedFirstExclusiveReleaseCompletionV1,
    LockRunnerNativeAcquireCreatedFirstExclusiveReleaseErrorBindingV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use created_first_shared_busy_close_succeeded::{
    lock_native_acquire_created_first_shared_busy_close_succeeded_selector_for_test,
    selected_lock_native_acquire_created_first_shared_busy_close_succeeded_selector_for_test,
};
pub(in super::super) use created_first_shared_busy_close_succeeded::{
    run_lock_native_acquire_created_first_shared_busy_close_succeeded_program_isolated,
    LockRunnerCreatedFirstSharedBusyCloseSucceededCompletionV1,
    LockRunnerNativeAcquireCreatedFirstSharedBusyCloseSucceededBindingV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use created_first_truncate_error_release_failed::{
    lock_native_acquire_created_first_truncate_error_release_failed_selector_for_test,
    selected_lock_native_acquire_created_first_truncate_error_release_failed_selector_for_test,
};
pub(in super::super) use created_first_truncate_error_release_failed::{
    run_lock_native_acquire_created_first_truncate_error_release_failed_program_isolated,
    LockRunnerCreatedFirstTruncateErrorReleaseFailedCompletionV1,
    LockRunnerNativeAcquireCreatedFirstTruncateErrorReleaseFailedBindingV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use created_first_truncate_error_release_succeeded::{
    lock_native_acquire_created_first_truncate_error_release_succeeded_selector_for_test,
    selected_lock_native_acquire_created_first_truncate_error_release_succeeded_selector_for_test,
};
pub(in super::super) use created_first_truncate_error_release_succeeded::{
    run_lock_native_acquire_created_first_truncate_error_release_succeeded_program_isolated,
    LockRunnerCreatedFirstTruncateErrorReleaseSucceededCompletionV1,
    LockRunnerNativeAcquireCreatedFirstTruncateErrorReleaseSucceededBindingV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use existing_first_exclusive_release_error::{
    lock_native_acquire_existing_first_exclusive_release_error_selector_for_test,
    selected_lock_native_acquire_existing_first_exclusive_release_error_selector_for_test,
};
pub(in super::super) use existing_first_exclusive_release_error::{
    run_lock_native_acquire_existing_first_exclusive_release_error_program_isolated,
    LockRunnerExistingFirstExclusiveReleaseCompletionV1,
    LockRunnerNativeAcquireExistingFirstExclusiveReleaseErrorBindingV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use existing_first_truncate_error_release_failed::{
    lock_native_acquire_existing_first_truncate_error_release_failed_selector_for_test,
    selected_lock_native_acquire_existing_first_truncate_error_release_failed_selector_for_test,
};
pub(in super::super) use existing_first_truncate_error_release_failed::{
    run_lock_native_acquire_existing_first_truncate_error_release_failed_program_isolated,
    LockRunnerExistingFirstTruncateErrorReleaseFailedCompletionV1,
    LockRunnerNativeAcquireExistingFirstTruncateErrorReleaseFailedBindingV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use existing_first_truncate_error_release_succeeded::{
    lock_native_acquire_existing_first_truncate_error_release_succeeded_selector_for_test,
    selected_lock_native_acquire_existing_first_truncate_error_release_succeeded_selector_for_test,
};
pub(in super::super) use existing_first_truncate_error_release_succeeded::{
    run_lock_native_acquire_existing_first_truncate_error_release_succeeded_program_isolated,
    LockRunnerExistingFirstTruncateErrorReleaseSucceededCompletionV1,
    LockRunnerNativeAcquireExistingFirstTruncateErrorReleaseSucceededBindingV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use local_protocol_rejection::{
    lock_local_protocol_rejection_selector_for_test,
    selected_lock_local_protocol_rejection_selector_for_test,
};
pub(in super::super) use local_protocol_rejection::{
    run_lock_local_protocol_rejection_program_isolated, LocalProtocolRejectionPathV1,
    LockRunnerLocalProtocolRejectionBindingV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use local_sibling_contention::{
    lock_local_sibling_contention_selector_for_test,
    selected_lock_local_sibling_contention_selector_for_test,
};
pub(in super::super) use local_sibling_contention::{
    run_lock_local_sibling_contention_program_isolated, LockRunnerLocalSiblingContentionBindingV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use native_acquire_busy::{
    lock_native_acquire_busy_selector_for_test, selected_lock_native_acquire_busy_selector_for_test,
};
pub(in super::super) use native_acquire_busy::{
    run_lock_native_acquire_busy_program_isolated, LockRunnerNativeAcquireBusyBindingV1,
};
#[cfg(all(test, windows))]
pub(in super::super) use pre_managed_rejection::{
    lock_pre_managed_rejection_selector_for_test,
    selected_lock_pre_managed_rejection_selector_for_test,
};
pub(in super::super) use pre_managed_rejection::{
    run_lock_pre_managed_rejection_program_isolated, LockRunnerPreManagedCompletionV1,
    LockRunnerPreManagedRejectionBindingV1, LockRunnerPreManagedRejectionV1,
};
pub(in super::super) use raw_state_rejection::*;
pub(in super::super) use request_validation::{LockRunnerActionV1, LockRunnerRequestValidationV1};
#[cfg(all(test, windows))]
pub(in super::super) use selector_test_support::{
    lock_stored_poison_selector_for_test, selected_lock_stored_poison_selector_for_test,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
pub(in super::super) use stored_poison_model::{
    LockRunnerStoredPoisonBindingV1, LockRunnerStoredPoisonCompletionV1,
    LockRunnerStoredPoisonProfileV1,
};

use anyhow::{anyhow, Context};
use sha2::{Digest, Sha256};

use super::child::SanitizedPayloadFamily;
use super::{
    ChildLaunchIdentity, DynamicChildFailure, ValidatedChildProcessReceipt,
    ValidatedParentCleanupReceipt, WindowsDynamicEnvironment,
};

const CHILD_ROOT_ENV: &str = "ELON_SQLITE_A2_LOCK_QUOTIENT_CHILD_ROOT";
pub(super) const STORED_POISON_SELECTOR_ENV: &str = "ELON_SQLITE_A2_LOCK_STORED_POISON_SELECTOR";
pub(super) const PAYLOAD_VERSION: &str = "a2lockq1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct LockRunnerProgramBindingV1 {
    pub(in super::super) action: LockRunnerActionV1,
    pub(in super::super) request_validation: LockRunnerRequestValidationV1,
    pub(in super::super) normalized_descriptor_sha256: [u8; 32],
    pub(in super::super) case_key_sha256: [u8; 32],
    pub(in super::super) full_record_sha256: [u8; 32],
    pub(in super::super) plan_sha256: [u8; 32],
    pub(in super::super) implementation_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum LockRunnerLifecyclePathV1 {
    NativeAcquire,
    NativeRelease,
    SharedLocalAcquire,
    SharedLocalRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct LockRunnerLifecycleBindingV1 {
    pub(in super::super) path: LockRunnerLifecyclePathV1,
    pub(in super::super) action: LockRunnerActionV1,
    pub(in super::super) first: u8,
    pub(in super::super) count: u8,
    pub(in super::super) mask: u8,
    pub(in super::super) normalized_descriptor_sha256: [u8; 32],
    pub(in super::super) case_key_sha256: [u8; 32],
    pub(in super::super) full_record_sha256: [u8; 32],
    pub(in super::super) plan_sha256: [u8; 32],
    pub(in super::super) implementation_sha256: [u8; 32],
}

pub(in super::super) enum LockRunnerIsolatedEvidenceV1 {
    ParentReceipt(LockRunnerEvidenceReceiptV1),
    ChildReported,
}

/// Opaque components produced only after exact-child exit, root rebinding and parent deletion.
pub(in super::super) struct LockRunnerEvidenceReceiptV1 {
    root_commitment_sha256: [u8; 32],
    child_fingerprint_sha256: [u8; 32],
    registration_commitment_sha256: [u8; 32],
    payload_commitment_sha256: [u8; 32],
    environment_sha256: [u8; 32],
    cleanup_sha256: [u8; 32],
    native_receipt_sha256: [u8; 32],
    child_exit_code: i32,
}

impl LockRunnerEvidenceReceiptV1 {
    pub(in super::super) fn into_bindings(
        self,
    ) -> (
        [u8; 32],
        [u8; 32],
        [u8; 32],
        [u8; 32],
        [u8; 32],
        [u8; 32],
        [u8; 32],
        i32,
    ) {
        (
            self.root_commitment_sha256,
            self.child_fingerprint_sha256,
            self.registration_commitment_sha256,
            self.payload_commitment_sha256,
            self.environment_sha256,
            self.cleanup_sha256,
            self.native_receipt_sha256,
            self.child_exit_code,
        )
    }
}

pub(in super::super) fn run_lock_program_isolated(
    exact_test: &str,
    binding: LockRunnerProgramBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    request_validation::validate_binding(binding)?;
    if let Some(root) = selected_child_root()? {
        request_validation::exercise_child(&root, binding)?;
        return Ok(LockRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_parent(exact_test, binding)
}

pub(in super::super) fn run_lock_lifecycle_program_isolated(
    exact_test: &str,
    binding: LockRunnerLifecycleBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    lifecycle::validate_binding(binding)?;
    if let Some(root) = selected_child_root()? {
        lifecycle::exercise_child(&root, binding)?;
        return Ok(LockRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_lifecycle_parent(exact_test, binding)
}

pub(in super::super) fn run_lock_stored_poison_program_isolated(
    exact_test: &str,
    binding: LockRunnerStoredPoisonBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    stored_poison_dispatch::validate_binding(binding)?;
    if let Some(root) = selected_child_root()? {
        let selected = std::env::var(STORED_POISON_SELECTOR_ENV)
            .context("read parent-selected Lock stored-poison program")?;
        if selected == stored_poison_dispatch::exact_selector(binding) {
            stored_poison_dispatch::exercise_child(&root, binding)?;
        }
        return Ok(LockRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_stored_poison_parent(exact_test, binding)
}

fn run_parent(
    exact_test: &str,
    binding: LockRunnerProgramBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!("Lock quotient exact test name is empty"));
    }
    let executable = std::env::current_exe().context("resolve current Lock test executable")?;
    let root = create_private_child_root()?;
    let launch = ChildLaunchIdentity::new();
    let spawned = match Command::new(executable)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_ROOT_ENV, &root)
        .env(super::A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Err(cleanup_failed_root(&root, anyhow!(error))),
    };
    let bound = launch
        .bind(spawned)
        .map_err(|failure| handle_child_failure(&root, failure))?;
    let child = bound
        .wait_for_successful_report()
        .map_err(|failure| handle_child_failure(&root, failure))?;
    validate_parent_receipt(&root, binding, child)
        .map_err(|error| cleanup_failed_root(&root, error))
}

fn run_lifecycle_parent(
    exact_test: &str,
    binding: LockRunnerLifecycleBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!("Lock lifecycle exact test name is empty"));
    }
    let executable = std::env::current_exe().context("resolve current Lock test executable")?;
    let root = create_private_child_root()?;
    let launch = ChildLaunchIdentity::new();
    let spawned = match Command::new(executable)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_ROOT_ENV, &root)
        .env(super::A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Err(cleanup_failed_root(&root, anyhow!(error))),
    };
    let bound = launch
        .bind(spawned)
        .map_err(|failure| handle_child_failure(&root, failure))?;
    let child = bound
        .wait_for_successful_report()
        .map_err(|failure| handle_child_failure(&root, failure))?;
    validate_lifecycle_parent_receipt(&root, binding, child)
        .map_err(|error| cleanup_failed_root(&root, error))
}

fn run_stored_poison_parent(
    exact_test: &str,
    binding: LockRunnerStoredPoisonBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!("Lock stored-poison exact test name is empty"));
    }
    let executable =
        std::env::current_exe().context("resolve current Lock stored-poison test executable")?;
    let root = create_private_child_root()?;
    let launch = ChildLaunchIdentity::new();
    let selector = stored_poison_dispatch::exact_selector(binding);
    let spawned = match Command::new(executable)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_ROOT_ENV, &root)
        .env(STORED_POISON_SELECTOR_ENV, selector)
        .env(super::A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Err(cleanup_failed_root(&root, anyhow!(error))),
    };
    let bound = launch
        .bind(spawned)
        .map_err(|failure| handle_child_failure(&root, failure))?;
    let child = bound
        .wait_for_successful_report()
        .map_err(|failure| handle_child_failure(&root, failure))?;
    validate_stored_poison_parent_receipt(&root, binding, child)
        .map_err(|error| cleanup_failed_root(&root, error))
}

fn validate_parent_receipt(
    root: &Path,
    binding: LockRunnerProgramBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::LockQuotient) {
        return Err(anyhow!("Lock quotient child payload family mismatch"));
    }
    let registration_id = request_validation::validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(registration_id) {
        return Err(anyhow!("Lock quotient child registration binding mismatch"));
    }
    let environment =
        WindowsDynamicEnvironment::capture(root, &child).map_err(anyhow::Error::msg)?;
    let cleanup = ValidatedParentCleanupReceipt::remove_after_child_exit(&child, &environment)
        .map_err(anyhow::Error::msg)?;
    let child_fingerprint = child.fingerprint();
    if child_fingerprint != cleanup.child_fingerprint
        || child.root_commitment != cleanup.root_commitment
        || child.registration_commitment != cleanup.registration_commitment
    {
        return Err(anyhow!("Lock quotient parent cleanup binding mismatch"));
    }
    Ok(LockRunnerIsolatedEvidenceV1::ParentReceipt(
        LockRunnerEvidenceReceiptV1 {
            root_commitment_sha256: child.root_commitment.0,
            child_fingerprint_sha256: child_fingerprint.0,
            registration_commitment_sha256: child.registration_commitment.0,
            payload_commitment_sha256: child.payload_commitment.0,
            environment_sha256: digest_environment(&environment),
            cleanup_sha256: digest_cleanup(&cleanup),
            native_receipt_sha256: digest_native_receipt(child.actual_payload()),
            child_exit_code: child.exit_code,
        },
    ))
}

fn validate_lifecycle_parent_receipt(
    root: &Path,
    binding: LockRunnerLifecycleBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::LockQuotient) {
        return Err(anyhow!("Lock lifecycle child payload family mismatch"));
    }
    let payload = lifecycle::validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(payload.registration_id) {
        return Err(anyhow!(
            "Lock lifecycle child registration binding mismatch"
        ));
    }
    let environment =
        WindowsDynamicEnvironment::capture(root, &child).map_err(anyhow::Error::msg)?;
    let cleanup = ValidatedParentCleanupReceipt::remove_after_child_exit(&child, &environment)
        .map_err(anyhow::Error::msg)?;
    let child_fingerprint = child.fingerprint();
    if child_fingerprint != cleanup.child_fingerprint
        || child.root_commitment != cleanup.root_commitment
        || child.registration_commitment != cleanup.registration_commitment
    {
        return Err(anyhow!("Lock lifecycle parent cleanup binding mismatch"));
    }
    Ok(LockRunnerIsolatedEvidenceV1::ParentReceipt(
        LockRunnerEvidenceReceiptV1 {
            root_commitment_sha256: child.root_commitment.0,
            child_fingerprint_sha256: child_fingerprint.0,
            registration_commitment_sha256: child.registration_commitment.0,
            payload_commitment_sha256: child.payload_commitment.0,
            environment_sha256: digest_environment(&environment),
            cleanup_sha256: digest_cleanup(&cleanup),
            native_receipt_sha256: payload.native_receipt_sha256,
            child_exit_code: child.exit_code,
        },
    ))
}

fn validate_stored_poison_parent_receipt(
    root: &Path,
    binding: LockRunnerStoredPoisonBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::LockQuotient) {
        return Err(anyhow!("Lock stored-poison child payload family mismatch"));
    }
    let payload = stored_poison_dispatch::validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(payload.registration_id) {
        return Err(anyhow!(
            "Lock stored-poison child registration binding mismatch"
        ));
    }
    let environment =
        WindowsDynamicEnvironment::capture(root, &child).map_err(anyhow::Error::msg)?;
    let cleanup = ValidatedParentCleanupReceipt::remove_after_child_exit(&child, &environment)
        .map_err(anyhow::Error::msg)?;
    let child_fingerprint = child.fingerprint();
    if child_fingerprint != cleanup.child_fingerprint
        || child.root_commitment != cleanup.root_commitment
        || child.registration_commitment != cleanup.registration_commitment
    {
        return Err(anyhow!(
            "Lock stored-poison parent cleanup binding mismatch"
        ));
    }
    Ok(LockRunnerIsolatedEvidenceV1::ParentReceipt(
        LockRunnerEvidenceReceiptV1 {
            root_commitment_sha256: child.root_commitment.0,
            child_fingerprint_sha256: child_fingerprint.0,
            registration_commitment_sha256: child.registration_commitment.0,
            payload_commitment_sha256: child.payload_commitment.0,
            environment_sha256: digest_environment(&environment),
            cleanup_sha256: digest_cleanup(&cleanup),
            native_receipt_sha256: payload.native_receipt_sha256,
            child_exit_code: child.exit_code,
        },
    ))
}

fn selected_child_root() -> anyhow::Result<Option<PathBuf>> {
    let Some(root) = std::env::var_os(CHILD_ROOT_ENV).map(PathBuf::from) else {
        return Ok(None);
    };
    if !root.is_absolute() {
        return Err(anyhow!("Lock quotient child root is not absolute"));
    }
    Ok(Some(root))
}

fn create_private_child_root() -> anyhow::Result<PathBuf> {
    let requested = std::env::temp_dir().join(format!(
        "elon-a2-lock-quotient-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir(&requested).context("create fresh parent-owned Lock quotient root")?;
    match fs::canonicalize(&requested) {
        Ok(root) if root.is_absolute() => Ok(root),
        Ok(_) => Err(cleanup_failed_root(
            &requested,
            anyhow!("canonical Lock quotient root is not absolute"),
        )),
        Err(error) => Err(cleanup_failed_root(&requested, anyhow!(error))),
    }
}

fn handle_child_failure(root: &Path, failure: DynamicChildFailure) -> anyhow::Error {
    let exit_confirmed = failure.exit_confirmed();
    let error = anyhow!(failure);
    if exit_confirmed {
        cleanup_failed_root(root, error)
    } else {
        error.context("retained Lock quotient root because child exit is unconfirmed")
    }
}

fn cleanup_failed_root(root: &Path, error: anyhow::Error) -> anyhow::Error {
    match fs::remove_dir_all(root) {
        Ok(()) => error,
        Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup) => error.context(format!("Lock quotient fallback cleanup failed: {cleanup}")),
    }
}

fn digest_native_receipt(payload: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-quotient-native-receipt-v1\0");
    hasher.update((payload.len() as u64).to_le_bytes());
    hasher.update(payload.as_bytes());
    hasher.finalize().into()
}

fn digest_environment(value: &WindowsDynamicEnvironment) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-quotient-environment-v1\0");
    for field in [
        value.git_sha.as_str(),
        value.target,
        value.windows_build.as_str(),
        value.architecture,
        value.volume_kind,
        value.filesystem.as_str(),
        value.bundled_sqlite.as_str(),
    ] {
        hasher.update((field.len() as u64).to_le_bytes());
        hasher.update(field.as_bytes());
    }
    hasher.update(value.root_commitment.0);
    hasher.update(value.child_fingerprint.0);
    hasher.update(value.registration_commitment.0);
    hasher.finalize().into()
}

fn digest_cleanup(value: &ValidatedParentCleanupReceipt) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-quotient-parent-cleanup-v1\0");
    hasher.update(value.child_fingerprint.0);
    hasher.update(value.root_commitment.0);
    hasher.update(value.registration_commitment.0);
    hasher.finalize().into()
}
