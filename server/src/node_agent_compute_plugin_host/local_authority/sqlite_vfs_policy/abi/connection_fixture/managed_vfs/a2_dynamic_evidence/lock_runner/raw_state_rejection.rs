//! Process-isolated q11 Lock raw-state rejection receipts.
//!
//! Every child exercises the installed production `xShmLock` callback with one controlled,
//! memory-safe raw representation. The resulting receipt is deliberately classified as
//! `controlled_fault_actual`; it is not evidence of natural production reachability.

mod fixture;
mod payload;

use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context};

use super::super::A2_DYNAMIC_CHILD_NONCE_ENV;
use super::{
    ChildLaunchIdentity, LockRunnerEvidenceReceiptV1, LockRunnerIsolatedEvidenceV1,
    SanitizedPayloadFamily, ValidatedChildProcessReceipt, ValidatedParentCleanupReceipt,
    WindowsDynamicEnvironment, CHILD_ROOT_ENV,
};

const SELECTOR_ENV: &str = "ELON_SQLITE_A2_LOCK_RAW_STATE_REJECTION_SELECTOR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum LockRunnerRawStateRejectionV1 {
    NullFileDirect,
    UninstalledDirect,
    MethodsNullStatePresentDirect,
    ForeignMethodsStateNullDirect,
    ForeignMethodsStatePresentDirect,
    ExactMethodsStateNullDirect,
    OtherTypePayloadMissingDropCompleted,
    OtherTypePayloadPresentDropCompleted,
    OtherTypePayloadPresentDropUnwindCaught,
    ExpectedTypePayloadMissingDropCompleted,
    HandleBoundFileMissingDirect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct LockRunnerRawStateRejectionBindingV1 {
    pub(in super::super::super) rejection: LockRunnerRawStateRejectionV1,
    pub(in super::super::super) normalized_descriptor_sha256: [u8; 32],
    pub(in super::super::super) case_key_sha256: [u8; 32],
    pub(in super::super::super) full_record_sha256: [u8; 32],
    pub(in super::super::super) plan_sha256: [u8; 32],
    pub(in super::super::super) implementation_sha256: [u8; 32],
}

pub(in super::super::super) fn run_lock_raw_state_rejection_program_isolated(
    exact_test: &str,
    binding: LockRunnerRawStateRejectionBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    validate_binding(binding)?;
    if let Some(root) = super::selected_child_root()? {
        let selected =
            std::env::var(SELECTOR_ENV).context("read parent-selected q11 Lock program")?;
        if selected == exact_selector(binding) {
            fixture::exercise_child(&root, binding)?;
        }
        return Ok(LockRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_parent(exact_test, binding)
}

fn run_parent(
    exact_test: &str,
    binding: LockRunnerRawStateRejectionBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!("q11 Lock raw-state exact test name is empty"));
    }
    let executable =
        std::env::current_exe().context("resolve q11 Lock raw-state test executable")?;
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
    binding: LockRunnerRawStateRejectionBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::LockQuotient) {
        return Err(anyhow!("q11 Lock raw-state child payload family mismatch"));
    }
    let payload = payload::validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(payload.registration_id) {
        return Err(anyhow!(
            "q11 Lock raw-state child registration binding mismatch"
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
            "q11 Lock raw-state parent cleanup binding mismatch"
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
    binding: LockRunnerRawStateRejectionBindingV1,
) -> anyhow::Result<()> {
    super::super::child::lock_raw_state_rejection::selector(
        raw_state_tag(binding.rejection),
        completion_tag(binding.rejection),
    )
    .map(|_| ())
    .map_err(anyhow::Error::msg)
}

pub(super) fn exact_selector(binding: LockRunnerRawStateRejectionBindingV1) -> &'static str {
    super::super::child::lock_raw_state_rejection::selector(
        raw_state_tag(binding.rejection),
        completion_tag(binding.rejection),
    )
    .expect("validated q11 Lock raw-state selector")
}

pub(super) const fn rejection_tag(value: LockRunnerRawStateRejectionV1) -> u64 {
    match value {
        LockRunnerRawStateRejectionV1::NullFileDirect => 1,
        LockRunnerRawStateRejectionV1::UninstalledDirect => 2,
        LockRunnerRawStateRejectionV1::MethodsNullStatePresentDirect => 3,
        LockRunnerRawStateRejectionV1::ForeignMethodsStateNullDirect => 4,
        LockRunnerRawStateRejectionV1::ForeignMethodsStatePresentDirect => 5,
        LockRunnerRawStateRejectionV1::ExactMethodsStateNullDirect => 6,
        LockRunnerRawStateRejectionV1::OtherTypePayloadMissingDropCompleted => 7,
        LockRunnerRawStateRejectionV1::OtherTypePayloadPresentDropCompleted => 8,
        LockRunnerRawStateRejectionV1::OtherTypePayloadPresentDropUnwindCaught => 9,
        LockRunnerRawStateRejectionV1::ExpectedTypePayloadMissingDropCompleted => 10,
        LockRunnerRawStateRejectionV1::HandleBoundFileMissingDirect => 11,
    }
}

pub(super) const fn raw_state_tag(value: LockRunnerRawStateRejectionV1) -> u64 {
    match value {
        LockRunnerRawStateRejectionV1::NullFileDirect => 1,
        LockRunnerRawStateRejectionV1::UninstalledDirect => 2,
        LockRunnerRawStateRejectionV1::MethodsNullStatePresentDirect => 3,
        LockRunnerRawStateRejectionV1::ForeignMethodsStateNullDirect => 4,
        LockRunnerRawStateRejectionV1::ForeignMethodsStatePresentDirect => 5,
        LockRunnerRawStateRejectionV1::ExactMethodsStateNullDirect => 6,
        LockRunnerRawStateRejectionV1::OtherTypePayloadMissingDropCompleted => 7,
        LockRunnerRawStateRejectionV1::OtherTypePayloadPresentDropCompleted
        | LockRunnerRawStateRejectionV1::OtherTypePayloadPresentDropUnwindCaught => 8,
        LockRunnerRawStateRejectionV1::ExpectedTypePayloadMissingDropCompleted => 9,
        LockRunnerRawStateRejectionV1::HandleBoundFileMissingDirect => 10,
    }
}

pub(super) const fn completion_tag(value: LockRunnerRawStateRejectionV1) -> u64 {
    match value {
        LockRunnerRawStateRejectionV1::OtherTypePayloadMissingDropCompleted
        | LockRunnerRawStateRejectionV1::OtherTypePayloadPresentDropCompleted
        | LockRunnerRawStateRejectionV1::ExpectedTypePayloadMissingDropCompleted => 6,
        LockRunnerRawStateRejectionV1::OtherTypePayloadPresentDropUnwindCaught => 7,
        _ => 1,
    }
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn selected_lock_raw_state_rejection_selector_for_test(
) -> Option<String> {
    std::env::var_os(CHILD_ROOT_ENV)?;
    std::env::var(SELECTOR_ENV).ok()
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn lock_raw_state_rejection_selector_for_test(
    raw_state_tag: u64,
    completion_tag: u64,
) -> Result<String, &'static str> {
    super::super::child::lock_raw_state_rejection::selector(raw_state_tag, completion_tag)
        .map(str::to_owned)
}
