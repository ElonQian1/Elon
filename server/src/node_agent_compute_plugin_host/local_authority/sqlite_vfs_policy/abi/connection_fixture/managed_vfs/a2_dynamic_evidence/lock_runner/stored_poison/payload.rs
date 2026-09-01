//! Canonical q3 payload and independent parent-side stored-poison receipt validation.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAction, ManagedSqliteShmTestLockPath, ManagedSqliteShmTestLockReceipt,
    ManagedSqliteShmTestStoredPoisonReceiptV1, ManagedSqliteShmTestStoredPoisonV1,
    ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::connection::ManagedTestShmLockCallbackObservation;
use super::super::super::child::lock_stored_poison::{
    selector, REPORT_VALUE_COUNT, REPORT_VERSION,
};
use super::fixture::{phase_tag, snapshot_values};
use super::{
    action_tag, raw_flags, LockRunnerStoredPoisonBindingV1, LockRunnerStoredPoisonCompletionV1,
    LockRunnerStoredPoisonProfileV1,
};

pub(in super::super) struct ValidatedStoredPoisonPayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: LockRunnerStoredPoisonBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    poison_receipt: ManagedSqliteShmTestStoredPoisonReceiptV1,
    callback: ManagedTestShmLockCallbackObservation,
    baseline: ManagedSqliteShmTestTargetSnapshot,
    poisoned: ManagedSqliteShmTestTargetSnapshot,
    after: ManagedSqliteShmTestTargetSnapshot,
    lower_receipt: ManagedSqliteShmTestLockReceipt,
    pending_before: usize,
    pending_after: usize,
    terminal: [u64; 18],
    registration: [u64; 4],
    route: [u64; 3],
    root_present: u64,
) -> String {
    let mut values = binding_values(binding);
    values.extend([
        registration_id,
        route_ordinal,
        runtime_generation,
        shm_connection_id,
    ]);
    values.extend([
        action_tag(binding.action),
        u64::from(binding.first),
        u64::from(binding.count),
        u64::from(binding.mask),
    ]);
    values.extend(poison_receipt_values(poison_receipt));
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
    values.extend(snapshot_values(baseline));
    values.extend(snapshot_values(poisoned));
    values.extend(snapshot_values(after));
    values.extend(lower_receipt_values(
        lower_receipt,
        pending_before,
        pending_after,
    ));
    values.extend(terminal);
    values.extend(registration);
    values.extend(route);
    values.push(root_present);
    assert_eq!(values.len(), REPORT_VALUE_COUNT);
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
    binding: LockRunnerStoredPoisonBindingV1,
) -> anyhow::Result<ValidatedStoredPoisonPayloadV1> {
    super::validate_binding(binding)?;
    let mut fields = payload.split(',');
    let expected_selector = exact_selector(binding);
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(expected_selector.as_str()) {
        return Err(anyhow!("Lock stored-poison payload identity mismatch"));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let registration_id = validate_common_values(
        &values,
        binding,
        [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0],
    )?;
    Ok(ValidatedStoredPoisonPayloadV1 {
        registration_id,
        native_receipt_sha256: digest_native_receipt(&values),
    })
}

pub(in super::super) fn validate_common_values(
    values: &[u64],
    binding: LockRunnerStoredPoisonBindingV1,
    terminal: [u64; 18],
) -> anyhow::Result<u64> {
    if values.len() != REPORT_VALUE_COUNT || values[..22] != binding_values(binding) {
        return Err(anyhow!(
            "Lock stored-poison payload program binding mismatch"
        ));
    }
    if values[22] == 0
        || values[23] != 1
        || values[24] == 0
        || values[25] == 0
        || values[26..30]
            != [
                action_tag(binding.action),
                u64::from(binding.first),
                u64::from(binding.count),
                u64::from(binding.mask),
            ]
        || values[30..38] != expected_poison_receipt_values(binding, values[24], values[25])
        || values[38..47]
            != [
                1,
                u64::from(binding.first),
                u64::from(binding.count),
                raw_flags(binding.action) as u64,
                ffi::SQLITE_IOERR_SHMLOCK as u64,
                1,
                1,
                1,
                1,
            ]
    {
        return Err(anyhow!(
            "Lock stored-poison payload installed-ABI binding mismatch"
        ));
    }
    let baseline = expected_snapshot_values(binding.profile, false);
    let poisoned = expected_snapshot_values(binding.profile, true);
    if values[47..61] != baseline
        || values[61..75] != poisoned
        || values[75..89] != poisoned
        || values[89..109] != expected_lower_receipt_values(binding, values[24], values[25])
        || values[109..127] != terminal
        || values[127..131] != [1, 1, 1, 1]
        || values[131..134] != [1, 1, 3]
        || values[134] != 1
    {
        return Err(anyhow!(
            "Lock stored-poison payload retention receipt mismatch"
        ));
    }
    Ok(values[22])
}

fn expected_lower_receipt_values(
    binding: LockRunnerStoredPoisonBindingV1,
    runtime_generation: u64,
    shm_connection_id: u64,
) -> [u64; 20] {
    [
        runtime_generation,
        shm_connection_id,
        action_tag(binding.action),
        u64::from(binding.first),
        u64::from(binding.count),
        u64::from(binding.mask),
        match binding.action {
            super::LockRunnerActionV1::LockShared | super::LockRunnerActionV1::LockExclusive => 1,
            super::LockRunnerActionV1::UnlockShared
            | super::LockRunnerActionV1::UnlockExclusive => 2,
        },
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
        0,
        0,
        1,
    ]
}

fn lower_receipt_values(
    value: ManagedSqliteShmTestLockReceipt,
    pending_before: usize,
    pending_after: usize,
) -> [u64; 20] {
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
        pending_before as u64,
        pending_after as u64,
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
    }
}

fn expected_snapshot_values(profile: LockRunnerStoredPoisonProfileV1, poisoned: bool) -> [u64; 14] {
    [
        1,
        0,
        0,
        1,
        1,
        1,
        1,
        1,
        1,
        u64::from(poisoned),
        u64::from(poisoned && profile.mutation_may_have_occurred()),
        u64::from(poisoned && profile.lock_outcome_uncertain()),
        u64::from(poisoned),
        0,
    ]
}

fn expected_poison_receipt_values(
    binding: LockRunnerStoredPoisonBindingV1,
    runtime_generation: u64,
    shm_connection_id: u64,
) -> [u64; 8] {
    [
        runtime_generation,
        shm_connection_id,
        binding.profile.tag(),
        phase_tag(super::fixture::managed_phase(binding.profile)),
        1,
        u64::from(binding.profile.mutation_may_have_occurred()),
        u64::from(binding.profile.lock_outcome_uncertain()),
        1,
    ]
}

fn poison_receipt_values(value: ManagedSqliteShmTestStoredPoisonReceiptV1) -> [u64; 8] {
    [
        value.runtime_generation,
        value.shm_connection_id,
        managed_profile_tag(value.profile),
        phase_tag(value.phase),
        u64::from(matches!(
            value.class,
            crate::node_agent_managed_fs::ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
        )),
        u64::from(value.mutation_may_have_occurred),
        u64::from(value.lock_outcome_uncertain),
        u64::from(value.domain_terminal),
    ]
}

fn managed_profile_tag(value: ManagedSqliteShmTestStoredPoisonV1) -> u64 {
    match value {
        ManagedSqliteShmTestStoredPoisonV1::GateNoMutation => 1,
        ManagedSqliteShmTestStoredPoisonV1::FileCloseNoMutation => 2,
        ManagedSqliteShmTestStoredPoisonV1::ExactSiblingDeleteNoMutation => 3,
        ManagedSqliteShmTestStoredPoisonV1::ExactSiblingOpenUncertain => 4,
        ManagedSqliteShmTestStoredPoisonV1::DmsTruncateUncertain => 5,
        ManagedSqliteShmTestStoredPoisonV1::FileCloseUncertain => 6,
        ManagedSqliteShmTestStoredPoisonV1::ExactSiblingDeleteUncertain => 7,
        ManagedSqliteShmTestStoredPoisonV1::FileGrowUncertain => 8,
        ManagedSqliteShmTestStoredPoisonV1::MappingCloseUncertain => 9,
        ManagedSqliteShmTestStoredPoisonV1::ViewUnmapUncertain => 10,
        ManagedSqliteShmTestStoredPoisonV1::LockReleaseUncertain => 11,
        ManagedSqliteShmTestStoredPoisonV1::ConnectionDetachUncertain => 12,
        ManagedSqliteShmTestStoredPoisonV1::DeleteAuthorizationUncertain => 13,
        ManagedSqliteShmTestStoredPoisonV1::DmsExclusiveReleaseUncertain => 14,
        ManagedSqliteShmTestStoredPoisonV1::DmsSharedReleaseUncertain => 15,
    }
}

pub(in super::super) fn binding_values(binding: LockRunnerStoredPoisonBindingV1) -> Vec<u64> {
    let mut values = vec![action_tag(binding.action), binding.profile.tag()];
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

pub(in super::super) fn exact_selector(binding: LockRunnerStoredPoisonBindingV1) -> String {
    selector(
        action_tag(binding.action),
        binding.profile.tag(),
        binding.first,
        binding.count,
    )
    .expect("validated Lock stored-poison selector")
}

fn digest_native_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-stored-poison-native-receipt-v3\0");
    for value in &values[22..127] {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

pub(in super::super) fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!(
            "Lock stored-poison payload scalar is not canonical"
        ));
    }
    Ok(parsed)
}

#[cfg(test)]
pub(in super::super) fn canonical_values_for_test(
    binding: LockRunnerStoredPoisonBindingV1,
    terminal: [u64; 18],
) -> Vec<u64> {
    let mut values = binding_values(binding);
    values.extend([7, 1, 9, 11]);
    values.extend([2, 1, 2, 6]);
    values.extend(expected_poison_receipt_values(binding, 9, 11));
    values.extend([
        1,
        1,
        2,
        raw_flags(binding.action) as u64,
        ffi::SQLITE_IOERR_SHMLOCK as u64,
        1,
        1,
        1,
        1,
    ]);
    values.extend(expected_snapshot_values(binding.profile, false));
    values.extend(expected_snapshot_values(binding.profile, true));
    values.extend(expected_snapshot_values(binding.profile, true));
    values.extend(expected_lower_receipt_values(binding, 9, 11));
    values.extend(terminal);
    values.extend([1, 1, 1, 1]);
    values.extend([1, 1, 3]);
    values.push(1);
    assert_eq!(values.len(), REPORT_VALUE_COUNT);
    values
}

#[cfg(test)]
pub(in super::super) fn native_receipt_sha256_for_test(values: &[u64]) -> [u8; 32] {
    digest_native_receipt(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::lock_runner::LockRunnerActionV1;

    fn binding(profile: LockRunnerStoredPoisonProfileV1) -> LockRunnerStoredPoisonBindingV1 {
        LockRunnerStoredPoisonBindingV1 {
            action: LockRunnerActionV1::LockExclusive,
            first: 1,
            count: 2,
            mask: 6,
            profile,
            completion: LockRunnerStoredPoisonCompletionV1::RetentionSucceeded,
            normalized_descriptor_sha256: [0x11; 32],
            case_key_sha256: [0x22; 32],
            full_record_sha256: [0x33; 32],
            plan_sha256: [0x44; 32],
            implementation_sha256: [0x55; 32],
        }
    }

    fn payload(binding: LockRunnerStoredPoisonBindingV1, values: &[u64]) -> String {
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

    #[test]
    fn canonical_q3_payload_is_bound_for_every_poison_shape() {
        for profile in [
            LockRunnerStoredPoisonProfileV1::GateNoMutation,
            LockRunnerStoredPoisonProfileV1::FileCloseNoMutation,
            LockRunnerStoredPoisonProfileV1::ExactSiblingDeleteNoMutation,
            LockRunnerStoredPoisonProfileV1::ExactSiblingOpenUncertain,
            LockRunnerStoredPoisonProfileV1::DmsTruncateUncertain,
            LockRunnerStoredPoisonProfileV1::FileCloseUncertain,
            LockRunnerStoredPoisonProfileV1::ExactSiblingDeleteUncertain,
            LockRunnerStoredPoisonProfileV1::FileGrowUncertain,
            LockRunnerStoredPoisonProfileV1::MappingCloseUncertain,
            LockRunnerStoredPoisonProfileV1::ViewUnmapUncertain,
            LockRunnerStoredPoisonProfileV1::LockReleaseUncertain,
            LockRunnerStoredPoisonProfileV1::ConnectionDetachUncertain,
            LockRunnerStoredPoisonProfileV1::DeleteAuthorizationUncertain,
            LockRunnerStoredPoisonProfileV1::DmsExclusiveReleaseUncertain,
            LockRunnerStoredPoisonProfileV1::DmsSharedReleaseUncertain,
        ] {
            let binding = binding(profile);
            let values = canonical_values_for_test(
                binding,
                [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0],
            );
            assert!(validate_payload(&payload(binding, &values), binding).is_ok());
        }
    }

    #[test]
    fn q3_payload_rejects_tamper_in_each_bound_section() {
        let binding = binding(LockRunnerStoredPoisonProfileV1::FileGrowUncertain);
        for index in [0, 23, 24, 26, 30, 38, 47, 61, 75, 89, 109, 127, 131, 134] {
            let mut values = canonical_values_for_test(
                binding,
                [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0],
            );
            values[index] ^= 1;
            assert!(validate_payload(&payload(binding, &values), binding).is_err());
        }
    }
}
