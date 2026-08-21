use serde_json::{json, Value};

use super::*;

const VERIFICATION_GOLDEN_DIGEST: &str =
    "5a9f2d79b7bbe2e503ee636d19edeb2e019269cfa65ad50e77a93753a23780c7";
const VERIFICATION_GOLDEN_JSON: &str = concat!(
    r#"{"canonicalization":"rfc8785_jcs","digest_algorithm":"sha256","lineage":{"consumer_review":{"consumer_review_event_digest":"6666666666666666666666666666666666666666666666666666666666666666","consumer_review_id":"consumer-review-v"},"#,
    r#""execution_lineage_digest":"2222222222222222222222222222222222222222222222222222222222222222","#,
    r#""execution_receipt":{"execution_receipt_digest":"1111111111111111111111111111111111111111111111111111111111111111","execution_receipt_id":"execution-receipt-v"},"#,
    r#""platform_observation":{"cumulative_observed_usage_digest":"8888888888888888888888888888888888888888888888888888888888888888","platform_observation_event_digest":"7777777777777777777777777777777777777777777777777777777777777777","platform_observation_id":"platform-observation-v"},"#,
    r#""provider_declared_usage":{"cumulative_usage_digest":"3333333333333333333333333333333333333333333333333333333333333333","usage_event_digest":"4444444444444444444444444444444444444444444444444444444444444444","usage_sequence_no":7,"usage_snapshot_id":"usage-snapshot-v"},"#,
    r#""terminal_candidate":{"terminal_candidate_event_digest":"5555555555555555555555555555555555555555555555555555555555555555","terminal_candidate_id":"terminal-candidate-v"},"#,
    r#""verification_decision":{"compensable_usage_digest":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","verification_decision_id":"verification-v","verification_event_digest":"9999999999999999999999999999999999999999999999999999999999999999","verified_usage_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}},"#,
    r#""lineage_digest":"5a9f2d79b7bbe2e503ee636d19edeb2e019269cfa65ad50e77a93753a23780c7","lineage_kind":"execution_verification_source_v1","schema":"compute_federation.core_historical_causal_reference.v1"}"#,
);

#[test]
fn execution_verification_source_has_literal_canonical_json_and_digest_golden() {
    let carrier = build_execution_verification_source_carrier(verification_lineage()).unwrap();

    assert_eq!(
        carrier.lineage_kind(),
        FederationHistoricalLineageKindV1::ExecutionVerificationSourceV1
    );
    assert_eq!(carrier.lineage_digest(), VERIFICATION_GOLDEN_DIGEST);
    assert_eq!(carrier.canonical_json().unwrap(), VERIFICATION_GOLDEN_JSON);
    assert_eq!(
        canonical_federation_historical_causal_reference_json_and_digest(&carrier).unwrap(),
        (
            VERIFICATION_GOLDEN_JSON.to_string(),
            VERIFICATION_GOLDEN_DIGEST.to_string()
        )
    );
    assert_eq!(
        federation_historical_causal_reference_from_json(VERIFICATION_GOLDEN_JSON).unwrap(),
        carrier
    );
}

#[test]
fn execution_verification_exact_profile_shape_fails_closed() {
    let missing_ref_key = mutate_golden(|value| {
        value["lineage"]["provider_declared_usage"]
            .as_object_mut()
            .unwrap()
            .remove("usage_event_digest");
    });
    let unknown_ref_key = mutate_golden(|value| {
        value["lineage"]["verification_decision"]["decision"] = json!("accepted");
    });
    let unknown_lineage_key = mutate_golden(|value| {
        value["lineage"]["provider"] = json!({});
    });
    let wrong_kind = mutate_golden(|value| {
        value["lineage_kind"] = json!("execution_source_v1");
    });
    let missing_lineage_digest = mutate_golden(|value| {
        value.as_object_mut().unwrap().remove("lineage_digest");
    });

    for invalid in [
        missing_ref_key,
        unknown_ref_key,
        unknown_lineage_key,
        wrong_kind,
        missing_lineage_digest,
    ] {
        assert_rejected(&invalid);
    }
}

#[test]
fn execution_verification_refs_and_sequence_fail_closed() {
    for invalid in [
        mutate_golden(|value| {
            value["lineage"]["provider_declared_usage"]["usage_sequence_no"] = json!(0)
        }),
        mutate_golden(|value| {
            value["lineage"]["provider_declared_usage"]["usage_sequence_no"] =
                json!(9_007_199_254_740_992_u64)
        }),
        mutate_golden(|value| {
            value["lineage"]["provider_declared_usage"]["usage_sequence_no"] = json!(7.0)
        }),
        mutate_golden(|value| {
            value["lineage"]["consumer_review"]["consumer_review_id"] = json!(" review ")
        }),
        mutate_golden(|value| {
            value["lineage"]["platform_observation"]["platform_observation_event_digest"] =
                json!("A".repeat(64))
        }),
        mutate_golden(|value| {
            value["lineage"]["verification_decision"]["verified_usage_digest"] =
                json!("a".repeat(63))
        }),
    ] {
        assert_rejected(&invalid);
    }
}

#[test]
fn execution_verification_noncanonical_bytes_and_role_changes_fail_closed() {
    assert_rejected(&format!(" {VERIFICATION_GOLDEN_JSON}"));
    assert_rejected(&VERIFICATION_GOLDEN_JSON.replacen(
        r#"{"canonicalization":"rfc8785_jcs","digest_algorithm":"sha256","#,
        r#"{"digest_algorithm":"sha256","canonicalization":"rfc8785_jcs","#,
        1,
    ));

    let original = build_execution_verification_source_carrier(verification_lineage()).unwrap();
    let mut changed = verification_lineage();
    changed.consumer_review.consumer_review_event_digest = "c".repeat(64);
    let changed = build_execution_verification_source_carrier(changed).unwrap();
    assert_ne!(original.lineage_digest(), changed.lineage_digest());
}

fn assert_rejected(json: &str) {
    assert!(
        federation_historical_causal_reference_from_json(json).is_err(),
        "unexpectedly accepted: {json}"
    );
}

fn mutate_golden(mutator: impl FnOnce(&mut Value)) -> String {
    let mut value: Value = serde_json::from_str(VERIFICATION_GOLDEN_JSON).unwrap();
    mutator(&mut value);
    serde_json::to_string(&value).unwrap()
}

fn verification_lineage() -> ExecutionVerificationSourceLineageV1 {
    ExecutionVerificationSourceLineageV1 {
        execution_receipt: ExecutionReceiptRef {
            execution_receipt_id: "execution-receipt-v".to_string(),
            execution_receipt_digest: "1".repeat(64),
        },
        execution_lineage_digest: "2".repeat(64),
        provider_declared_usage: ProviderDeclaredUsageRef {
            usage_snapshot_id: "usage-snapshot-v".to_string(),
            usage_sequence_no: 7,
            cumulative_usage_digest: "3".repeat(64),
            usage_event_digest: "4".repeat(64),
        },
        terminal_candidate: TerminalCandidateRef {
            terminal_candidate_id: "terminal-candidate-v".to_string(),
            terminal_candidate_event_digest: "5".repeat(64),
        },
        consumer_review: ConsumerReviewRef {
            consumer_review_id: "consumer-review-v".to_string(),
            consumer_review_event_digest: "6".repeat(64),
        },
        platform_observation: PlatformObservationRef {
            platform_observation_id: "platform-observation-v".to_string(),
            platform_observation_event_digest: "7".repeat(64),
            cumulative_observed_usage_digest: "8".repeat(64),
        },
        verification_decision: VerificationDecisionRef {
            verification_decision_id: "verification-v".to_string(),
            verification_event_digest: "9".repeat(64),
            verified_usage_digest: "a".repeat(64),
            compensable_usage_digest: "b".repeat(64),
        },
    }
}
