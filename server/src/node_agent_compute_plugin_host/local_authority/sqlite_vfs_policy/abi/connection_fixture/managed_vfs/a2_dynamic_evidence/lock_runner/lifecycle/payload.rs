//! Canonical q2 payload encoding and independent parent-side Lock receipt validation.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAction, ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestLockPath,
    ManagedSqliteShmTestLockReceipt, ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::connection::ManagedTestShmLockCallbackObservation;
use super::super::super::child::lock_lifecycle::{selector, REPORT_VALUE_COUNT, REPORT_VERSION};
use super::fixture::{dms_tag, snapshot_values};
use super::{
    action_tag, path_tag, raw_flags, LockRunnerActionV1, LockRunnerLifecycleBindingV1,
    LockRunnerLifecyclePathV1,
};

pub(in super::super) struct ValidatedLifecyclePayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: LockRunnerLifecycleBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    callback: ManagedTestShmLockCallbackObservation,
    selected_before: ManagedSqliteShmTestTargetSnapshot,
    selected_after: ManagedSqliteShmTestTargetSnapshot,
    sibling_before: [u64; 3],
    sibling_after: [u64; 3],
    receipt: ManagedSqliteShmTestLockReceipt,
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
    values.extend(sibling_before);
    values.extend(sibling_after);
    values.extend(lock_receipt_values(receipt, pending_count));
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
    binding: LockRunnerLifecycleBindingV1,
) -> anyhow::Result<ValidatedLifecyclePayloadV1> {
    super::validate_binding(binding)?;
    let mut fields = payload.split(',');
    let exact_selector = exact_selector(binding);
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(exact_selector.as_str()) {
        return Err(anyhow!("Lock lifecycle payload identity mismatch"));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[..22] != binding_values(binding) {
        return Err(anyhow!("Lock lifecycle payload program binding mismatch"));
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
        || values[23] == 0
        || values[24] == 0
        || values[25] == 0
        || values[22..30] != identity
        || values[30..39]
            != [
                1,
                u64::from(binding.first),
                u64::from(binding.count),
                raw_flags(binding.action) as u64,
                ffi::SQLITE_OK as u64,
                1,
                1,
                1,
                1,
            ]
    {
        return Err(anyhow!(
            "Lock lifecycle payload installed-ABI binding mismatch"
        ));
    }
    let selected_before = expected_selected_values(binding, false);
    let selected_after = expected_selected_values(binding, true);
    let sibling = if binding.path.is_local() {
        [1, u64::from(binding.mask), 0]
    } else {
        [0; 3]
    };
    if values[39..53] != selected_before
        || values[53..67] != selected_after
        || values[67..70] != sibling
        || values[70..73] != sibling
        || values[73..92] != expected_lock_receipt_values(binding, values[24], values[25])
        || values[92..96] != [1, 1, 1, 1]
        || values[96..99]
            != if binding.path.is_local() {
                [2, 2, 6]
            } else {
                [1, 1, 3]
            }
        || values[99..103] != [1, 1, 1, 1]
    {
        return Err(anyhow!("Lock lifecycle payload native receipt mismatch"));
    }
    Ok(ValidatedLifecyclePayloadV1 {
        registration_id: values[22],
        native_receipt_sha256: digest_native_receipt(&values),
    })
}

fn expected_selected_values(binding: LockRunnerLifecycleBindingV1, after: bool) -> [u64; 14] {
    let (shared, exclusive) = match (binding.path, after) {
        (LockRunnerLifecyclePathV1::NativeAcquire, false)
        | (LockRunnerLifecyclePathV1::NativeRelease, true)
        | (LockRunnerLifecyclePathV1::SharedLocalAcquire, false)
        | (LockRunnerLifecyclePathV1::SharedLocalRelease, true) => (0, 0),
        (LockRunnerLifecyclePathV1::NativeAcquire, true) => match binding.action {
            LockRunnerActionV1::LockShared => (binding.mask, 0),
            LockRunnerActionV1::LockExclusive => (0, binding.mask),
            _ => (0, 0),
        },
        (LockRunnerLifecyclePathV1::NativeRelease, false) => match binding.action {
            LockRunnerActionV1::UnlockShared => (binding.mask, 0),
            LockRunnerActionV1::UnlockExclusive => (0, binding.mask),
            _ => (0, 0),
        },
        (LockRunnerLifecyclePathV1::SharedLocalAcquire, true)
        | (LockRunnerLifecyclePathV1::SharedLocalRelease, false) => (binding.mask, 0),
    };
    [
        1,
        u64::from(shared),
        u64::from(exclusive),
        u64::from(binding.path.connection_count()),
        1,
        1,
        1,
        dms_tag(ManagedSqliteShmTestDmsCustody::Shared),
        1,
        0,
        0,
        0,
        0,
        0,
    ]
}

fn expected_lock_receipt_values(
    binding: LockRunnerLifecycleBindingV1,
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
        ledger_path_tag(binding.path),
        1,
        1,
        u64::from(binding.path == LockRunnerLifecyclePathV1::NativeAcquire),
        u64::from(binding.path == LockRunnerLifecyclePathV1::NativeAcquire),
        0,
        0,
        u64::from(binding.path == LockRunnerLifecyclePathV1::NativeRelease),
        u64::from(binding.path == LockRunnerLifecyclePathV1::NativeRelease),
        0,
        u64::from(binding.path.is_local()),
        0,
        1,
    ]
}

fn binding_values(binding: LockRunnerLifecycleBindingV1) -> Vec<u64> {
    let mut values = vec![action_tag(binding.action), path_tag(binding.path)];
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

fn lock_receipt_values(value: ManagedSqliteShmTestLockReceipt, pending_count: usize) -> [u64; 19] {
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

fn exact_selector(binding: LockRunnerLifecycleBindingV1) -> String {
    selector(
        action_tag(binding.action),
        path_tag(binding.path),
        binding.first,
        binding.count,
    )
    .expect("validated Lock lifecycle selector")
}

fn managed_action_tag(action: ManagedSqliteShmLockAction) -> u64 {
    match action {
        ManagedSqliteShmLockAction::LockShared => 1,
        ManagedSqliteShmLockAction::LockExclusive => 2,
        ManagedSqliteShmLockAction::UnlockShared => 3,
        ManagedSqliteShmLockAction::UnlockExclusive => 4,
    }
}

fn ledger_path_tag(path: LockRunnerLifecyclePathV1) -> u64 {
    match path {
        LockRunnerLifecyclePathV1::NativeAcquire => 1,
        LockRunnerLifecyclePathV1::NativeRelease => 2,
        LockRunnerLifecyclePathV1::SharedLocalAcquire
        | LockRunnerLifecyclePathV1::SharedLocalRelease => 3,
    }
}

fn managed_path_tag(path: ManagedSqliteShmTestLockPath) -> u64 {
    match path {
        ManagedSqliteShmTestLockPath::NativeAcquire => 1,
        ManagedSqliteShmTestLockPath::NativeRelease => 2,
        ManagedSqliteShmTestLockPath::Local => 3,
        ManagedSqliteShmTestLockPath::SiblingContention => 4,
    }
}

fn digest_native_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-lifecycle-native-receipt-v2\0");
    // Bind exact target identity, installed ABI, before/after snapshots and the lower ledger. The
    // registration/route/terminal sections remain separately bound by the child report.
    for value in &values[22..92] {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!("Lock lifecycle payload scalar is not canonical"));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRATION_ID: u64 = 7;
    const ROUTE_ORDINAL: u64 = 1;
    const RUNTIME_GENERATION: u64 = 9;
    const SHM_CONNECTION_ID: u64 = 11;

    fn binding(
        path: LockRunnerLifecyclePathV1,
        action: LockRunnerActionV1,
    ) -> LockRunnerLifecycleBindingV1 {
        let (first, count, mask) = if path.is_local() {
            (2, 1, 4)
        } else {
            (1, 2, 6)
        };
        LockRunnerLifecycleBindingV1 {
            path,
            action,
            first,
            count,
            mask,
            normalized_descriptor_sha256: [0x11; 32],
            case_key_sha256: [0x22; 32],
            full_record_sha256: [0x33; 32],
            plan_sha256: [0x44; 32],
            implementation_sha256: [0x55; 32],
        }
    }

    fn canonical_values(binding: LockRunnerLifecycleBindingV1) -> Vec<u64> {
        let mut values = binding_values(binding);
        values.extend([
            REGISTRATION_ID,
            ROUTE_ORDINAL,
            RUNTIME_GENERATION,
            SHM_CONNECTION_ID,
            action_tag(binding.action),
            u64::from(binding.first),
            u64::from(binding.count),
            u64::from(binding.mask),
        ]);
        values.extend([
            1,
            u64::from(binding.first),
            u64::from(binding.count),
            raw_flags(binding.action) as u64,
            ffi::SQLITE_OK as u64,
            1,
            1,
            1,
            1,
        ]);
        values.extend(expected_selected_values(binding, false));
        values.extend(expected_selected_values(binding, true));
        let sibling = if binding.path.is_local() {
            [1, u64::from(binding.mask), 0]
        } else {
            [0; 3]
        };
        values.extend(sibling);
        values.extend(sibling);
        values.extend(expected_lock_receipt_values(
            binding,
            RUNTIME_GENERATION,
            SHM_CONNECTION_ID,
        ));
        values.extend([1, 1, 1, 1]);
        values.extend(if binding.path.is_local() {
            [2, 2, 6]
        } else {
            [1, 1, 3]
        });
        values.extend([1, 1, 1, 1]);
        assert_eq!(values.len(), REPORT_VALUE_COUNT);
        values
    }

    fn payload(binding: LockRunnerLifecycleBindingV1, values: &[u64]) -> String {
        format!(
            "{REPORT_VERSION},{},{}",
            exact_selector(binding),
            values
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    fn assert_rejected(binding: LockRunnerLifecycleBindingV1, values: &[u64]) {
        assert!(validate_payload(&payload(binding, values), binding).is_err());
    }

    #[test]
    fn accepts_canonical_payload_for_every_lifecycle_path() {
        for binding in [
            binding(
                LockRunnerLifecyclePathV1::NativeAcquire,
                LockRunnerActionV1::LockExclusive,
            ),
            binding(
                LockRunnerLifecyclePathV1::NativeRelease,
                LockRunnerActionV1::UnlockExclusive,
            ),
            binding(
                LockRunnerLifecyclePathV1::SharedLocalAcquire,
                LockRunnerActionV1::LockShared,
            ),
            binding(
                LockRunnerLifecyclePathV1::SharedLocalRelease,
                LockRunnerActionV1::UnlockShared,
            ),
        ] {
            let values = canonical_values(binding);
            assert!(validate_payload(&payload(binding, &values), binding).is_ok());
        }
    }

    #[test]
    fn rejects_tamper_in_each_exact_bound_payload_section() {
        let binding = binding(
            LockRunnerLifecyclePathV1::NativeAcquire,
            LockRunnerActionV1::LockExclusive,
        );
        for index in [0, 30, 39, 53, 67, 70, 73, 92, 96, 99] {
            let mut values = canonical_values(binding);
            values[index] ^= 1;
            assert_rejected(binding, &values);
        }
    }

    #[test]
    fn rejects_zero_dynamic_identity_fields() {
        let binding = binding(
            LockRunnerLifecyclePathV1::SharedLocalAcquire,
            LockRunnerActionV1::LockShared,
        );
        for index in 22..=25 {
            let mut values = canonical_values(binding);
            values[index] = 0;
            assert_rejected(binding, &values);
        }
    }

    #[test]
    fn rejects_lower_receipt_identity_divergence() {
        let binding = binding(
            LockRunnerLifecyclePathV1::NativeRelease,
            LockRunnerActionV1::UnlockExclusive,
        );
        for (index, identity_index) in [(73, 24), (74, 25)] {
            let mut values = canonical_values(binding);
            values[index] = values[identity_index] + 1;
            assert_rejected(binding, &values);
        }
    }
}
