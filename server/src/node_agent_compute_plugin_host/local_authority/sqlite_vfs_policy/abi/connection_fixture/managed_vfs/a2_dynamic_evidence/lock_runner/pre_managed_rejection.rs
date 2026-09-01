//! Process-isolated q9 Lock callbacks rejected before managed/native dispatch.

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

const SELECTOR_ENV: &str = "ELON_SQLITE_A2_LOCK_PRE_MANAGED_REJECTION_SELECTOR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum LockRunnerPreManagedRejectionV1 {
    AdmissionRouteUnknown,
    AdmissionCounterOverflow,
    UnsupportedFileRole,
    ShmDetached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum LockRunnerPreManagedCompletionV1 {
    Direct,
    Completed,
    RouteUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct LockRunnerPreManagedRejectionBindingV1 {
    pub(in super::super::super) rejection: LockRunnerPreManagedRejectionV1,
    pub(in super::super::super) completion: LockRunnerPreManagedCompletionV1,
    pub(in super::super::super) action: LockRunnerActionV1,
    pub(in super::super::super) first: u8,
    pub(in super::super::super) count: u8,
    pub(in super::super::super) mask: u8,
    pub(in super::super::super) normalized_descriptor_sha256: [u8; 32],
    pub(in super::super::super) case_key_sha256: [u8; 32],
    pub(in super::super::super) full_record_sha256: [u8; 32],
    pub(in super::super::super) plan_sha256: [u8; 32],
    pub(in super::super::super) implementation_sha256: [u8; 32],
}

pub(in super::super::super) fn run_lock_pre_managed_rejection_program_isolated(
    exact_test: &str,
    binding: LockRunnerPreManagedRejectionBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    validate_binding(binding)?;
    if let Some(root) = super::selected_child_root()? {
        let selected = std::env::var(SELECTOR_ENV)
            .context("read parent-selected q9 Lock program")?;
        if selected == exact_selector(binding) {
            fixture::exercise_child(&root, binding)?;
        }
        return Ok(LockRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_parent(exact_test, binding)
}

fn run_parent(
    exact_test: &str,
    binding: LockRunnerPreManagedRejectionBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!("q9 Lock exact test name is empty"));
    }
    let executable = std::env::current_exe().context("resolve q9 Lock test executable")?;
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
    binding: LockRunnerPreManagedRejectionBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::LockQuotient) {
        return Err(anyhow!("q9 Lock child payload family mismatch"));
    }
    let payload = payload::validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(payload.registration_id) {
        return Err(anyhow!("q9 Lock child registration binding mismatch"));
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
        return Err(anyhow!("q9 Lock parent cleanup binding mismatch"));
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
    binding: LockRunnerPreManagedRejectionBindingV1,
) -> anyhow::Result<()> {
    let completion_valid = matches!(
        (binding.rejection, binding.completion),
        (
            LockRunnerPreManagedRejectionV1::AdmissionRouteUnknown
                | LockRunnerPreManagedRejectionV1::AdmissionCounterOverflow,
            LockRunnerPreManagedCompletionV1::Direct
        ) | (
            LockRunnerPreManagedRejectionV1::UnsupportedFileRole
                | LockRunnerPreManagedRejectionV1::ShmDetached,
            LockRunnerPreManagedCompletionV1::Completed
                | LockRunnerPreManagedCompletionV1::RouteUnknown
        )
    );
    let end = binding
        .first
        .checked_add(binding.count)
        .ok_or_else(|| anyhow!("q9 Lock range overflow"))?;
    let shared = matches!(
        binding.action,
        LockRunnerActionV1::LockShared | LockRunnerActionV1::UnlockShared
    );
    if !completion_valid
        || binding.count == 0
        || binding.first >= 8
        || end > 8
        || (shared && binding.count != 1)
    {
        return Err(anyhow!("q9 Lock program axes are invalid"));
    }
    if binding.mask != (((1_u16 << binding.count) - 1) << binding.first) as u8 {
        return Err(anyhow!("q9 Lock range mask mismatch"));
    }
    super::super::child::lock_pre_managed_rejection::selector(
        rejection_tag(binding.rejection),
        completion_tag(binding.completion),
        super::lifecycle::action_tag(binding.action),
        binding.first,
        binding.count,
    )
    .map(|_| ())
    .map_err(anyhow::Error::msg)
}

pub(super) fn exact_selector(binding: LockRunnerPreManagedRejectionBindingV1) -> String {
    super::super::child::lock_pre_managed_rejection::selector(
        rejection_tag(binding.rejection),
        completion_tag(binding.completion),
        super::lifecycle::action_tag(binding.action),
        binding.first,
        binding.count,
    )
    .expect("validated q9 Lock selector")
}

pub(super) const fn rejection_tag(value: LockRunnerPreManagedRejectionV1) -> u64 {
    match value {
        LockRunnerPreManagedRejectionV1::AdmissionRouteUnknown => 1,
        LockRunnerPreManagedRejectionV1::AdmissionCounterOverflow => 2,
        LockRunnerPreManagedRejectionV1::UnsupportedFileRole => 3,
        LockRunnerPreManagedRejectionV1::ShmDetached => 4,
    }
}

pub(super) const fn completion_tag(value: LockRunnerPreManagedCompletionV1) -> u64 {
    match value {
        LockRunnerPreManagedCompletionV1::Direct => 1,
        LockRunnerPreManagedCompletionV1::Completed => 2,
        LockRunnerPreManagedCompletionV1::RouteUnknown => 3,
    }
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn selected_lock_pre_managed_rejection_selector_for_test(
) -> Option<String> {
    std::env::var_os(CHILD_ROOT_ENV)?;
    std::env::var(SELECTOR_ENV).ok()
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn lock_pre_managed_rejection_selector_for_test(
    binding: LockRunnerPreManagedRejectionBindingV1,
) -> Result<String, &'static str> {
    validate_binding(binding)
        .map(|()| exact_selector(binding))
        .map_err(|_| "q9 Lock selector binding invalid")
}
