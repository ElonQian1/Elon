//! Canonical q6 payload and independent parent-side sibling-contention validation.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAction, ManagedSqliteShmTestLockPath, ManagedSqliteShmTestLockReceipt,
    ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::connection::ManagedTestShmLockCallbackObservation;
use super::super::super::child::lock_local_sibling_contention::{
    REPORT_VALUE_COUNT, REPORT_VERSION,
};
use super::fixture::{raw_flags, sibling_values, snapshot_values};
use super::{action_tag, exact_selector, LockRunnerLocalSiblingContentionBindingV1};

const SIBLING_CONTENTION_PATH_TAG: u64 = 4;

pub(in super::super) struct ValidatedLocalSiblingContentionPayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: LockRunnerLocalSiblingContentionBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    callback: ManagedTestShmLockCallbackObservation,
    selected_before: ManagedSqliteShmTestTargetSnapshot,
    selected_after: ManagedSqliteShmTestTargetSnapshot,
    sibling_before: ManagedSqliteShmTestTargetSnapshot,
    sibling_after: ManagedSqliteShmTestTargetSnapshot,
    selected_cleaned: ManagedSqliteShmTestTargetSnapshot,
    sibling_cleaned: ManagedSqliteShmTestTargetSnapshot,
    lower: ManagedSqliteShmTestLockReceipt,
    pending_count: usize,
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
        action_tag(binding.action),
        u64::from(binding.first),
        u64::from(binding.count),
        u64::from(binding.mask),
    ]);
    values.extend([
        1,
        callback.offset() as u64,
        callback.count() as u64,
        callback.raw_flags() as u64,
        callback.result_code() as u64,
        u64::from(callback.before().methods_installed),
        u64::from(callback.before().state_installed),
        u64::from(callback.after().methods_installed),
        u64::from(callback.after().state_installed),
    ]);
    values.extend(snapshot_values(selected_before));
    values.extend(snapshot_values(selected_after));
    values.extend(sibling_values(sibling_before));
    values.extend(sibling_values(sibling_after));
    values.extend(sibling_values(selected_cleaned));
    values.extend(sibling_values(sibling_cleaned));
    values.extend(lower_values(lower, pending_count));
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
    binding: LockRunnerLocalSiblingContentionBindingV1,
) -> anyhow::Result<ValidatedLocalSiblingContentionPayloadV1> {
    super::validate_binding(binding)?;
    let mut fields = payload.split(',');
    let selector = exact_selector(binding);
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(selector.as_str()) {
        return Err(anyhow!(
            "Lock local sibling-contention payload identity mismatch"
        ));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[..22] != binding_values(binding) {
        return Err(anyhow!(
            "Lock local sibling-contention payload program binding mismatch"
        ));
    }
    let identity = [
        values[22],
        values[23],
        values[24],
        values[25],
        action_tag(binding.action),
        u64::from(binding.first),
        u64::from(binding.count),
        u64::from(binding.mask),
    ];
    if values[22] == 0
        || values[23] != 1
        || values[24] == 0
        || values[25] == 0
        || values[22..30] != identity
        || values[30..39]
            != [
                1,
                u64::from(binding.first),
                u64::from(binding.count),
                raw_flags(binding.action) as u64,
                ffi::SQLITE_BUSY as u64,
                1,
                1,
                1,
                1,
            ]
    {
        return Err(anyhow!(
            "Lock local sibling-contention payload installed-ABI binding mismatch"
        ));
    }
    let sibling = expected_sibling(binding);
    if values[39..53] != expected_selected_snapshot()
        || values[53..67] != expected_selected_snapshot()
        || values[67..70] != sibling
        || values[70..73] != sibling
        || values[73..76] != [1, 0, 0]
        || values[76..79] != [1, 0, 0]
        || values[79..98] != expected_lower(binding, values[24], values[25])
        || values[98..102] != [1, 1, 1, 1]
        || values[102..105] != [2, 2, 6]
        || values[105..109] != [1, 1, 1, 1]
    {
        return Err(anyhow!(
            "Lock local sibling-contention payload native receipt mismatch"
        ));
    }
    Ok(ValidatedLocalSiblingContentionPayloadV1 {
        registration_id: values[22],
        native_receipt_sha256: digest_native_receipt(&values),
    })
}

fn binding_values(binding: LockRunnerLocalSiblingContentionBindingV1) -> Vec<u64> {
    let mut values = vec![action_tag(binding.action), SIBLING_CONTENTION_PATH_TAG];
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

fn expected_selected_snapshot() -> [u64; 14] {
    [1, 0, 0, 2, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0]
}

fn expected_sibling(binding: LockRunnerLocalSiblingContentionBindingV1) -> [u64; 3] {
    match binding.action {
        super::LockRunnerActionV1::LockShared => [1, 0, u64::from(binding.mask)],
        super::LockRunnerActionV1::LockExclusive => [1, u64::from(binding.mask), 0],
        _ => [0; 3],
    }
}

fn expected_lower(
    binding: LockRunnerLocalSiblingContentionBindingV1,
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
        SIBLING_CONTENTION_PATH_TAG,
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

fn managed_path_tag(value: ManagedSqliteShmTestLockPath) -> u64 {
    match value {
        ManagedSqliteShmTestLockPath::NativeAcquire => 1,
        ManagedSqliteShmTestLockPath::NativeRelease => 2,
        ManagedSqliteShmTestLockPath::Local => 3,
        ManagedSqliteShmTestLockPath::SiblingContention => SIBLING_CONTENTION_PATH_TAG,
        ManagedSqliteShmTestLockPath::LocalProtocolRejection => 5,
    }
}

fn digest_native_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-local-sibling-contention-receipt-v1\0");
    for value in &values[22..98] {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!(
            "Lock local sibling-contention payload scalar is not canonical"
        ));
    }
    Ok(parsed)
}
