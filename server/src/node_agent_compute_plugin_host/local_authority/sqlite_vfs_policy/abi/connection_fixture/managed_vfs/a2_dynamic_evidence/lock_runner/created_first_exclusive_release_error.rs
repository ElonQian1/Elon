//! Process-isolated q12 receipts for one controlled initialization release uncertainty.
//!
//! The child invokes the installed production `xShmLock` callback and the production
//! `UnlockFileEx` site. The test seam discards only that native return receipt, so every emitted
//! payload is classified as `controlled_fault_actual`, never as natural production evidence.

mod fixture;
mod payload;

use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context};

use super::super::A2_DYNAMIC_CHILD_NONCE_ENV;
use super::{
    ChildLaunchIdentity, LockRunnerActionV1, LockRunnerEvidenceReceiptV1,
    LockRunnerIsolatedEvidenceV1, SanitizedPayloadFamily, ValidatedChildProcessReceipt,
    ValidatedParentCleanupReceipt, WindowsDynamicEnvironment, CHILD_ROOT_ENV,
};

const SELECTOR_ENV: &str = "ELON_SQLITE_A2_LOCK_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_SELECTOR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum LockRunnerCreatedFirstExclusiveReleaseCompletionV1 {
    RetentionSucceeded,
    RetentionRouteUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct LockRunnerNativeAcquireCreatedFirstExclusiveReleaseErrorBindingV1
{
    pub(in super::super::super) action: LockRunnerActionV1,
    pub(in super::super::super) first: u8,
    pub(in super::super::super) count: u8,
    pub(in super::super::super) mask: u8,
    pub(in super::super::super) completion: LockRunnerCreatedFirstExclusiveReleaseCompletionV1,
    pub(in super::super::super) normalized_descriptor_sha256: [u8; 32],
    pub(in super::super::super) case_key_sha256: [u8; 32],
    pub(in super::super::super) full_record_sha256: [u8; 32],
    pub(in super::super::super) plan_sha256: [u8; 32],
    pub(in super::super::super) implementation_sha256: [u8; 32],
}

pub(in super::super::super) fn run_lock_native_acquire_created_first_exclusive_release_error_program_isolated(
    exact_test: &str,
    binding: LockRunnerNativeAcquireCreatedFirstExclusiveReleaseErrorBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    validate_binding(binding)?;
    if let Some(root) = super::selected_child_root()? {
        let selected = std::env::var(SELECTOR_ENV)
            .context("read parent-selected q12 Lock initialization program")?;
        if selected == exact_selector(binding) {
            fixture::exercise_child(&root, binding)?;
        }
        return Ok(LockRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_parent(exact_test, binding)
}

fn run_parent(
    exact_test: &str,
    binding: LockRunnerNativeAcquireCreatedFirstExclusiveReleaseErrorBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!("q12 Lock initialization exact test name is empty"));
    }
    let executable =
        std::env::current_exe().context("resolve q12 Lock initialization test executable")?;
    let root = super::create_private_child_root()?;
    let launch = ChildLaunchIdentity::new();
    let spawned = match Command::new(executable)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_ROOT_ENV, &root)
        .env(SELECTOR_ENV, exact_selector(binding))
        .env(A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Err(super::cleanup_failed_root(&root, anyhow!(error))),
    };
    let bound = launch
        .bind(spawned)
        .map_err(|failure| super::handle_child_failure(&root, failure))?;
    let child = bound
        .wait_for_successful_report()
        .map_err(|failure| super::handle_child_failure(&root, failure))?;
    validate_parent_receipt(&root, binding, child)
        .map_err(|error| super::cleanup_failed_root(&root, error))
}

fn validate_parent_receipt(
    root: &Path,
    binding: LockRunnerNativeAcquireCreatedFirstExclusiveReleaseErrorBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::LockQuotient) {
        return Err(anyhow!(
            "q12 Lock initialization child payload family mismatch"
        ));
    }
    let payload = payload::validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(payload.registration_id) {
        return Err(anyhow!(
            "q12 Lock initialization child registration binding mismatch"
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
            "q12 Lock initialization parent cleanup binding mismatch"
        ));
    }
    Ok(LockRunnerIsolatedEvidenceV1::ParentReceipt(
        LockRunnerEvidenceReceiptV1 {
            root_commitment_sha256: child.root_commitment.0,
            child_fingerprint_sha256: child_fingerprint.0,
            registration_commitment_sha256: child.registration_commitment.0,
            payload_commitment_sha256: child.payload_commitment.0,
            environment_sha256: super::digest_environment(&environment),
            cleanup_sha256: super::digest_cleanup(&cleanup),
            native_receipt_sha256: payload.native_receipt_sha256,
            child_exit_code: child.exit_code,
        },
    ))
}

pub(super) fn validate_binding(
    binding: LockRunnerNativeAcquireCreatedFirstExclusiveReleaseErrorBindingV1,
) -> anyhow::Result<()> {
    let end = binding
        .first
        .checked_add(binding.count)
        .ok_or_else(|| anyhow!("q12 Lock initialization range overflow"))?;
    if binding.count == 0 || binding.first >= 8 || end > 8 {
        return Err(anyhow!(
            "q12 Lock initialization range is outside eight slots"
        ));
    }
    if binding.action == LockRunnerActionV1::LockShared && binding.count != 1 {
        return Err(anyhow!(
            "q12 shared Lock initialization range is not one slot"
        ));
    }
    if !matches!(
        binding.action,
        LockRunnerActionV1::LockShared | LockRunnerActionV1::LockExclusive
    ) {
        return Err(anyhow!("q12 Lock initialization action is not an acquire"));
    }
    let mask = (((1_u16 << binding.count) - 1) << binding.first) as u8;
    if binding.mask != mask {
        return Err(anyhow!("q12 Lock initialization range mask mismatch"));
    }
    super::super::child::lock_created_first_exclusive_release_error::selector(
        super::lifecycle::action_tag(binding.action),
        binding.first,
        binding.count,
        completion_tag(binding.completion),
    )
    .map(|_| ())
    .map_err(anyhow::Error::msg)
}

pub(super) fn exact_selector(
    binding: LockRunnerNativeAcquireCreatedFirstExclusiveReleaseErrorBindingV1,
) -> String {
    super::super::child::lock_created_first_exclusive_release_error::selector(
        super::lifecycle::action_tag(binding.action),
        binding.first,
        binding.count,
        completion_tag(binding.completion),
    )
    .expect("validated q12 Lock initialization selector")
}

pub(super) const fn completion_tag(
    completion: LockRunnerCreatedFirstExclusiveReleaseCompletionV1,
) -> u64 {
    match completion {
        LockRunnerCreatedFirstExclusiveReleaseCompletionV1::RetentionSucceeded => 1,
        LockRunnerCreatedFirstExclusiveReleaseCompletionV1::RetentionRouteUnknown => 2,
    }
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn selected_lock_native_acquire_created_first_exclusive_release_error_selector_for_test(
) -> Option<String> {
    std::env::var_os(CHILD_ROOT_ENV)?;
    std::env::var(SELECTOR_ENV).ok()
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn lock_native_acquire_created_first_exclusive_release_error_selector_for_test(
    action: LockRunnerActionV1,
    first: u8,
    count: u8,
    mask: u8,
    completion: LockRunnerCreatedFirstExclusiveReleaseCompletionV1,
) -> Result<String, &'static str> {
    let binding = LockRunnerNativeAcquireCreatedFirstExclusiveReleaseErrorBindingV1 {
        action,
        first,
        count,
        mask,
        completion,
        normalized_descriptor_sha256: [0; 32],
        case_key_sha256: [0; 32],
        full_record_sha256: [0; 32],
        plan_sha256: [0; 32],
        implementation_sha256: [0; 32],
    };
    validate_binding(binding)
        .map(|()| exact_selector(binding))
        .map_err(|_| "q12 Lock initialization selector binding invalid")
}
