//! Canonical Q19 actual payload and independent parent-side validation.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use super::super::super::child::lock_existing_first_shared_busy_close_succeeded::{
    REPORT_VALUE_COUNT, REPORT_VERSION,
};
use super::super::lifecycle;
use super::{
    completion_tag, exact_selector, validate_binding,
    LockRunnerExistingFirstSharedBusyCloseSucceededCompletionV1,
    LockRunnerNativeAcquireExistingFirstSharedBusyCloseSucceededBindingV1,
};

const ACTUAL_TAG: u64 = 1;
const BINDING_END: usize = 25;
const PRECREATION_START: usize = 30;
const PRECREATION_END: usize = 38;
const COLD_START: usize = 38;
const COLD_END: usize = 63;
const CALLBACK_START: usize = 63;
const CALLBACK_END: usize = 71;
const AFTER_START: usize = 71;
const AFTER_END: usize = 85;
const INITIALIZATION_START: usize = 85;
const INITIALIZATION_END: usize = 128;
const HOLDER_START: usize = 128;
const HOLDER_END: usize = 143;
const LOCK_START: usize = 143;
const LOCK_END: usize = 161;
const PENDING_INDEX: usize = 161;
const TERMINAL_START: usize = 162;
const TERMINAL_END: usize = 180;
const PREEMPTION_START: usize = 180;
const PREEMPTION_END: usize = 186;
const REGISTRATION_START: usize = 186;
const REGISTRATION_END: usize = 190;
const ROUTE_START: usize = 190;
const ROUTE_END: usize = 193;
const ROOT_SHAPE_INDEX: usize = 193;
const EXISTING_SHM_PRECREATION_RECEIPT: [u64; 8] = [1, 1, 1, 4, 1, 1, 4, 1];

pub(in super::super) struct ValidatedExistingFirstSharedBusyCloseSucceededPayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: LockRunnerNativeAcquireExistingFirstSharedBusyCloseSucceededBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    existing_shm_precreation: [u64; 8],
    cold_setup: [u64; 25],
    callback: [u64; 8],
    after: [u64; 14],
    initialization: [u64; 43],
    holder: [u64; 15],
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
        ACTUAL_TAG,
    ]);
    values.extend(existing_shm_precreation);
    values.extend(cold_setup);
    values.extend(callback);
    values.extend(after);
    values.extend(initialization);
    values.extend(holder);
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
    binding: LockRunnerNativeAcquireExistingFirstSharedBusyCloseSucceededBindingV1,
) -> anyhow::Result<ValidatedExistingFirstSharedBusyCloseSucceededPayloadV1> {
    validate_binding(binding)?;
    let selector = exact_selector(binding);
    let mut fields = payload.split(',');
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(selector.as_str()) {
        return Err(anyhow!("q19 Lock initialization payload identity mismatch"));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[..BINDING_END] != binding_values(binding) {
        return Err(anyhow!("q19 Lock initialization payload binding mismatch"));
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
    let expected_after = [1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let expected_terminal = match binding.completion {
        LockRunnerExistingFirstSharedBusyCloseSucceededCompletionV1::RetentionSucceeded => {
            [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0]
        }
        LockRunnerExistingFirstSharedBusyCloseSucceededCompletionV1::RetentionRouteUnknown => {
            [3, 1, 0, 0, 2, 2, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0]
        }
    };
    let expected_preemption = match binding.completion {
        LockRunnerExistingFirstSharedBusyCloseSucceededCompletionV1::RetentionSucceeded => [0; 6],
        LockRunnerExistingFirstSharedBusyCloseSucceededCompletionV1::RetentionRouteUnknown => {
            [1; 6]
        }
    };
    let runtime_generation = values[27];
    let shm_connection_id = values[28];
    if values[25] == 0
        || values[26] != 1
        || runtime_generation == 0
        || shm_connection_id == 0
        || values[29] != ACTUAL_TAG
        || values[PRECREATION_START..PRECREATION_END] != EXISTING_SHM_PRECREATION_RECEIPT
        || values[COLD_START..COLD_END] != expected_cold
        || values[CALLBACK_START..CALLBACK_END] != expected_callback
        || values[AFTER_START..AFTER_END] != expected_after
        || !exact_initialization_values(
            binding,
            &values[INITIALIZATION_START..INITIALIZATION_END],
            runtime_generation,
            shm_connection_id,
        )
        || values[HOLDER_START..HOLDER_END]
            != expected_holder_values(runtime_generation, shm_connection_id)
        || values[LOCK_START..LOCK_END]
            != expected_lock_values(binding, runtime_generation, shm_connection_id)
        || values[PENDING_INDEX] != 0
        || values[TERMINAL_START..TERMINAL_END] != expected_terminal
        || values[PREEMPTION_START..PREEMPTION_END] != expected_preemption
        || values[REGISTRATION_START..REGISTRATION_END] != [1; 4]
        || values[ROUTE_START..ROUTE_END] != [1, 1, 3]
        || values[ROOT_SHAPE_INDEX] != 1
    {
        return Err(anyhow!(
            "q19 existing-first precreation/contention/close custody mismatch"
        ));
    }
    Ok(ValidatedExistingFirstSharedBusyCloseSucceededPayloadV1 {
        registration_id: values[25],
        native_receipt_sha256: digest_receipt(&values),
    })
}

fn binding_values(
    binding: LockRunnerNativeAcquireExistingFirstSharedBusyCloseSucceededBindingV1,
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

fn exact_initialization_values(
    binding: LockRunnerNativeAcquireExistingFirstSharedBusyCloseSucceededBindingV1,
    values: &[u64],
    runtime_generation: u64,
    shm_connection_id: u64,
) -> bool {
    values.len() == 43
        && values[0..3] == [1, 8, ACTUAL_TAG]
        && values[3] == runtime_generation
        && values[4] == shm_connection_id
        && values[5] == lifecycle::action_tag(binding.action)
        && values[6..9]
            == [
                u64::from(binding.first),
                u64::from(binding.count),
                u64::from(binding.mask),
            ]
        && values[9..13] == [1, 1, 1, 0]
        && values[13..23] == [1; 10]
        && values[23..29] == [0, 1, 0, 1, 1, 0]
        && values[29..39] == [2, 1, 1, 0, 1, 1, 1, 1, 1, 0]
        && values[39..43] == [1, 0, 1, 1]
}

fn expected_holder_values(runtime_generation: u64, shm_connection_id: u64) -> [u64; 15] {
    [
        runtime_generation,
        shm_connection_id,
        128,
        1,
        1,
        1,
        1,
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

fn expected_lock_values(
    binding: LockRunnerNativeAcquireExistingFirstSharedBusyCloseSucceededBindingV1,
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

fn digest_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-lock-existing-first-shared-busy-close-succeeded-actual-v1\0");
    hasher.update((values.len() as u64).to_le_bytes());
    for value in values {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!("q19 payload scalar is not canonical"));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::super::LockRunnerActionV1;
    use super::*;

    fn binding() -> LockRunnerNativeAcquireExistingFirstSharedBusyCloseSucceededBindingV1 {
        LockRunnerNativeAcquireExistingFirstSharedBusyCloseSucceededBindingV1 {
            action: LockRunnerActionV1::LockShared,
            first: 2,
            count: 1,
            mask: 4,
            completion:
                LockRunnerExistingFirstSharedBusyCloseSucceededCompletionV1::RetentionSucceeded,
            normalized_descriptor_sha256: [1; 32],
            case_key_sha256: [2; 32],
            full_record_sha256: [3; 32],
            plan_sha256: [4; 32],
            implementation_sha256: [5; 32],
        }
    }

    fn valid_payload() -> String {
        let binding = binding();
        let runtime = 101;
        let connection = 202;
        let mut initialization = [1; 43];
        initialization[0..13]
            .copy_from_slice(&[1, 8, 1, runtime, connection, 1, 2, 1, 4, 1, 1, 1, 0]);
        initialization[23..43]
            .copy_from_slice(&[0, 1, 0, 1, 1, 0, 2, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1]);
        encode(
            binding,
            303,
            1,
            runtime,
            connection,
            EXISTING_SHM_PRECREATION_RECEIPT,
            [
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
            ],
            [
                2,
                1,
                lifecycle::raw_flags(binding.action) as u64,
                ffi::SQLITE_IOERR_SHMLOCK as u64,
                1,
                1,
                1,
                1,
            ],
            [1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            initialization,
            expected_holder_values(runtime, connection),
            expected_lock_values(binding, runtime, connection),
            0,
            [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0],
            [0; 6],
            [1; 4],
            [1, 1, 3],
            1,
        )
    }

    fn mutate_value(payload: &str, index: usize, value: u64) -> String {
        let mut fields = payload.split(',').map(str::to_owned).collect::<Vec<_>>();
        fields[index + 2] = value.to_string();
        fields.join(",")
    }

    #[test]
    fn exact_q19_payload_is_accepted() {
        let payload = valid_payload();
        assert_eq!(payload.split(',').skip(2).count(), REPORT_VALUE_COUNT);
        assert!(validate_payload(&payload, binding()).is_ok());
    }

    #[test]
    fn precreation_created_first_and_receipt_splices_fail_closed() {
        assert!(validate_payload(
            &mutate_value(&valid_payload(), PRECREATION_START + 6, 0),
            binding()
        )
        .is_err());
        assert!(validate_payload(
            &mutate_value(&valid_payload(), INITIALIZATION_START + 1, 7),
            binding()
        )
        .is_err());
        let payload = valid_payload();
        let mut missing = payload.split(',').collect::<Vec<_>>();
        missing.pop();
        assert!(validate_payload(&missing.join(","), binding()).is_err());
        assert!(validate_payload(&format!("{payload},1"), binding()).is_err());
    }

    #[test]
    fn holder_target_and_close_receipts_fail_closed() {
        assert!(validate_payload(
            &mutate_value(&valid_payload(), HOLDER_START + 6, 0),
            binding()
        )
        .is_err());
        assert!(validate_payload(
            &mutate_value(&valid_payload(), HOLDER_START + 7, 0),
            binding()
        )
        .is_err());
        assert!(validate_payload(
            &mutate_value(&valid_payload(), INITIALIZATION_START + 27, 0),
            binding()
        )
        .is_err());
    }
}
