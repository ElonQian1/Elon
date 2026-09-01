//! Process-isolated q10 evidence for the seven installed xShmLock ABI-scalar rejections.

use std::{
    num::NonZeroU8,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use super::super::super::{
    connection::ManagedTestShmLockAbiLedgerObservation,
    lifecycle_faults::ManagedTestPreManagedLockPath, ManagedSqliteRoutedConnectionFixture,
    ManagedSqliteTestVfsRouteCustodySnapshot, ManagedSqliteTestVfsRoutePhase,
};
use super::super::{
    child::{lock_abi_scalar_rejection, SanitizedPayloadFamily},
    ChildLaunchIdentity, SanitizedChildReport, ValidatedChildProcessReceipt,
    ValidatedParentCleanupReceipt, WindowsDynamicEnvironment, A2_DYNAMIC_CHILD_NONCE_ENV,
};
use super::{
    LockRunnerEvidenceReceiptV1, LockRunnerIsolatedEvidenceV1, CHILD_ROOT_ENV,
};
use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAction, ManagedSqliteShmLockRequest,
};

mod payload;

use payload::{encode_payload, validate_payload};

const SELECTOR_ENV: &str = "ELON_SQLITE_A2_LOCK_ABI_SCALAR_REJECTION_SELECTOR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum LockRunnerAbiScalarValidityV1 {
    Invalid,
    Valid,
}

impl LockRunnerAbiScalarValidityV1 {
    const fn tag(self) -> u64 {
        match self {
            Self::Invalid => 1,
            Self::Valid => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct LockRunnerAbiScalarRejectionBindingV1 {
    pub(in super::super::super) offset: LockRunnerAbiScalarValidityV1,
    pub(in super::super::super) count: LockRunnerAbiScalarValidityV1,
    pub(in super::super::super) flags: LockRunnerAbiScalarValidityV1,
    pub(in super::super::super) normalized_descriptor_sha256: [u8; 32],
    pub(in super::super::super) case_key_sha256: [u8; 32],
    pub(in super::super::super) full_record_sha256: [u8; 32],
    pub(in super::super::super) plan_sha256: [u8; 32],
    pub(in super::super::super) implementation_sha256: [u8; 32],
}

pub(in super::super::super) fn run_lock_abi_scalar_rejection_program_isolated(
    exact_test: &str,
    binding: LockRunnerAbiScalarRejectionBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    validate_binding(binding)?;
    if let Some(root) = super::selected_child_root()? {
        let selected = std::env::var(SELECTOR_ENV)
            .context("read parent-selected q10 Lock ABI-scalar program")?;
        if selected == exact_selector(binding) {
            exercise_child(&root, binding)?;
        }
        return Ok(LockRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_parent(exact_test, binding)
}

fn run_parent(
    exact_test: &str,
    binding: LockRunnerAbiScalarRejectionBindingV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!("q10 Lock ABI-scalar exact test name is empty"));
    }
    let executable =
        std::env::current_exe().context("resolve q10 Lock ABI-scalar test executable")?;
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
    binding: LockRunnerAbiScalarRejectionBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::LockQuotient) {
        return Err(anyhow!("q10 Lock ABI-scalar child payload family mismatch"));
    }
    let registration_id = validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(registration_id) {
        return Err(anyhow!(
            "q10 Lock ABI-scalar child registration binding mismatch"
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
        return Err(anyhow!("q10 Lock ABI-scalar parent cleanup binding mismatch"));
    }
    Ok(LockRunnerIsolatedEvidenceV1::ParentReceipt(
        LockRunnerEvidenceReceiptV1 {
            root_commitment_sha256: child.root_commitment.0,
            child_fingerprint_sha256: child_fingerprint.0,
            registration_commitment_sha256: child.registration_commitment.0,
            payload_commitment_sha256: child.payload_commitment.0,
            environment_sha256: super::digest_environment(&environment),
            cleanup_sha256: super::digest_cleanup(&cleanup),
            native_receipt_sha256: super::digest_native_receipt(child.actual_payload()),
            child_exit_code: child.exit_code,
        },
    ))
}

fn exercise_child(
    root: &Path,
    binding: LockRunnerAbiScalarRejectionBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created q10 Lock ABI-scalar child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;
    let fixture = ManagedSqliteRoutedConnectionFixture::open(root, [0xaa; 16])?;
    let registration_id = fixture.registration_id_for_test();
    let route_ordinal = fixture.route_ordinal().counter_value();
    if registration_id == 0 || route_ordinal == 0 {
        return Err(anyhow!(
            "q10 Lock ABI-scalar fixture registration/route identity is zero"
        ));
    }
    let journal_mode: String = fixture
        .connection()
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!(
            "q10 Lock ABI-scalar fixture did not enter WAL mode"
        ));
    }
    fixture.into_schema_migration()?;
    fixture.into_runtime()?;

    let target_before = fixture
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?;
    let route_before = fixture
        .route_custody_snapshot()
        .map_err(anyhow::Error::msg)?;
    validate_live_route(route_before)?;
    if target_before {
        return Err(anyhow!(
            "q10 Lock ABI-scalar target existed before installed xShmLock"
        ));
    }

    let (offset, count, raw_flags) = raw_tuple(binding);
    let route_request = ManagedSqliteShmLockRequest::new(
        0,
        NonZeroU8::new(1).ok_or_else(|| anyhow!("q10 canonical route count is zero"))?,
        ManagedSqliteShmLockAction::LockShared,
    )
    .map_err(anyhow::Error::msg)?;
    fixture
        .arm_pre_managed_lock_observation(
            ManagedTestPreManagedLockPath::AbiRejected,
            route_request,
        )
        .map_err(anyhow::Error::msg)?;
    let observation = fixture
        .observe_main_shm_lock_raw_with_abi_ledger(offset, count, raw_flags)
        .map_err(anyhow::Error::msg)?;
    let route_no_entry = fixture
        .finish_abi_rejected_lock_observation()
        .map_err(anyhow::Error::msg)?
        .ordered_values();

    let target_after = fixture
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?;
    let route_after = fixture
        .route_custody_snapshot()
        .map_err(anyhow::Error::msg)?;
    validate_observation(
        binding,
        &observation,
        route_no_entry,
        route_before,
        route_after,
    )?;
    if target_after {
        return Err(anyhow!(
            "q10 Lock ABI-scalar rejection unexpectedly installed a SHM target"
        ));
    }

    let autocommit = fixture.connection().is_autocommit();
    let liveness: i64 = fixture
        .connection()
        .query_row("SELECT 1", [], |row| row.get(0))?;
    let registration = fixture.live_registration_snapshot_for_test()?;
    let registration_values = [
        u64::from(registration.registered()),
        u64::from(registration.table_present()),
        u64::from(registration.name_present()),
        u64::from(registration.context_present()),
    ];
    if registration_values != [1; 4] {
        return Err(anyhow!(
            "q10 Lock ABI-scalar VFS registration did not remain exact and live"
        ));
    }
    let close = fixture.close();
    let close_succeeded = close.is_ok();
    close?;
    let terminal_values = [
        u64::from(autocommit),
        u64::from(liveness == 1),
        u64::from(close_succeeded),
        u64::from(root.is_dir()),
    ];
    if terminal_values != [1; 4] {
        return Err(anyhow!("q10 Lock ABI-scalar terminal state mismatch"));
    }

    let payload = encode_payload(
        binding,
        registration_id,
        route_ordinal,
        &observation,
        route_no_entry,
        target_before,
        target_after,
        route_before,
        route_after,
        registration_values,
        terminal_values,
    );
    let report =
        SanitizedChildReport::encode_for_current_child(&nonce, root, registration_id, &payload)
            .map_err(anyhow::Error::msg)?;
    println!("{report}");
    Ok(())
}

fn validate_live_route(
    route: ManagedSqliteTestVfsRouteCustodySnapshot,
) -> anyhow::Result<()> {
    if route.phase() != ManagedSqliteTestVfsRoutePhase::Active
        || !route.connection_owner()
        || route.main_file_lock_owner_lease()
        || route.shm_lease()
        || route.callbacks_in_flight() != 0
        || !route.access_callback_allowed()
    {
        return Err(anyhow!(
            "q10 Lock ABI-scalar route was not active and callback-free"
        ));
    }
    Ok(())
}

fn validate_observation(
    binding: LockRunnerAbiScalarRejectionBindingV1,
    observation: &ManagedTestShmLockAbiLedgerObservation,
    route_no_entry: [u64; 18],
    route_before: ManagedSqliteTestVfsRouteCustodySnapshot,
    route_after: ManagedSqliteTestVfsRouteCustodySnapshot,
) -> anyhow::Result<()> {
    let (offset, count, raw_flags) = raw_tuple(binding);
    let callback = observation.callback();
    let abi = observation.abi();
    let before = callback.before();
    let after = callback.after();
    let expected_validity = [binding.offset, binding.count, binding.flags].map(|validity| {
        matches!(validity, LockRunnerAbiScalarValidityV1::Valid)
    });
    validate_live_route(route_after)?;
    if callback.offset() != offset
        || callback.count() != count
        || callback.raw_flags() != raw_flags
        || callback.result_code() != ffi::SQLITE_IOERR_SHMLOCK
        || before != after
        || !before.methods_installed
        || !before.state_installed
        || !before.methods_exact
        || !before.state_type_exact
        || abi.observation_id() == 0
        || (abi.offset(), abi.count(), abi.flags()) != (offset, count, raw_flags)
        || abi.entry_count() != 1
        || abi.scalar_rejection_count() != 1
        || [abi.offset_valid(), abi.count_valid(), abi.flags_valid()] != expected_validity
        || abi.run_code_entry_count() != 0
        || abi.return_count() != 1
        || abi.result_code() != callback.result_code()
        || route_no_entry
            != [1, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        || route_before != route_after
    {
        return Err(anyhow!(
            "q10 Lock installed xShmLock ABI-scalar receipt mismatch"
        ));
    }
    Ok(())
}

fn raw_tuple(binding: LockRunnerAbiScalarRejectionBindingV1) -> (i32, i32, i32) {
    let offset = match binding.offset {
        LockRunnerAbiScalarValidityV1::Invalid => 256,
        LockRunnerAbiScalarValidityV1::Valid => 0,
    };
    let count = match binding.count {
        LockRunnerAbiScalarValidityV1::Invalid => 0,
        LockRunnerAbiScalarValidityV1::Valid => 1,
    };
    let flags = match binding.flags {
        LockRunnerAbiScalarValidityV1::Invalid => 0,
        LockRunnerAbiScalarValidityV1::Valid => ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_SHARED,
    };
    (offset, count, flags)
}

fn validate_binding(binding: LockRunnerAbiScalarRejectionBindingV1) -> anyhow::Result<()> {
    lock_abi_scalar_rejection::selector(
        binding.offset.tag(),
        binding.count.tag(),
        binding.flags.tag(),
    )
    .map(|_| ())
    .map_err(anyhow::Error::msg)
}

fn exact_selector(binding: LockRunnerAbiScalarRejectionBindingV1) -> String {
    lock_abi_scalar_rejection::selector(
        binding.offset.tag(),
        binding.count.tag(),
        binding.flags.tag(),
    )
    .expect("validated q10 Lock ABI-scalar selector")
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn selected_lock_abi_scalar_rejection_selector_for_test(
) -> Option<String> {
    std::env::var_os(CHILD_ROOT_ENV)?;
    std::env::var(SELECTOR_ENV).ok()
}

#[cfg(all(test, windows))]
pub(in super::super::super) fn lock_abi_scalar_rejection_selector_for_test(
    offset_tag: u64,
    count_tag: u64,
    flags_tag: u64,
) -> Result<String, &'static str> {
    lock_abi_scalar_rejection::selector(offset_tag, count_tag, flags_tag)
}
