//! Exact installed-ABI inputs and native receipts for Lock managed-request rejection.

use std::path::Path;

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use super::super::super::{
    connection::ManagedTestShmLockCallbackObservation, ManagedSqliteRoutedConnectionFixture,
};
use super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};
use super::{LockRunnerProgramBindingV1, PAYLOAD_VERSION};

pub(super) const PAYLOAD_VALUE_COUNT: usize = 51;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum LockRunnerActionV1 {
    LockShared,
    LockExclusive,
    UnlockShared,
    UnlockExclusive,
}

impl LockRunnerActionV1 {
    const fn tag(self) -> u64 {
        match self {
            Self::LockShared => 1,
            Self::LockExclusive => 2,
            Self::UnlockShared => 3,
            Self::UnlockExclusive => 4,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::LockShared => "lock-shared",
            Self::LockExclusive => "lock-exclusive",
            Self::UnlockShared => "unlock-shared",
            Self::UnlockExclusive => "unlock-exclusive",
        }
    }

    const fn raw_flags(self) -> i32 {
        match self {
            Self::LockShared => ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_SHARED,
            Self::LockExclusive => ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_EXCLUSIVE,
            Self::UnlockShared => ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_SHARED,
            Self::UnlockExclusive => ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_EXCLUSIVE,
        }
    }

    const fn is_shared(self) -> bool {
        matches!(self, Self::LockShared | Self::UnlockShared)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum LockRunnerRequestValidationV1 {
    RangeOverflow,
    EndPastEight,
    SharedMultiSlot,
}

impl LockRunnerRequestValidationV1 {
    const fn tag(self) -> u64 {
        match self {
            Self::RangeOverflow => 1,
            Self::EndPastEight => 2,
            Self::SharedMultiSlot => 3,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::RangeOverflow => "range-overflow",
            Self::EndPastEight => "end-past-eight",
            Self::SharedMultiSlot => "shared-multi-slot",
        }
    }

    const fn offset(self) -> i32 {
        match self {
            Self::RangeOverflow => 255,
            Self::EndPastEight => 8,
            Self::SharedMultiSlot => 0,
        }
    }

    const fn count(self) -> i32 {
        match self {
            Self::RangeOverflow | Self::EndPastEight => 1,
            Self::SharedMultiSlot => 2,
        }
    }

    const fn supports(self, action: LockRunnerActionV1) -> bool {
        !matches!(self, Self::SharedMultiSlot) || action.is_shared()
    }

    fn selector(self, action: LockRunnerActionV1) -> String {
        format!("{}-{}-completed", self.label(), action.label())
    }
}

pub(super) fn validate_binding(binding: LockRunnerProgramBindingV1) -> anyhow::Result<()> {
    if !binding.request_validation.supports(binding.action) {
        return Err(anyhow!(
            "Lock quotient shared-multi-slot selected a non-shared action"
        ));
    }
    Ok(())
}

pub(super) fn exercise_child(
    root: &Path,
    binding: LockRunnerProgramBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created Lock quotient child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;
    let fixture = ManagedSqliteRoutedConnectionFixture::open(root, [0xa3; 16])?;
    let registration_id = fixture.registration_id_for_test();
    if registration_id == 0 {
        return Err(anyhow!(
            "Lock quotient fixture registration identity is zero"
        ));
    }
    let journal_mode: String =
        fixture
            .connection()
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!("Lock quotient fixture did not enter WAL mode"));
    }
    fixture.into_schema_migration()?;
    fixture.into_runtime()?;
    if fixture
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?
    {
        return Err(anyhow!(
            "Lock quotient target existed before the rejected xShmLock request"
        ));
    }
    let observation = fixture
        .observe_main_shm_lock_raw(
            binding.request_validation.offset(),
            binding.request_validation.count(),
            binding.action.raw_flags(),
        )
        .map_err(anyhow::Error::msg)?;
    if fixture
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?
    {
        return Err(anyhow!(
            "Lock quotient rejected xShmLock request unexpectedly installed a SHM target"
        ));
    }
    validate_native_lock(binding, observation)?;
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
    if registration_values != [1, 1, 1, 1] {
        return Err(anyhow!(
            "Lock quotient VFS registration did not remain exact and live"
        ));
    }
    fixture.close()?;
    if !autocommit || liveness != 1 || !root.is_dir() {
        return Err(anyhow!("Lock quotient fixture terminal state mismatch"));
    }
    let payload = encode_payload(binding, registration_id, observation, registration_values);
    let report =
        SanitizedChildReport::encode_for_current_child(&nonce, root, registration_id, &payload)
            .map_err(anyhow::Error::msg)?;
    println!("{report}");
    Ok(())
}

fn validate_native_lock(
    binding: LockRunnerProgramBindingV1,
    observation: ManagedTestShmLockCallbackObservation,
) -> anyhow::Result<()> {
    let before = observation.before();
    let after = observation.after();
    if observation.offset() != binding.request_validation.offset()
        || observation.count() != binding.request_validation.count()
        || observation.raw_flags() != binding.action.raw_flags()
        || observation.result_code() != ffi::SQLITE_IOERR_SHMLOCK
        || !before.methods_installed
        || !before.state_installed
        || !after.methods_installed
        || !after.state_installed
    {
        return Err(anyhow!(
            "Lock quotient native xShmLock request-rejection receipt mismatch"
        ));
    }
    Ok(())
}

fn encode_payload(
    binding: LockRunnerProgramBindingV1,
    registration_id: u64,
    observation: ManagedTestShmLockCallbackObservation,
    registration_values: [u64; 4],
) -> String {
    let mut values = binding_values(binding);
    values.extend([
        registration_id,
        observation.offset() as u64,
        observation.count() as u64,
        observation.raw_flags() as u64,
        observation.result_code() as u64,
        u64::from(observation.before().methods_installed),
        u64::from(observation.before().state_installed),
        u64::from(observation.after().methods_installed),
        u64::from(observation.after().state_installed),
    ]);
    // The installed ABI entry/return and request rejection are reached exactly once. The managed
    // callback ledger, native lock/unlock and selected/sibling snapshots remain unreachable.
    values.extend([1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    // These four fields come from sqlite3_vfs_find and the live registration owner's retained
    // table/name/context custody after the rejected xShmLock call.
    values.extend(registration_values);
    // Autocommit, SQL liveness, fixture close and child-root presence are independently observed.
    values.extend([1, 1, 1, 1]);
    debug_assert_eq!(values.len(), PAYLOAD_VALUE_COUNT);
    format!(
        "{PAYLOAD_VERSION},{},{}",
        binding.request_validation.selector(binding.action),
        values
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(super) fn validate_payload(
    payload: &str,
    binding: LockRunnerProgramBindingV1,
) -> anyhow::Result<u64> {
    validate_binding(binding)?;
    let mut fields = payload.split(',');
    let selector = binding.request_validation.selector(binding.action);
    if fields.next() != Some(PAYLOAD_VERSION) || fields.next() != Some(selector.as_str()) {
        return Err(anyhow!("Lock quotient payload identity mismatch"));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != PAYLOAD_VALUE_COUNT || values[..22] != binding_values(binding) {
        return Err(anyhow!("Lock quotient payload program binding mismatch"));
    }
    let expected_request = [
        binding.request_validation.offset() as u64,
        binding.request_validation.count() as u64,
        binding.action.raw_flags() as u64,
        ffi::SQLITE_IOERR_SHMLOCK as u64,
    ];
    if values[22] == 0
        || values[23..27] != expected_request
        || values[27..31] != [1, 1, 1, 1]
        || values[31..43] != [1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        || values[43..47] != [1, 1, 1, 1]
        || values[47..51] != [1, 1, 1, 1]
    {
        return Err(anyhow!("Lock quotient payload native receipt mismatch"));
    }
    Ok(values[22])
}

fn binding_values(binding: LockRunnerProgramBindingV1) -> Vec<u64> {
    let mut values = vec![binding.action.tag(), binding.request_validation.tag()];
    for digest in [
        binding.normalized_descriptor_sha256,
        binding.case_key_sha256,
        binding.full_record_sha256,
        binding.plan_sha256,
        binding.implementation_sha256,
    ] {
        for chunk in digest.chunks_exact(8) {
            values.push(u64::from_le_bytes(chunk.try_into().expect("digest chunk")));
        }
    }
    values
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!("Lock quotient payload scalar is not canonical"));
    }
    Ok(parsed)
}
