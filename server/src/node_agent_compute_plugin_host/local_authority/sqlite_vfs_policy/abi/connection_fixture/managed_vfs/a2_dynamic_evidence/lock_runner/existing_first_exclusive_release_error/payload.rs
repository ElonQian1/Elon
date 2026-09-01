//! Canonical q13 controlled-fault payload and independent parent-side validation.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use super::super::super::child::lock_existing_first_exclusive_release_error::{
    REPORT_VALUE_COUNT, REPORT_VERSION,
};
use super::super::lifecycle;
use super::{
    completion_tag, exact_selector, validate_binding,
    LockRunnerExistingFirstExclusiveReleaseCompletionV1,
    LockRunnerNativeAcquireExistingFirstExclusiveReleaseErrorBindingV1,
};

const CONTROLLED_FAULT_ACTUAL_TAG: u64 = 1;
const EXISTING_SHM_PRECREATION_RECEIPT: [u64; 8] = [1, 1, 1, 4, 1, 1, 4, 1];

pub(in super::super) struct ValidatedExistingFirstExclusiveReleaseErrorPayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: LockRunnerNativeAcquireExistingFirstExclusiveReleaseErrorBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    existing_shm_precreation: [u64; 8],
    cold_setup: [u64; 25],
    callback: [u64; 8],
    after: [u64; 14],
    initialization: [u64; 32],
    lock_no_requested_native: [u64; 18],
    pending_count: u64,
    terminal: [u64; 18],
    preemption: [u64; 6],
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
        CONTROLLED_FAULT_ACTUAL_TAG,
    ]);
    values.extend(existing_shm_precreation);
    values.extend(cold_setup);
    values.extend(callback);
    values.extend(after);
    values.extend(initialization);
    values.extend(lock_no_requested_native);
    values.push(pending_count);
    values.extend(terminal);
    values.extend(preemption);
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
    binding: LockRunnerNativeAcquireExistingFirstExclusiveReleaseErrorBindingV1,
) -> anyhow::Result<ValidatedExistingFirstExclusiveReleaseErrorPayloadV1> {
    validate_binding(binding)?;
    let selector = exact_selector(binding);
    let mut fields = payload.split(',');
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(selector.as_str()) {
        return Err(anyhow!("q13 Lock initialization payload identity mismatch"));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[..25] != binding_values(binding) {
        return Err(anyhow!("q13 Lock initialization payload binding mismatch"));
    }

    let expected_cold = [
        1,
        1,
        256,
        32 * 1024,
        0,
        ffi::SQLITE_IOERR_SHMMAP as u64,
        1,
        1,
        1,
        1,
        1,
        1,
        0,
        0,
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
    ];
    let expected_callback = [
        u64::from(binding.first),
        u64::from(binding.count),
        lifecycle::raw_flags(binding.action) as u64,
        ffi::SQLITE_IOERR_SHMLOCK as u64,
        1,
        1,
        1,
        1,
    ];
    let expected_after = [1, 0, 0, 1, 1, 0, 0, 4, 1, 1, 1, 1, 1, 0];
    let expected_lock = expected_lock_values(binding, values[27], values[28]);
    let expected_terminal = match binding.completion {
        LockRunnerExistingFirstExclusiveReleaseCompletionV1::RetentionSucceeded => {
            [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0]
        }
        LockRunnerExistingFirstExclusiveReleaseCompletionV1::RetentionRouteUnknown => {
            [3, 1, 0, 0, 2, 2, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0]
        }
    };
    let expected_preemption = match binding.completion {
        LockRunnerExistingFirstExclusiveReleaseCompletionV1::RetentionSucceeded => [0; 6],
        LockRunnerExistingFirstExclusiveReleaseCompletionV1::RetentionRouteUnknown => [1; 6],
    };
    if values[25] == 0
        || values[26] != 1
        || values[27] == 0
        || values[28] == 0
        || values[29] != CONTROLLED_FAULT_ACTUAL_TAG
        || values[30..38] != EXISTING_SHM_PRECREATION_RECEIPT
        || values[38..63] != expected_cold
        || values[63..71] != expected_callback
        || values[71..85] != expected_after
        || !exact_initialization_values(binding, &values[85..117], values[27], values[28])
        || values[117..135] != expected_lock
        || values[135] != 0
        || values[136..154] != expected_terminal
        || values[154..160] != expected_preemption
        || values[160..164] != [1; 4]
        || values[164..167] != [1, 1, 3]
        || values[167] != 1
    {
        return Err(anyhow!("q13 controlled initialization receipt/custody mismatch"));
    }
    Ok(ValidatedExistingFirstExclusiveReleaseErrorPayloadV1 {
        registration_id: values[25],
        native_receipt_sha256: digest_receipt(&values),
    })
}

fn binding_values(
    binding: LockRunnerNativeAcquireExistingFirstExclusiveReleaseErrorBindingV1,
) -> Vec<u64> {
    let mut values = vec![
        lifecycle::action_tag(binding.action),
        u64::from(binding.first),
        u64::from(binding.count),
        u64::from(binding.mask),
        completion_tag(binding.completion),
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

fn expected_lock_values(
    binding: LockRunnerNativeAcquireExistingFirstExclusiveReleaseErrorBindingV1,
    runtime_generation: u64,
    shm_connection_id: u64,
) -> [u64; 18] {
    [
        runtime_generation,
        shm_connection_id,
        lifecycle::action_tag(binding.action),
        u64::from(binding.first),
        u64::from(binding.count),
        u64::from(binding.mask),
        6,
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
        1,
    ]
}

fn exact_initialization_values(
    binding: LockRunnerNativeAcquireExistingFirstExclusiveReleaseErrorBindingV1,
    values: &[u64],
    runtime_generation: u64,
    shm_connection_id: u64,
) -> bool {
    values.len() == 32
        && values[0] == 1
        && values[1] == 2
        && values[2] == CONTROLLED_FAULT_ACTUAL_TAG
        && values[3] == runtime_generation
        && values[4] == shm_connection_id
        && values[5] == lifecycle::action_tag(binding.action)
        && values[6] == u64::from(binding.first)
        && values[7] == u64::from(binding.count)
        && values[8] == u64::from(binding.mask)
        && values[9..12] == [1, 1, 1]
        && values[12] == 0
        && values[13..21] == [1; 8]
        && values[21..28] == [1, 128, 1, 1, 1, 1, 1]
        && values[28] == 511
        && values[29..32] == [0, 1, 1]
}

fn digest_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-lock-existing-first-exclusive-release-error-controlled-fault-v1\0");
    hasher.update((values.len() as u64).to_le_bytes());
    for value in values {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!("q13 Lock initialization payload scalar is not canonical"));
    }
    Ok(parsed)
}
