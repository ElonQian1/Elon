//! Canonical q7 payload and independent parent validation of masked real lower evidence.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAction, ManagedSqliteShmTestLockPath, ManagedSqliteShmTestLockReceipt,
    ManagedSqliteShmTestNativeContentionReceipt, ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::connection::ManagedTestShmLockCallbackObservation;
use super::super::super::child::lock_callback_route_unknown::{REPORT_VALUE_COUNT, REPORT_VERSION};
use super::super::{lifecycle, LockRunnerActionV1};
use super::fixture::{snapshot_values, target_values};
use super::{
    exact_selector, path_tag, validate_binding, LockRunnerCallbackRouteUnknownBindingV1,
    LockRunnerCallbackRouteUnknownPathV1,
};

pub(in super::super) struct ValidatedCallbackRouteUnknownPayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    callback: ManagedTestShmLockCallbackObservation,
    selected_before: ManagedSqliteShmTestTargetSnapshot,
    selected_after: ManagedSqliteShmTestTargetSnapshot,
    sibling_before: Option<ManagedSqliteShmTestTargetSnapshot>,
    sibling_after: Option<ManagedSqliteShmTestTargetSnapshot>,
    selected_cleaned: ManagedSqliteShmTestTargetSnapshot,
    sibling_cleaned: Option<ManagedSqliteShmTestTargetSnapshot>,
    holder: Option<ManagedSqliteShmTestNativeContentionReceipt>,
    lower: ManagedSqliteShmTestLockReceipt,
    pending_count: usize,
    preemption: [u64; 6],
    terminal: [u64; 18],
    registration: [u64; 4],
    route: [u64; 3],
    root_shape_present: u64,
) -> String {
    let mut values = binding_values(binding);
    values.extend([
        registration_id,
        route_ordinal,
        runtime_generation,
        shm_connection_id,
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
    values.extend(optional_target_values(sibling_before));
    values.extend(optional_target_values(sibling_after));
    values.extend(target_values(selected_cleaned));
    values.extend(optional_target_values(sibling_cleaned));
    values.extend(holder_values(holder));
    values.extend(lower_values(lower, pending_count));
    values.extend(preemption);
    values.extend(terminal);
    values.extend(registration);
    values.extend(route);
    values.push(root_shape_present);
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
    binding: LockRunnerCallbackRouteUnknownBindingV1,
) -> anyhow::Result<ValidatedCallbackRouteUnknownPayloadV1> {
    validate_binding(binding)?;
    let mut fields = payload.split(',');
    let selector = exact_selector(binding);
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(selector.as_str()) {
        return Err(anyhow!(
            "Lock callback RouteUnknown payload identity mismatch"
        ));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[..25] != binding_values(binding) {
        return Err(anyhow!(
            "Lock callback RouteUnknown payload program binding mismatch"
        ));
    }
    if values[25] == 0
        || values[26] != 1
        || values[27] == 0
        || values[28] == 0
        || values[29..38]
            != [
                1,
                u64::from(binding.first),
                u64::from(binding.count),
                lifecycle::raw_flags(binding.action) as u64,
                ffi::SQLITE_IOERR_SHMLOCK as u64,
                1,
                1,
                1,
                1,
            ]
    {
        return Err(anyhow!(
            "Lock callback RouteUnknown payload installed-ABI binding mismatch"
        ));
    }
    let before = expected_snapshot(binding, false)?;
    let after = expected_snapshot(binding, true)?;
    let sibling_before = expected_sibling(binding, false)?;
    let sibling_after = expected_sibling(binding, true)?;
    let selected_cleaned = [after[0], after[1], after[2]];
    let sibling_cleaned = if binding.path.connection_count() == 2 {
        [1, 0, 0]
    } else {
        [0; 3]
    };
    if values[38..52] != before
        || values[52..66] != after
        || values[66..69] != sibling_before
        || values[69..72] != sibling_after
        || values[72..75] != selected_cleaned
        || values[75..78] != sibling_cleaned
        || values[78..90] != expected_holder(binding, values[27], values[28])
        || values[90..109] != expected_lower(binding, values[27], values[28])
        || values[109..115] != [1; 6]
        || values[115..133] != [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0]
        || values[133..137] != [1; 4]
        || values[137..140]
            != if binding.path.connection_count() == 1 {
                [1, 1, 3]
            } else {
                [2, 2, 6]
            }
        || values[140] != 1
    {
        return Err(anyhow!(
            "Lock callback RouteUnknown payload real lower/terminal receipt mismatch"
        ));
    }
    Ok(ValidatedCallbackRouteUnknownPayloadV1 {
        registration_id: values[25],
        native_receipt_sha256: digest_native_receipt(&values),
    })
}

fn binding_values(binding: LockRunnerCallbackRouteUnknownBindingV1) -> Vec<u64> {
    let mut values = vec![
        path_tag(binding.path),
        lifecycle::action_tag(binding.action),
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

fn expected_snapshot(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    after: bool,
) -> anyhow::Result<[u64; 14]> {
    let (shared, exclusive, _, _) = expected_masks(binding, after)?;
    Ok(live_snapshot(
        binding.path.connection_count(),
        shared,
        exclusive,
    ))
}

fn expected_sibling(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    after: bool,
) -> anyhow::Result<[u64; 3]> {
    if binding.path.connection_count() == 1 {
        return Ok([0; 3]);
    }
    let (_, _, shared, exclusive) = expected_masks(binding, after)?;
    Ok([1, u64::from(shared), u64::from(exclusive)])
}

fn expected_masks(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    after: bool,
) -> anyhow::Result<(u8, u8, u8, u8)> {
    use LockRunnerCallbackRouteUnknownPathV1 as Path;
    Ok(match binding.path {
        Path::NativeAcquireAcquired if after => match binding.action {
            LockRunnerActionV1::LockShared => (binding.mask, 0, 0, 0),
            LockRunnerActionV1::LockExclusive => (0, binding.mask, 0, 0),
            _ => return Err(anyhow!("native-acquire action mismatch")),
        },
        Path::NativeAcquireAcquired | Path::NativeAcquireBusy => (0, 0, 0, 0),
        Path::NativeRelease if !after => match binding.action {
            LockRunnerActionV1::UnlockShared => (binding.mask, 0, 0, 0),
            LockRunnerActionV1::UnlockExclusive => (0, binding.mask, 0, 0),
            _ => return Err(anyhow!("native-release action mismatch")),
        },
        Path::NativeRelease => (0, 0, 0, 0),
        Path::SharedLocalAcquire if after => (binding.mask, 0, binding.mask, 0),
        Path::SharedLocalAcquire => (0, 0, binding.mask, 0),
        Path::SharedLocalRelease if after => (0, 0, binding.mask, 0),
        Path::SharedLocalRelease => (binding.mask, 0, binding.mask, 0),
        Path::LocalSiblingContention => match binding.action {
            LockRunnerActionV1::LockShared => (0, 0, 0, binding.mask),
            LockRunnerActionV1::LockExclusive => (0, 0, binding.mask, 0),
            _ => return Err(anyhow!("local sibling-contention action mismatch")),
        },
    })
}

fn live_snapshot(connection_count: u8, shared: u8, exclusive: u8) -> [u64; 14] {
    [
        1,
        u64::from(shared),
        u64::from(exclusive),
        u64::from(connection_count),
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

fn expected_holder(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    runtime_generation: u64,
    shm_connection_id: u64,
) -> [u64; 12] {
    if binding.path != LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy {
        return [0; 12];
    }
    [
        runtime_generation,
        shm_connection_id,
        120 + u64::from(binding.first),
        u64::from(binding.count),
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
    ]
}

fn expected_lower(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    runtime_generation: u64,
    shm_connection_id: u64,
) -> [u64; 19] {
    let path = binding.path;
    let native_acquired = path == LockRunnerCallbackRouteUnknownPathV1::NativeAcquireAcquired;
    let native_busy = path == LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy;
    let native_release = path == LockRunnerCallbackRouteUnknownPathV1::NativeRelease;
    let local = matches!(
        path,
        LockRunnerCallbackRouteUnknownPathV1::SharedLocalAcquire
            | LockRunnerCallbackRouteUnknownPathV1::SharedLocalRelease
    );
    [
        runtime_generation,
        shm_connection_id,
        lifecycle::action_tag(binding.action),
        u64::from(binding.first),
        u64::from(binding.count),
        u64::from(binding.mask),
        lower_path_tag(path),
        1,
        u64::from(!path.is_contended()),
        u64::from(native_acquired || native_busy),
        u64::from(native_acquired),
        u64::from(native_busy),
        0,
        u64::from(native_release),
        u64::from(native_release),
        0,
        u64::from(local),
        0,
        1,
    ]
}

fn optional_target_values(value: Option<ManagedSqliteShmTestTargetSnapshot>) -> [u64; 3] {
    value.map(target_values).unwrap_or([0; 3])
}

fn holder_values(value: Option<ManagedSqliteShmTestNativeContentionReceipt>) -> [u64; 12] {
    value.map_or([0; 12], |value| {
        [
            value.runtime_generation,
            value.shm_connection_id,
            value.absolute_offset,
            value.length,
            u64::from(value.target_identity_verified),
            u64::from(value.holder_identity_verified),
            u64::from(value.distinct_handle),
            u64::from(value.exclusive_holder),
            u64::from(value.acquire_attempts),
            u64::from(value.acquired),
            u64::from(value.held_during_callback),
            u64::from(value.released),
        ]
    })
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
        ManagedSqliteShmTestLockPath::SiblingContention => 4,
        ManagedSqliteShmTestLockPath::LocalProtocolRejection => 5,
    }
}

fn lower_path_tag(value: LockRunnerCallbackRouteUnknownPathV1) -> u64 {
    match value {
        LockRunnerCallbackRouteUnknownPathV1::NativeAcquireAcquired
        | LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy => 1,
        LockRunnerCallbackRouteUnknownPathV1::NativeRelease => 2,
        LockRunnerCallbackRouteUnknownPathV1::SharedLocalAcquire
        | LockRunnerCallbackRouteUnknownPathV1::SharedLocalRelease => 3,
        LockRunnerCallbackRouteUnknownPathV1::LocalSiblingContention => 4,
    }
}

fn digest_native_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-callback-route-unknown-receipt-v1\0");
    for value in &values[25..] {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!(
            "Lock callback RouteUnknown payload scalar is not canonical"
        ));
    }
    Ok(parsed)
}
