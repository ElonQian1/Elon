//! Canonical q11 controlled-fault payload and independent parent-side validation.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use super::super::super::child::lock_raw_state_rejection::{REPORT_VALUE_COUNT, REPORT_VERSION};
use super::{
    completion_tag, exact_selector, raw_state_tag, rejection_tag, validate_binding,
    LockRunnerRawStateRejectionBindingV1,
};

const CONTROLLED_FAULT_ACTUAL_TAG: u64 = 1;

pub(in super::super) struct ValidatedRawStateRejectionPayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: LockRunnerRawStateRejectionBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    abi_values: [u64; 32],
    route_no_entry: [u64; 18],
    target_values: [u64; 2],
    route_before: [u64; 6],
    route_after: [u64; 6],
    registration_values: [u64; 4],
    retained_values: [u64; 4],
) -> String {
    let mut values = binding_values(binding);
    values.extend([registration_id, route_ordinal, CONTROLLED_FAULT_ACTUAL_TAG]);
    values.extend(abi_values);
    values.extend(route_no_entry);
    values.extend(target_values);
    values.extend(route_before);
    values.extend(route_after);
    values.extend(registration_values);
    values.extend(retained_values);
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
    binding: LockRunnerRawStateRejectionBindingV1,
) -> anyhow::Result<ValidatedRawStateRejectionPayloadV1> {
    validate_binding(binding)?;
    let mut fields = payload.split(',');
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(exact_selector(binding)) {
        return Err(anyhow!("q11 Lock raw-state payload identity mismatch"));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[..23] != binding_values(binding) {
        return Err(anyhow!(
            "q11 Lock raw-state payload program binding mismatch"
        ));
    }
    let abi_values: [u64; 32] = values[26..58]
        .try_into()
        .map_err(|_| anyhow!("q11 Lock raw-state ABI receipt width mismatch"))?;
    if values[23] == 0
        || values[24] != 1
        || values[25] != CONTROLLED_FAULT_ACTUAL_TAG
        || abi_values[3] == 0
        || abi_values != expected_abi_values(binding, abi_values[3])
        || values[58..76] != [1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        || values[76..78] != [0, 0]
        || values[78..84] != [3, 1, 0, 0, 0, 1]
        || values[84..90] != values[78..84]
        || values[90..94] != [1; 4]
        || values[94..98] != [1, 0, 1, 1]
    {
        return Err(anyhow!(
            "q11 Lock controlled raw-state receipt/custody mismatch"
        ));
    }
    Ok(ValidatedRawStateRejectionPayloadV1 {
        registration_id: values[23],
        native_receipt_sha256: digest_receipt(&values),
    })
}

fn binding_values(binding: LockRunnerRawStateRejectionBindingV1) -> Vec<u64> {
    let mut values = vec![
        rejection_tag(binding.rejection),
        raw_state_tag(binding.rejection),
        completion_tag(binding.rejection),
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

fn expected_abi_values(
    binding: LockRunnerRawStateRejectionBindingV1,
    observation_id: u64,
) -> [u64; 32] {
    let mut values = [0; 32];
    values[0] = 1;
    values[1] = rejection_tag(binding.rejection);
    values[2] = CONTROLLED_FAULT_ACTUAL_TAG;
    values[3] = observation_id;
    values[4] = 1;
    values[6] = 7;
    values[8] = 1;
    values[9] = 1;
    values[10] = 1;
    values[28] = 1;
    values[29] = ffi::SQLITE_IOERR_SHMLOCK as u64;
    let row = expected_case_values(binding);
    for (slot, value) in [
        1, 5, 7, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 30, 31,
    ]
    .into_iter()
    .zip(row)
    {
        values[slot] = value;
    }
    values
}

fn expected_case_values(binding: LockRunnerRawStateRejectionBindingV1) -> [u64; 22] {
    use super::LockRunnerRawStateRejectionV1 as R;
    match binding.rejection {
        R::NullFileDirect => [
            1, 1, 7, 1, 0, 0, 0, 0, 0, 0, 2, 1, 2, 0, 0, 0, 0, 0, 0, 0, 7, 0,
        ],
        R::UninstalledDirect => [
            2, 0, 0, 2, 0, 0, 0, 0, 0, 0, 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 5,
        ],
        R::MethodsNullStatePresentDirect => [
            3, 0, 2, 3, 0, 0, 0, 0, 0, 0, 2, 1, 3, 0, 0, 0, 0, 0, 0, 0, 2, 4,
        ],
        R::ForeignMethodsStateNullDirect => [
            4, 0, 1, 3, 0, 0, 0, 0, 0, 0, 2, 1, 3, 0, 0, 0, 0, 0, 0, 0, 1, 5,
        ],
        R::ForeignMethodsStatePresentDirect => [
            5, 0, 3, 3, 0, 0, 0, 0, 0, 0, 2, 1, 3, 0, 0, 0, 0, 0, 0, 0, 3, 4,
        ],
        R::ExactMethodsStateNullDirect => [
            6, 0, 5, 4, 0, 0, 0, 0, 0, 0, 2, 1, 4, 0, 0, 0, 0, 0, 0, 0, 5, 5,
        ],
        R::OtherTypePayloadMissingDropCompleted => [
            7, 0, 7, 5, 1, 0, 1, 0, 0, 0, 2, 1, 5, 1, 1, 0, 0, 0, 1, 0, 0, 5,
        ],
        R::OtherTypePayloadPresentDropCompleted => [
            8, 0, 7, 5, 1, 0, 1, 1, 0, 0, 2, 1, 5, 1, 1, 1, 1, 0, 1, 0, 0, 5,
        ],
        R::OtherTypePayloadPresentDropUnwindCaught => [
            9, 0, 7, 5, 1, 0, 1, 1, 0, 0, 2, 1, 6, 1, 1, 1, 0, 1, 0, 1, 0, 5,
        ],
        R::ExpectedTypePayloadMissingDropCompleted => [
            10, 0, 7, 6, 1, 1, 1, 0, 0, 0, 3, 1, 5, 1, 1, 0, 0, 0, 1, 0, 0, 5,
        ],
        R::HandleBoundFileMissingDirect => [
            11, 0, 7, 6, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 6,
        ],
    }
}

fn digest_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-lock-raw-state-rejection-controlled-fault-v1\0");
    hasher.update((values.len() as u64).to_le_bytes());
    for value in values {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!(
            "q11 Lock raw-state payload scalar is not canonical"
        ));
    }
    Ok(parsed)
}
