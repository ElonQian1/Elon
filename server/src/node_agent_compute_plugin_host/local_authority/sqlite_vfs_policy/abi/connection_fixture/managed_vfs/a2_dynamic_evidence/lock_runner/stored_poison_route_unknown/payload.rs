//! Canonical q4 payload with an exact ordered route-preemption receipt.

use anyhow::anyhow;
use sha2::{Digest, Sha256};

use super::super::stored_poison::{self, payload as common};
use super::super::LockRunnerStoredPoisonBindingV1;
use super::exact_selector;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::child::lock_stored_poison::route_unknown::{
    REPORT_VALUE_COUNT, REPORT_VERSION,
};

const COMMON_VALUE_COUNT: usize = 135;
const ORDERED_RECEIPT: [u64; 5] = [1, 1, 1, 1, 1];
const TERMINAL_LEDGER: [u64; 18] = [3, 1, 0, 0, 2, 2, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0];

pub(in super::super) fn upgrade_payload(
    binding: LockRunnerStoredPoisonBindingV1,
    q3_payload: &str,
    ordered_receipt: [u64; 5],
) -> anyhow::Result<String> {
    if ordered_receipt != ORDERED_RECEIPT {
        return Err(anyhow!("q4 Lock stored-poison ordered receipt mismatch"));
    }
    let mut fields = q3_payload.split(',');
    let expected_common_selector = common::exact_selector(binding);
    if fields.next()
        != Some(
            crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::child::lock_stored_poison::REPORT_VERSION,
        )
        || fields.next() != Some(expected_common_selector.as_str())
    {
        return Err(anyhow!("q4 Lock stored-poison common payload identity mismatch"));
    }
    let common_values = fields
        .map(common::parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if common_values.len() != COMMON_VALUE_COUNT {
        return Err(anyhow!(
            "q4 Lock stored-poison common payload width mismatch"
        ));
    }
    let mut values = common_values;
    values.extend(ordered_receipt);
    debug_assert_eq!(values.len(), REPORT_VALUE_COUNT);
    Ok(format!(
        "{REPORT_VERSION},{},{}",
        exact_selector(binding),
        values
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    ))
}

pub(in super::super) fn validate_payload(
    payload: &str,
    binding: LockRunnerStoredPoisonBindingV1,
) -> anyhow::Result<stored_poison::ValidatedStoredPoisonPayloadV1> {
    super::validate_binding(binding)?;
    let mut fields = payload.split(',');
    let expected_selector = exact_selector(binding);
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(expected_selector.as_str()) {
        return Err(anyhow!("q4 Lock stored-poison payload identity mismatch"));
    }
    let values = fields
        .map(common::parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[COMMON_VALUE_COUNT..] != ORDERED_RECEIPT {
        return Err(anyhow!(
            "q4 Lock stored-poison receipt width/order mismatch"
        ));
    }
    let registration_id =
        common::validate_common_values(&values[..COMMON_VALUE_COUNT], binding, TERMINAL_LEDGER)?;
    Ok(stored_poison::ValidatedStoredPoisonPayloadV1 {
        registration_id,
        native_receipt_sha256: digest_native_receipt(&values),
    })
}

fn digest_native_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-stored-poison-retention-route-unknown-native-receipt-v4\0");
    for value in &values[22..127] {
        hasher.update(value.to_le_bytes());
    }
    for value in &values[COMMON_VALUE_COUNT..] {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
        child::lock_stored_poison::REPORT_VERSION as Q3_REPORT_VERSION,
        lock_runner::{
            LockRunnerActionV1, LockRunnerStoredPoisonCompletionV1,
            LockRunnerStoredPoisonProfileV1,
        },
    };

    fn binding(completion: LockRunnerStoredPoisonCompletionV1) -> LockRunnerStoredPoisonBindingV1 {
        LockRunnerStoredPoisonBindingV1 {
            action: LockRunnerActionV1::LockExclusive,
            first: 1,
            count: 2,
            mask: 6,
            profile: LockRunnerStoredPoisonProfileV1::FileGrowUncertain,
            completion,
            normalized_descriptor_sha256: [0x11; 32],
            case_key_sha256: [0x22; 32],
            full_record_sha256: [0x33; 32],
            plan_sha256: [0x44; 32],
            implementation_sha256: [0x55; 32],
        }
    }

    fn wire(version: &str, selector: &str, values: &[u64]) -> String {
        format!(
            "{version},{selector},{}",
            values
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    fn canonical_payload(binding: LockRunnerStoredPoisonBindingV1) -> (String, Vec<u64>) {
        let common_values = common::canonical_values_for_test(binding, TERMINAL_LEDGER);
        let q3_payload = wire(
            Q3_REPORT_VERSION,
            &common::exact_selector(binding),
            &common_values,
        );
        let payload = upgrade_payload(binding, &q3_payload, ORDERED_RECEIPT)
            .expect("upgrade canonical q4 payload");
        let mut values = common_values;
        values.extend(ORDERED_RECEIPT);
        (payload, values)
    }

    #[test]
    fn canonical_q4_payload_is_bound_and_native_domain_separated() {
        let binding = binding(LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown);
        let (payload, values) = canonical_payload(binding);
        let validated = validate_payload(&payload, binding).expect("validate canonical q4 payload");
        assert_ne!(
            validated.native_receipt_sha256,
            common::native_receipt_sha256_for_test(&values[..COMMON_VALUE_COUNT])
        );
    }

    #[test]
    fn q4_payload_rejects_each_terminal_and_ordered_receipt_tamper() {
        let binding = binding(LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown);
        let (_, canonical) = canonical_payload(binding);
        for index in (109..127).chain(COMMON_VALUE_COUNT..REPORT_VALUE_COUNT) {
            let mut values = canonical.clone();
            values[index] ^= 1;
            assert!(validate_payload(
                &wire(REPORT_VERSION, &exact_selector(binding), &values),
                binding,
            )
            .is_err());
        }
    }

    #[test]
    fn q4_payload_rejects_cross_identity_completion_and_upgrade_drift() {
        let q4_binding = binding(LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown);
        let (q4_payload, values) = canonical_payload(q4_binding);
        let q3_binding = binding(LockRunnerStoredPoisonCompletionV1::RetentionSucceeded);

        assert!(validate_payload(&q4_payload, q3_binding).is_err());
        assert!(stored_poison::validate_payload(&q4_payload, q3_binding).is_err());
        assert!(validate_payload(
            &wire(Q3_REPORT_VERSION, &exact_selector(q4_binding), &values),
            q4_binding,
        )
        .is_err());
        assert!(validate_payload(
            &wire(REPORT_VERSION, &common::exact_selector(q4_binding), &values,),
            q4_binding,
        )
        .is_err());

        let common_values = &values[..COMMON_VALUE_COUNT];
        let wrong_common_selector = wire(Q3_REPORT_VERSION, "wrong-selector", common_values);
        assert!(upgrade_payload(q4_binding, &wrong_common_selector, ORDERED_RECEIPT).is_err());
        let mut wrong_receipt = ORDERED_RECEIPT;
        wrong_receipt[4] = 0;
        let canonical_common = wire(
            Q3_REPORT_VERSION,
            &common::exact_selector(q4_binding),
            common_values,
        );
        assert!(upgrade_payload(q4_binding, &canonical_common, wrong_receipt).is_err());
    }
}
