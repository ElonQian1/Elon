//! Canonical q9 payload and independent full-vector parent validation.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use super::super::super::super::connection::ManagedTestShmLockCallbackObservation;
use super::super::super::super::lifecycle_faults::ManagedTestPreManagedLockSnapshot;
use super::super::super::child::lock_pre_managed_rejection::{REPORT_VALUE_COUNT, REPORT_VERSION};
use super::super::{lifecycle, LockRunnerActionV1};
use super::fixture::{lifecycle_values, lock_effect};
use super::{
    completion_tag, exact_selector, rejection_tag, validate_binding,
    LockRunnerPreManagedCompletionV1, LockRunnerPreManagedRejectionBindingV1,
    LockRunnerPreManagedRejectionV1,
};

pub(in super::super) struct ValidatedPreManagedRejectionPayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: LockRunnerPreManagedRejectionBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    raw: ManagedTestShmLockCallbackObservation,
    setup: [u64; 19],
    prime: [u64; 4],
    admission_quarantine: u64,
    observation: ManagedTestPreManagedLockSnapshot,
    terminal: [u64; 17],
    route: [u64; 7],
    registration: [u64; 4],
    cleanup: [u64; 5],
) -> String {
    let mut values = binding_values(binding);
    values.extend([registration_id, route_ordinal]);
    values.extend([
        1,
        raw.offset() as u64,
        raw.count() as u64,
        raw.raw_flags() as u64,
        raw.result_code() as u64,
        u64::from(raw.before().methods_installed),
        u64::from(raw.before().state_installed),
        u64::from(raw.after().methods_installed),
        u64::from(raw.after().state_installed),
    ]);
    values.extend(setup);
    values.extend(prime);
    values.push(admission_quarantine);
    values.extend(lifecycle_values(observation));
    values.extend(observation.lower_ledger_values());
    values.extend(terminal);
    values.extend(route);
    values.extend(registration);
    values.extend(cleanup);
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
    binding: LockRunnerPreManagedRejectionBindingV1,
) -> anyhow::Result<ValidatedPreManagedRejectionPayloadV1> {
    validate_binding(binding)?;
    let mut fields = payload.split(',');
    let selector = exact_selector(binding);
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(selector.as_str()) {
        return Err(anyhow!("q9 Lock payload identity mismatch"));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[..26] != binding_values(binding) {
        return Err(anyhow!("q9 Lock payload program binding mismatch"));
    }
    let raw = [
        1,
        binding.first as u64,
        binding.count as u64,
        lifecycle::raw_flags(binding.action) as u64,
        ffi::SQLITE_IOERR_SHMLOCK as u64,
        1,
        1,
        1,
        1,
    ];
    if values[26] == 0
        || values[27] != 1
        || values[28..37] != raw
        || values[37..56] != expected_setup(binding)
        || values[56..60] != expected_prime(binding)
        || values[60] != expected_admission_quarantine(binding)
        || values[61..79] != expected_observation(binding)
        || values[79..82] != [lock_effect(binding), 0, 0]
        || values[82..99] != expected_terminal(binding)
        || values[99..106] != expected_route(binding)
        || values[106..110] != [1; 4]
        || values[110..115] != expected_cleanup(binding)
    {
        return Err(anyhow!(
            "q9 Lock installed-ABI/pre-managed/terminal receipt mismatch"
        ));
    }
    Ok(ValidatedPreManagedRejectionPayloadV1 {
        registration_id: values[26],
        native_receipt_sha256: digest_receipt(&values),
    })
}

fn binding_values(binding: LockRunnerPreManagedRejectionBindingV1) -> Vec<u64> {
    let mut values = vec![
        rejection_tag(binding.rejection),
        completion_tag(binding.completion),
        lifecycle::action_tag(binding.action),
        binding.first as u64,
        binding.count as u64,
        binding.mask as u64,
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

fn expected_setup(binding: LockRunnerPreManagedRejectionBindingV1) -> [u64; 19] {
    if binding.rejection == LockRunnerPreManagedRejectionV1::ShmDetached {
        [
            1,
            0,
            32_768,
            1,
            ffi::SQLITE_OK as u64,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            0,
            ffi::SQLITE_OK as u64,
            1,
            1,
            1,
            1,
            0,
        ]
    } else {
        [0; 19]
    }
}

fn expected_prime(binding: LockRunnerPreManagedRejectionBindingV1) -> [u64; 4] {
    if binding.rejection == LockRunnerPreManagedRejectionV1::AdmissionCounterOverflow {
        [1, 0, u32::MAX as u64, 1]
    } else {
        [0; 4]
    }
}

const fn expected_admission_quarantine(binding: LockRunnerPreManagedRejectionBindingV1) -> u64 {
    matches!(
        binding.rejection,
        LockRunnerPreManagedRejectionV1::AdmissionRouteUnknown
    ) as u64
}

fn expected_observation(binding: LockRunnerPreManagedRejectionBindingV1) -> [u64; 18] {
    use LockRunnerPreManagedCompletionV1 as C;
    use LockRunnerPreManagedRejectionV1 as R;
    match (binding.rejection, binding.completion) {
        (R::AdmissionRouteUnknown, C::Direct) => {
            [1, 1, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        }
        (R::AdmissionCounterOverflow, C::Direct) => {
            [1, 2, 1, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        }
        (R::UnsupportedFileRole, C::Completed) => {
            [1, 3, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0]
        }
        (R::UnsupportedFileRole, C::RouteUnknown) => {
            [1, 4, 1, 1, 1, 1, 1, 0, 1, 0, 1, 2, 1, 1, 1, 1, 1, 0]
        }
        (R::ShmDetached, C::Completed) => [1, 5, 1, 1, 1, 1, 2, 0, 2, 0, 1, 1, 0, 0, 0, 0, 0, 0],
        (R::ShmDetached, C::RouteUnknown) => [1, 6, 1, 1, 1, 1, 2, 0, 2, 0, 1, 2, 1, 1, 1, 1, 1, 0],
        _ => [0; 18],
    }
}

fn expected_terminal(binding: LockRunnerPreManagedRejectionBindingV1) -> [u64; 17] {
    use LockRunnerPreManagedCompletionV1 as C;
    use LockRunnerPreManagedRejectionV1 as R;
    match (binding.rejection, binding.completion) {
        (R::AdmissionRouteUnknown, C::Direct) => {
            [1, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 0, 0]
        }
        (R::AdmissionCounterOverflow, C::Direct) => {
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]
        }
        (R::UnsupportedFileRole | R::ShmDetached, C::Completed) => {
            [0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1]
        }
        (R::UnsupportedFileRole, C::RouteUnknown) => {
            [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 0, 0]
        }
        (R::ShmDetached, C::RouteUnknown) => [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0],
        _ => [0; 17],
    }
}

fn expected_route(binding: LockRunnerPreManagedRejectionBindingV1) -> [u64; 7] {
    match (binding.rejection, binding.completion) {
        (
            LockRunnerPreManagedRejectionV1::UnsupportedFileRole,
            LockRunnerPreManagedCompletionV1::Completed,
        ) => [1, 3, 1, 1, 0, 0, 1],
        (
            LockRunnerPreManagedRejectionV1::ShmDetached,
            LockRunnerPreManagedCompletionV1::Completed,
        ) => [1, 3, 1, 1, 1, 0, 1],
        _ => [0; 7],
    }
}

fn expected_cleanup(binding: LockRunnerPreManagedRejectionBindingV1) -> [u64; 5] {
    if binding.completion == LockRunnerPreManagedCompletionV1::Completed {
        [1, 0, 1, 1, 1]
    } else {
        [0, 1, 1, 1, 1]
    }
}

fn digest_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-lock-pre-managed-rejection-v1\0");
    hasher.update((values.len() as u64).to_le_bytes());
    for value in values {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!("q9 Lock payload scalar is not canonical"));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_vectors_keep_admission_unchanged_and_adapter_not_reached() {
        let binding = |rejection| LockRunnerPreManagedRejectionBindingV1 {
            rejection,
            completion: if matches!(
                rejection,
                LockRunnerPreManagedRejectionV1::AdmissionRouteUnknown
                    | LockRunnerPreManagedRejectionV1::AdmissionCounterOverflow
            ) {
                LockRunnerPreManagedCompletionV1::Direct
            } else {
                LockRunnerPreManagedCompletionV1::Completed
            },
            action: LockRunnerActionV1::LockShared,
            first: 0,
            count: 1,
            mask: 1,
            normalized_descriptor_sha256: [1; 32],
            case_key_sha256: [2; 32],
            full_record_sha256: [3; 32],
            plan_sha256: [4; 32],
            implementation_sha256: [5; 32],
        };
        assert_eq!(
            lock_effect(binding(
                LockRunnerPreManagedRejectionV1::AdmissionRouteUnknown
            )),
            1
        );
        assert_eq!(
            lock_effect(binding(
                LockRunnerPreManagedRejectionV1::UnsupportedFileRole
            )),
            2
        );
    }
}
