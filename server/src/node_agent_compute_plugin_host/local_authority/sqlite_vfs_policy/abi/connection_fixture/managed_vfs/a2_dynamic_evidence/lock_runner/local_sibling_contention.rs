//! Exact installed-ABI execution for real coordinator sibling contention.

pub(super) mod fixture;
mod payload;

use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context};

use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestLockExpectation, ManagedSqliteShmTestLockReceipt,
    ManagedSqliteShmTestTargetObserver,
};

use super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};
use super::{
    ChildLaunchIdentity, LockRunnerActionV1, LockRunnerEvidenceReceiptV1,
    LockRunnerIsolatedEvidenceV1, SanitizedPayloadFamily, ValidatedChildProcessReceipt,
    ValidatedParentCleanupReceipt, WindowsDynamicEnvironment, CHILD_ROOT_ENV,
};

const SELECTOR_ENV: &str = "ELON_SQLITE_A2_LOCK_LOCAL_SIBLING_CONTENTION_SELECTOR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct LockRunnerLocalSiblingContentionBindingV1 {
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

struct ArmedLockObservation<'a> {
    observer: &'a ManagedSqliteShmTestTargetObserver,
    active: bool,
}

impl<'a> ArmedLockObservation<'a> {
    fn begin(
        observer: &'a ManagedSqliteShmTestTargetObserver,
        expectation: ManagedSqliteShmTestLockExpectation,
    ) -> anyhow::Result<Self> {
        observer
            .begin_lock_action_observation(expectation)
            .map_err(anyhow::Error::msg)?;
        Ok(Self {
            observer,
            active: true,
        })
    }

    fn finish(mut self) -> anyhow::Result<ManagedSqliteShmTestLockReceipt> {
        let receipt = self
            .observer
            .finish_lock_action_observation()
            .map_err(anyhow::Error::msg)?;
        self.active = false;
        Ok(receipt)
    }
}

impl Drop for ArmedLockObservation<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.observer.cancel_lock_action_observation();
        }
    }
}

pub(in super::super::super) fn run_lock_local_sibling_contention_program_isolated(
    exact_test: &str,
    binding: LockRunnerLocalSiblingContentionBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    validate_binding(binding)?;
    if let Some(root) = super::selected_child_root()? {
        let selected = std::env::var(SELECTOR_ENV)
            .context("read parent-selected Lock local sibling-contention program")?;
        if selected == exact_selector(binding) {
            exercise_child(&root, binding)?;
        }
        return Ok(LockRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_parent(exact_test, binding)
}

fn run_parent(
    exact_test: &str,
    binding: LockRunnerLocalSiblingContentionBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!(
            "Lock local sibling-contention exact test name is empty"
        ));
    }
    let executable = std::env::current_exe()
        .context("resolve current Lock local sibling-contention test executable")?;
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
    binding: LockRunnerLocalSiblingContentionBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::LockQuotient) {
        return Err(anyhow!(
            "Lock local sibling-contention child payload family mismatch"
        ));
    }
    let payload = payload::validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(payload.registration_id) {
        return Err(anyhow!(
            "Lock local sibling-contention child registration binding mismatch"
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
            "Lock local sibling-contention parent cleanup binding mismatch"
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
    binding: LockRunnerLocalSiblingContentionBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created Lock local sibling-contention child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;

    let fixture = fixture::prepare(root)?;
    let selected_binding = fixture
        .route(fixture::SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let selected_observer = selected_binding.observer().map_err(anyhow::Error::msg)?;
    let target = selected_binding
        .target_witness()
        .map_err(anyhow::Error::msg)?;
    let sibling_binding = fixture
        .route(fixture::SIBLING)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let sibling_observer = sibling_binding.observer().map_err(anyhow::Error::msg)?;

    fixture::install_prestate(&fixture, binding)?;
    let selected_before = selected_observer.snapshot()?;
    let sibling_before = sibling_observer.snapshot()?;
    fixture::validate_prestate(binding, selected_before, sibling_before)?;

    let lock_observation =
        ArmedLockObservation::begin(&selected_observer, lock_expectation(binding))?;
    let callback = fixture
        .route(fixture::SELECTED)?
        .observe_main_shm_lock_raw(
            i32::from(binding.first),
            i32::from(binding.count),
            fixture::raw_flags(binding.action),
        )
        .map_err(anyhow::Error::msg)?;
    let selected_after = selected_observer.snapshot()?;
    let sibling_after = sibling_observer.snapshot()?;
    let lower_receipt = lock_observation.finish()?;
    let pending_count = selected_binding
        .pending_count()
        .map_err(anyhow::Error::msg)?;
    fixture::validate_action(
        binding,
        callback,
        selected_before,
        selected_after,
        sibling_before,
        sibling_after,
        lower_receipt,
        pending_count,
    )?;

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

    fixture::cleanup_locks(&fixture, binding)?;
    let selected_cleaned = selected_observer.snapshot()?;
    let sibling_cleaned = sibling_observer.snapshot()?;
    fixture::validate_cleanup(selected_cleaned, sibling_cleaned)?;
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
        callback,
        selected_before,
        selected_after,
        sibling_before,
        sibling_after,
        selected_cleaned,
        sibling_cleaned,
        lower_receipt,
        pending_count,
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
    binding: LockRunnerLocalSiblingContentionBindingV1,
) -> anyhow::Result<()> {
    let end = binding
        .first
        .checked_add(binding.count)
        .ok_or_else(|| anyhow!("Lock local sibling-contention range overflow"))?;
    if binding.count == 0 || binding.first >= 8 || end > 8 {
        return Err(anyhow!(
            "Lock local sibling-contention range is outside eight slots"
        ));
    }
    if binding.action == LockRunnerActionV1::LockShared && binding.count != 1 {
        return Err(anyhow!(
            "Lock local sibling-contention shared range is not one slot"
        ));
    }
    if !matches!(
        binding.action,
        LockRunnerActionV1::LockShared | LockRunnerActionV1::LockExclusive
    ) {
        return Err(anyhow!(
            "Lock local sibling-contention action is not an acquire"
        ));
    }
    let mask = (((1_u16 << binding.count) - 1) << binding.first) as u8;
    if binding.mask != mask {
        return Err(anyhow!("Lock local sibling-contention range mask mismatch"));
    }
    Ok(())
}

pub(super) fn lock_expectation(
    binding: LockRunnerLocalSiblingContentionBindingV1,
) -> ManagedSqliteShmTestLockExpectation {
    fixture::lock_expectation(binding)
}

pub(super) fn action_tag(action: LockRunnerActionV1) -> u64 {
    fixture::action_tag(action)
}

pub(super) fn exact_selector(binding: LockRunnerLocalSiblingContentionBindingV1) -> String {
    super::super::child::lock_local_sibling_contention::selector(
        action_tag(binding.action),
        binding.first,
        binding.count,
    )
    .expect("validated Lock local sibling-contention selector")
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn selected_lock_local_sibling_contention_selector_for_test(
) -> Option<String> {
    std::env::var_os(CHILD_ROOT_ENV)?;
    std::env::var(SELECTOR_ENV).ok()
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn lock_local_sibling_contention_selector_for_test(
    action_tag: u64,
    first: u8,
    count: u8,
) -> Result<String, &'static str> {
    super::super::child::lock_local_sibling_contention::selector(action_tag, first, count)
}
