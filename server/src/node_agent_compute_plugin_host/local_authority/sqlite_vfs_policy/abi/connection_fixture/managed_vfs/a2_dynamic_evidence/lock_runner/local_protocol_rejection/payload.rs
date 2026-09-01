//! Canonical q8 payload and independent parent validation of real local protocol rejections.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAction, ManagedSqliteShmTestLockPath, ManagedSqliteShmTestLockReceipt,
    ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::connection::ManagedTestShmLockCallbackObservation;
use super::super::super::child::lock_local_protocol_rejection::{
    REPORT_VALUE_COUNT, REPORT_VERSION,
};
use super::super::LockRunnerActionV1;
use super::fixture::snapshot_values;
use super::{
    exact_selector, path_tag, validate_binding, LocalProtocolRejectionPathV1,
    LockRunnerLocalProtocolRejectionBindingV1,
};

const LOCAL_PROTOCOL_REJECTION_PATH_TAG: u64 = 5;

pub(in super::super) struct ValidatedLocalProtocolRejectionPayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: LockRunnerLocalProtocolRejectionBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    setup: Option<ManagedTestShmLockCallbackObservation>,
    callback: ManagedTestShmLockCallbackObservation,
    before: ManagedSqliteShmTestTargetSnapshot,
    after: ManagedSqliteShmTestTargetSnapshot,
    lower: ManagedSqliteShmTestLockReceipt,
    pending_count: usize,
    active_route: [u64; 6],
    cleanup: Option<ManagedTestShmLockCallbackObservation>,
    cleaned: ManagedSqliteShmTestTargetSnapshot,
    registration: [u64; 4],
    route: [u64; 3],
    terminal: [u64; 4],
) -> String {
    let mut values = binding_values(binding);
    values.extend([
        registration_id,
        route_ordinal,
        runtime_generation,
        shm_connection_id,
    ]);
    values.extend(optional_callback_values(setup));
    values.extend(callback_values(callback));
    values.extend(snapshot_values(before));
    values.extend(snapshot_values(after));
    values.extend(lower_values(lower, pending_count));
    values.extend(active_route);
    values.extend(optional_callback_values(cleanup));
    values.extend(snapshot_values(cleaned));
    values.extend(registration);
    values.extend(route);
    values.extend(terminal);
    debug_assert_eq!(values.len(), REPORT_VALUE_COUNT);
    format!(
        "{REPORT_VERSION},{},{}",
        exact_selector(binding),
        values
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(in super::super) fn validate_payload(
    payload: &str,
    binding: LockRunnerLocalProtocolRejectionBindingV1,
) -> anyhow::Result<ValidatedLocalProtocolRejectionPayloadV1> {
    validate_binding(binding)?;
    let mut fields = payload.split(',');
    let selector = exact_selector(binding);
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(selector.as_str()) {
        return Err(anyhow!(
            "Lock local protocol-rejection payload identity mismatch"
        ));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[..25] != binding_values(binding) {
        return Err(anyhow!(
            "Lock local protocol-rejection payload program binding mismatch"
        ));
    }
    if values[25] == 0
        || values[26] != 1
        || values[27] == 0
        || values[28] == 0
        || values[29..38] != expected_setup(binding)
        || values[38..47] != expected_target_callback(binding)
    {
        return Err(anyhow!(
            "Lock local protocol-rejection installed-ABI binding mismatch"
        ));
    }
    let snapshot = expected_live_snapshot(binding);
    if values[47..61] != snapshot
        || values[61..75] != snapshot
        || values[75..94] != expected_lower(binding, values[27], values[28])
        || values[94..100] != [3, 1, 1, 1, 0, 1]
        || values[100..109] != expected_cleanup(binding)
        || values[109..123] != clean_live_snapshot()
        || values[123..127] != [1; 4]
        || values[127..130] != [1, 1, 3]
        || values[130..134] != [1; 4]
    {
        return Err(anyhow!(
            "Lock local protocol-rejection real receipt mismatch"
        ));
    }
    Ok(ValidatedLocalProtocolRejectionPayloadV1 {
        registration_id: values[25],
        native_receipt_sha256: digest_native_receipt(&values),
    })
}

fn binding_values(binding: LockRunnerLocalProtocolRejectionBindingV1) -> Vec<u64> {
    let mut values = vec![
        path_tag(binding.path),
        action_tag(binding.action),
        u64::from(binding.first),
        u64::from(binding.count),
        u64::from(binding.mask),
    ];
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

fn expected_setup(binding: LockRunnerLocalProtocolRejectionBindingV1) -> [u64; 9] {
    if binding.path == LocalProtocolRejectionPathV1::NotHeld {
        return [0; 9];
    }
    expected_callback(binding.action, binding.first, binding.count, ffi::SQLITE_OK)
}

fn expected_target_callback(binding: LockRunnerLocalProtocolRejectionBindingV1) -> [u64; 9] {
    expected_callback(
        binding.action,
        binding.first,
        binding.count,
        ffi::SQLITE_IOERR_SHMLOCK,
    )
}

fn expected_cleanup(binding: LockRunnerLocalProtocolRejectionBindingV1) -> [u64; 9] {
    if binding.path == LocalProtocolRejectionPathV1::NotHeld {
        return [0; 9];
    }
    expected_callback(
        cleanup_action(binding.action),
        binding.first,
        binding.count,
        ffi::SQLITE_OK,
    )
}

fn expected_callback(action: LockRunnerActionV1, first: u8, count: u8, result: i32) -> [u64; 9] {
    [
        1,
        u64::from(first),
        u64::from(count),
        raw_flags(action) as u64,
        result as u64,
        1,
        1,
        1,
        1,
    ]
}

fn expected_live_snapshot(binding: LockRunnerLocalProtocolRejectionBindingV1) -> [u64; 14] {
    let (shared, exclusive) = match (binding.path, binding.action) {
        (LocalProtocolRejectionPathV1::OwnOverlap, LockRunnerActionV1::LockShared) => {
            (binding.mask, 0)
        }
        (LocalProtocolRejectionPathV1::OwnOverlap, LockRunnerActionV1::LockExclusive) => {
            (0, binding.mask)
        }
        (LocalProtocolRejectionPathV1::NotHeld, _) => (0, 0),
        _ => (0, 0),
    };
    [
        1,
        u64::from(shared),
        u64::from(exclusive),
        1,
        1,
        1,
        1,
        1,
        1,
        0,
        0,
        0,
        0,
        0,
    ]
}

fn clean_live_snapshot() -> [u64; 14] {
    [1, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0]
}

fn expected_lower(
    binding: LockRunnerLocalProtocolRejectionBindingV1,
    runtime_generation: u64,
    shm_connection_id: u64,
) -> [u64; 19] {
    [
        runtime_generation,
        shm_connection_id,
        action_tag(binding.action),
        u64::from(binding.first),
        u64::from(binding.count),
        u64::from(binding.mask),
        LOCAL_PROTOCOL_REJECTION_PATH_TAG,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        1,
    ]
}

fn callback_values(value: ManagedTestShmLockCallbackObservation) -> [u64; 9] {
    [
        1,
        value.offset() as u64,
        value.count() as u64,
        value.raw_flags() as u64,
        value.result_code() as u64,
        u64::from(value.before().methods_installed),
        u64::from(value.before().state_installed),
        u64::from(value.after().methods_installed),
        u64::from(value.after().state_installed),
    ]
}

fn optional_callback_values(value: Option<ManagedTestShmLockCallbackObservation>) -> [u64; 9] {
    value.map(callback_values).unwrap_or([0; 9])
}

fn lower_values(value: ManagedSqliteShmTestLockReceipt, pending_count: usize) -> [u64; 19] {
    [
        value.runtime_generation,
        value.shm_connection_id,
        managed_action_tag(value.expectation.action),
        u64::from(value.expectation.first),
        u64::from(value.expectation.count),
        u64::from(value.expectation.mask),
        managed_path_tag(value.expectation.path),
        u64::from(value.managed_attempts),
        u64::from(value.managed_successes),
        u64::from(value.native_lock_attempts),
        u64::from(value.native_lock_acquired),
        u64::from(value.native_lock_contended),
        u64::from(value.native_lock_errors),
        u64::from(value.native_unlock_attempts),
        u64::from(value.native_unlock_successes),
        u64::from(value.native_unlock_errors),
        u64::from(value.local_transitions),
        pending_count as u64,
        u64::from(value.finished),
    ]
}

fn managed_action_tag(value: ManagedSqliteShmLockAction) -> u64 {
    match value {
        ManagedSqliteShmLockAction::LockShared => 1,
        ManagedSqliteShmLockAction::LockExclusive => 2,
        ManagedSqliteShmLockAction::UnlockShared => 3,
        ManagedSqliteShmLockAction::UnlockExclusive => 4,
    }
}

fn action_tag(value: LockRunnerActionV1) -> u64 {
    match value {
        LockRunnerActionV1::LockShared => 1,
        LockRunnerActionV1::LockExclusive => 2,
        LockRunnerActionV1::UnlockShared => 3,
        LockRunnerActionV1::UnlockExclusive => 4,
    }
}

fn raw_flags(value: LockRunnerActionV1) -> i32 {
    match value {
        LockRunnerActionV1::LockShared => ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_SHARED,
        LockRunnerActionV1::LockExclusive => ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_EXCLUSIVE,
        LockRunnerActionV1::UnlockShared => ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_SHARED,
        LockRunnerActionV1::UnlockExclusive => ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_EXCLUSIVE,
    }
}

fn cleanup_action(value: LockRunnerActionV1) -> LockRunnerActionV1 {
    match value {
        LockRunnerActionV1::LockShared => LockRunnerActionV1::UnlockShared,
        LockRunnerActionV1::LockExclusive => LockRunnerActionV1::UnlockExclusive,
        LockRunnerActionV1::UnlockShared | LockRunnerActionV1::UnlockExclusive => value,
    }
}

fn managed_path_tag(value: ManagedSqliteShmTestLockPath) -> u64 {
    match value {
        ManagedSqliteShmTestLockPath::NativeAcquire => 1,
        ManagedSqliteShmTestLockPath::NativeRelease => 2,
        ManagedSqliteShmTestLockPath::Local => 3,
        ManagedSqliteShmTestLockPath::SiblingContention => 4,
        ManagedSqliteShmTestLockPath::LocalProtocolRejection => LOCAL_PROTOCOL_REJECTION_PATH_TAG,
    }
}

fn digest_native_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-local-protocol-rejection-receipt-v1\0");
    for value in &values[25..] {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!(
            "Lock local protocol-rejection payload scalar is not canonical"
        ));
    }
    Ok(parsed)
}
