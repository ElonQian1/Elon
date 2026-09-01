//! Process-isolated real installed-ABI evidence for local Lock protocol rejections.

mod fixture;
mod payload;

use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context};

use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestLockExpectation, ManagedSqliteShmTestLockPath,
};

use super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};
use super::{
    lifecycle, ChildLaunchIdentity, LockRunnerActionV1, LockRunnerEvidenceReceiptV1,
    LockRunnerIsolatedEvidenceV1, SanitizedPayloadFamily, ValidatedChildProcessReceipt,
    ValidatedParentCleanupReceipt, WindowsDynamicEnvironment, CHILD_ROOT_ENV,
};

const SELECTOR_ENV: &str = "ELON_SQLITE_A2_LOCK_LOCAL_PROTOCOL_REJECTION_SELECTOR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum LocalProtocolRejectionPathV1 {
    OwnOverlap,
    NotHeld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct LockRunnerLocalProtocolRejectionBindingV1 {
    pub(in super::super::super) path: LocalProtocolRejectionPathV1,
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

pub(in super::super::super) fn run_lock_local_protocol_rejection_program_isolated(
    exact_test: &str,
    binding: LockRunnerLocalProtocolRejectionBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    validate_binding(binding)?;
    if let Some(root) = super::selected_child_root()? {
        let selected = std::env::var(SELECTOR_ENV)
            .context("read parent-selected Lock local protocol-rejection program")?;
        if selected == exact_selector(binding) {
            exercise_child(&root, binding)?;
        }
        return Ok(LockRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_parent(exact_test, binding)
}

fn run_parent(
    exact_test: &str,
    binding: LockRunnerLocalProtocolRejectionBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!(
            "Lock local protocol-rejection exact test name is empty"
        ));
    }
    let executable = std::env::current_exe()
        .context("resolve current Lock local protocol-rejection test executable")?;
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
    binding: LockRunnerLocalProtocolRejectionBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::LockQuotient) {
        return Err(anyhow!(
            "Lock local protocol-rejection child payload family mismatch"
        ));
    }
    let payload = payload::validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(payload.registration_id) {
        return Err(anyhow!(
            "Lock local protocol-rejection child registration binding mismatch"
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
            "Lock local protocol-rejection parent cleanup binding mismatch"
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

fn exercise_child(
    root: &Path,
    binding: LockRunnerLocalProtocolRejectionBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created Lock local protocol-rejection child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;

    let fixture = fixture::prepare(root)?;
    let selected_binding = fixture
        .route(fixture::SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let observer = selected_binding.observer().map_err(anyhow::Error::msg)?;
    let target = selected_binding
        .target_witness()
        .map_err(anyhow::Error::msg)?;

    let setup = fixture::install_prestate(&fixture, binding)?;
    let before = observer.snapshot()?;
    fixture::validate_prestate(binding, before, setup)?;

    let observation = fixture::ArmedLockObservation::begin(&observer, lock_expectation(binding))?;
    let callback = fixture
        .route(fixture::SELECTED)?
        .observe_main_shm_lock_raw(
            i32::from(binding.first),
            i32::from(binding.count),
            lifecycle::raw_flags(binding.action),
        )
        .map_err(anyhow::Error::msg)?;
    let after = observer.snapshot()?;
    let lower = observation.finish()?;
    let pending_count = selected_binding
        .pending_count()
        .map_err(anyhow::Error::msg)?;
    let active_route = fixture::active_route_values(&fixture)?;
    fixture::validate_action(
        binding,
        callback,
        before,
        after,
        lower,
        pending_count,
        active_route,
    )?;

    let cleanup = fixture::cleanup(&fixture, binding)?;
    let cleaned = observer.snapshot()?;
    fixture::validate_cleanup(binding, cleaned, cleanup)?;

    let registration = fixture.live_registration_snapshot()?;
    let registration_values = [
        u64::from(registration.registered()),
        u64::from(registration.table_present()),
        u64::from(registration.name_present()),
        u64::from(registration.context_present()),
    ];
    let (routes, logical_names) = fixture.logical_route_counts()?;
    let route_values = [
        fixture.live_connection_count() as u64,
        routes as u64,
        logical_names as u64,
    ];
    let autocommit = fixture.connection(fixture::SELECTED)?.is_autocommit();
    let liveness: i64 =
        fixture
            .connection(fixture::SELECTED)?
            .query_row("SELECT 1", [], |row| row.get(0))?;
    fixture.close()?;
    let terminal_values = [
        u64::from(autocommit),
        u64::from(liveness == 1),
        1,
        u64::from(root.is_dir()),
    ];
    let payload = payload::encode(
        binding,
        target.registration_id(),
        target.route_ordinal(),
        target.runtime_generation(),
        target.shm_connection_id(),
        setup,
        callback,
        before,
        after,
        lower,
        pending_count,
        active_route,
        cleanup,
        cleaned,
        registration_values,
        route_values,
        terminal_values,
    );
    let report = SanitizedChildReport::encode_for_current_child(
        &nonce,
        root,
        target.registration_id(),
        &payload,
    )
    .map_err(anyhow::Error::msg)?;
    println!("{report}");
    Ok(())
}

pub(super) fn validate_binding(
    binding: LockRunnerLocalProtocolRejectionBindingV1,
) -> anyhow::Result<()> {
    let end = binding
        .first
        .checked_add(binding.count)
        .ok_or_else(|| anyhow!("Lock local protocol-rejection range overflow"))?;
    if binding.count == 0 || binding.first >= 8 || end > 8 {
        return Err(anyhow!(
            "Lock local protocol-rejection range is outside eight slots"
        ));
    }
    let mask = (((1_u16 << binding.count) - 1) << binding.first) as u8;
    if binding.mask != mask {
        return Err(anyhow!("Lock local protocol-rejection range mask mismatch"));
    }
    super::super::child::lock_local_protocol_rejection::selector(
        path_tag(binding.path),
        lifecycle::action_tag(binding.action),
        binding.first,
        binding.count,
    )
    .map(|_| ())
    .map_err(anyhow::Error::msg)
}

pub(super) fn lock_expectation(
    binding: LockRunnerLocalProtocolRejectionBindingV1,
) -> ManagedSqliteShmTestLockExpectation {
    ManagedSqliteShmTestLockExpectation {
        action: lifecycle::managed_action(binding.action),
        first: binding.first,
        count: binding.count,
        mask: binding.mask,
        path: ManagedSqliteShmTestLockPath::LocalProtocolRejection,
    }
}

pub(super) fn exact_selector(binding: LockRunnerLocalProtocolRejectionBindingV1) -> String {
    super::super::child::lock_local_protocol_rejection::selector(
        path_tag(binding.path),
        lifecycle::action_tag(binding.action),
        binding.first,
        binding.count,
    )
    .expect("validated Lock local protocol-rejection selector")
}

pub(super) const fn path_tag(path: LocalProtocolRejectionPathV1) -> u64 {
    match path {
        LocalProtocolRejectionPathV1::OwnOverlap => 1,
        LocalProtocolRejectionPathV1::NotHeld => 2,
    }
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn selected_lock_local_protocol_rejection_selector_for_test(
) -> Option<String> {
    std::env::var_os(CHILD_ROOT_ENV)?;
    std::env::var(SELECTOR_ENV).ok()
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn lock_local_protocol_rejection_selector_for_test(
    path_tag: u64,
    action_tag: u64,
    first: u8,
    count: u8,
) -> Result<String, &'static str> {
    super::super::child::lock_local_protocol_rejection::selector(path_tag, action_tag, first, count)
}
