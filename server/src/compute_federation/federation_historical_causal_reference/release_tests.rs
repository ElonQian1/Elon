use serde_json::{json, Value};

use super::*;

const SETTLEMENT_LINEAGE_DIGEST: &str =
    "1111111111111111111111111111111111111111111111111111111111111111";
const RELEASE_GATE_DIGEST: &str =
    "2222222222222222222222222222222222222222222222222222222222222222";
const RELEASE_GOLDEN_DIGEST: &str =
    "96f1377ff17c099419ce60a60778cbac00028b1cfba2fd4384adfc8f5ed9dae4";
const RELEASE_GOLDEN_JSON: &str = concat!(
    r#"{"canonicalization":"rfc8785_jcs","digest_algorithm":"sha256","lineage":{"attempt_settlement":{"settlement_event_digest":"4444444444444444444444444444444444444444444444444444444444444444","settlement_receipt_digest":"3333333333333333333333333333333333333333333333333333333333333333","settlement_receipt_id":"settlement-receipt-r"},"#,
    r#""release_gate":{"challenge":{"settlement_challenge_event_digest":"8888888888888888888888888888888888888888888888888888888888888888","settlement_challenge_id":"challenge-r"},"challenge_gate_digest":"2222222222222222222222222222222222222222222222222222222222222222","correction":{"settlement_correction_event_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","settlement_correction_id":"correction-r"},"correction_posting":{"settlement_correction_posting_digest":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","settlement_correction_posting_id":"correction-posting-r"},"gate_kind":"accepted_corrected","resolution":{"settlement_challenge_resolution_event_digest":"9999999999999999999999999999999999999999999999999999999999999999","settlement_challenge_resolution_id":"resolution-r"}},"#,
    r#""release_posting":{"settlement_release_posting_digest":"7777777777777777777777777777777777777777777777777777777777777777","settlement_release_posting_id":"release-posting-r"},"settlement_lineage_digest":"1111111111111111111111111111111111111111111111111111111111111111","settlement_release":{"settlement_release_event_digest":"6666666666666666666666666666666666666666666666666666666666666666","settlement_release_id":"release-r"},"source_settlement_posting":{"settlement_posting_digest":"5555555555555555555555555555555555555555555555555555555555555555","settlement_posting_id":"source-posting-r"}},"#,
    r#""lineage_digest":"96f1377ff17c099419ce60a60778cbac00028b1cfba2fd4384adfc8f5ed9dae4","lineage_kind":"settlement_release_source_v1","schema":"compute_federation.core_historical_causal_reference.v1"}"#,
);

#[test]
fn settlement_release_source_has_literal_canonical_json_and_digest_golden() {
    let carrier = build_settlement_release_source_carrier(release_lineage()).unwrap();

    assert_eq!(
        carrier.lineage_kind(),
        FederationHistoricalLineageKindV1::SettlementReleaseSourceV1
    );
    assert_eq!(carrier.lineage_digest(), RELEASE_GOLDEN_DIGEST);
    assert_eq!(carrier.canonical_json().unwrap(), RELEASE_GOLDEN_JSON);
    assert_eq!(
        canonical_federation_historical_causal_reference_json_and_digest(&carrier).unwrap(),
        (
            RELEASE_GOLDEN_JSON.to_string(),
            RELEASE_GOLDEN_DIGEST.to_string()
        )
    );
    assert_eq!(
        federation_historical_causal_reference_from_json(RELEASE_GOLDEN_JSON).unwrap(),
        carrier
    );
    assert_eq!(
        federation_historical_causal_reference_from_json_bytes(RELEASE_GOLDEN_JSON.as_bytes())
            .unwrap(),
        carrier
    );
}

#[test]
fn release_lineage_has_exact_six_keys_and_non_null_gate_profiles() {
    let accepted = build_settlement_release_source_carrier(release_lineage()).unwrap();
    let accepted_value: Value = serde_json::from_str(&accepted.canonical_json().unwrap()).unwrap();
    assert_eq!(
        accepted_value["lineage"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "attempt_settlement",
            "release_gate",
            "release_posting",
            "settlement_lineage_digest",
            "settlement_release",
            "source_settlement_posting",
        ]
    );
    assert_exact_gate_shape(&accepted_value, "accepted_corrected", 6);

    let mut no_challenge = release_lineage();
    no_challenge.release_gate = SettlementReleaseGateV1::NoChallenge {
        challenge_gate_digest: RELEASE_GATE_DIGEST.to_string(),
    };
    let no_challenge = build_settlement_release_source_carrier(no_challenge).unwrap();
    let no_challenge: Value =
        serde_json::from_str(&no_challenge.canonical_json().unwrap()).unwrap();
    assert_exact_gate_shape(&no_challenge, "no_challenge", 2);

    for action in [
        SettlementChallengeResolutionActionV1::Rejected,
        SettlementChallengeResolutionActionV1::Withdrawn,
    ] {
        let mut resolved = release_lineage();
        resolved.release_gate = SettlementReleaseGateV1::ResolvedChallenge {
            challenge_gate_digest: RELEASE_GATE_DIGEST.to_string(),
            resolution_action: action,
            challenge: challenge_ref(),
            resolution: resolution_ref(),
        };
        let resolved = build_settlement_release_source_carrier(resolved).unwrap();
        let resolved: Value = serde_json::from_str(&resolved.canonical_json().unwrap()).unwrap();
        assert_exact_gate_shape(&resolved, "resolved_challenge", 5);
    }
}

#[test]
fn release_gate_action_and_digest_contracts_fail_closed() {
    let mut resolved_gate = serde_json::to_value(SettlementReleaseGateV1::ResolvedChallenge {
        challenge_gate_digest: RELEASE_GATE_DIGEST.to_string(),
        resolution_action: SettlementChallengeResolutionActionV1::Rejected,
        challenge: challenge_ref(),
        resolution: resolution_ref(),
    })
    .unwrap();
    resolved_gate["resolution_action"] = json!("accepted");
    assert!(serde_json::from_value::<SettlementReleaseGateV1>(resolved_gate).is_err());

    let mut corrected_gate = serde_json::to_value(release_lineage().release_gate).unwrap();
    corrected_gate["resolution_action"] = json!("rejected");
    assert!(serde_json::from_value::<SettlementReleaseGateV1>(corrected_gate).is_err());

    let mut bad_gate_digest = release_lineage();
    bad_gate_digest.release_gate = SettlementReleaseGateV1::NoChallenge {
        challenge_gate_digest: "A".repeat(64),
    };
    assert!(build_settlement_release_source_carrier(bad_gate_digest).is_err());

    let mut bad_parent_digest = release_lineage();
    bad_parent_digest.settlement_lineage_digest = "1".repeat(63);
    assert!(build_settlement_release_source_carrier(bad_parent_digest).is_err());

    let mut bad_id = release_lineage();
    bad_id.settlement_release.settlement_release_id = " release-r".to_string();
    assert!(build_settlement_release_source_carrier(bad_id).is_err());

    let mut bad_native_digest = release_lineage();
    bad_native_digest
        .source_settlement_posting
        .settlement_posting_digest = "A".repeat(64);
    assert!(build_settlement_release_source_carrier(bad_native_digest).is_err());
}

#[test]
fn release_shape_kind_and_correction_posting_fail_closed() {
    let invalid = [
        mutate_release(|value| value["lineage_kind"] = json!("settlement_source_v1")),
        mutate_release(|value| value["lineage"]["release_gate"] = Value::Null),
        mutate_release(|value| {
            value["lineage"]["release_gate"]
                .as_object_mut()
                .unwrap()
                .remove("correction_posting");
        }),
        mutate_release(|value| {
            value["lineage"]["release_gate"]["correction_posting"] = Value::Null;
        }),
        mutate_release(|value| {
            value["lineage"]["release_gate"]["correction_posting"]["owner"] = json!("caller");
        }),
        mutate_release(|value| value["lineage"]["seventh_key"] = json!(true)),
    ];
    for json in invalid {
        assert!(
            federation_historical_causal_reference_from_json(&json).is_err(),
            "unexpectedly accepted: {json}"
        );
    }
}

fn assert_exact_gate_shape(value: &Value, variant: &str, payload_keys: usize) {
    let gate = value["lineage"]["release_gate"].as_object().unwrap();
    assert_eq!(gate.len(), payload_keys);
    assert_eq!(gate["gate_kind"], variant);
    assert!(gate.values().all(|value| !value.is_null()));
}

fn mutate_release(mutator: impl FnOnce(&mut Value)) -> String {
    let mut value: Value = serde_json::from_str(RELEASE_GOLDEN_JSON).unwrap();
    mutator(&mut value);
    serde_json::to_string(&value).unwrap()
}

fn release_lineage() -> SettlementReleaseSourceLineageV1 {
    SettlementReleaseSourceLineageV1 {
        attempt_settlement: AttemptSettlementRef {
            settlement_receipt_id: "settlement-receipt-r".to_string(),
            settlement_receipt_digest: "3".repeat(64),
            settlement_event_digest: "4".repeat(64),
        },
        settlement_lineage_digest: SETTLEMENT_LINEAGE_DIGEST.to_string(),
        source_settlement_posting: SettlementSourcePostingRef {
            settlement_posting_id: "source-posting-r".to_string(),
            settlement_posting_digest: "5".repeat(64),
        },
        release_gate: SettlementReleaseGateV1::AcceptedCorrected {
            challenge_gate_digest: RELEASE_GATE_DIGEST.to_string(),
            challenge: challenge_ref(),
            resolution: resolution_ref(),
            correction: correction_ref(),
            correction_posting: correction_posting_ref(),
        },
        settlement_release: SettlementReleaseRef {
            settlement_release_id: "release-r".to_string(),
            settlement_release_event_digest: "6".repeat(64),
        },
        release_posting: SettlementReleasePostingRef {
            settlement_release_posting_id: "release-posting-r".to_string(),
            settlement_release_posting_digest: "7".repeat(64),
        },
    }
}

fn challenge_ref() -> SettlementChallengeRef {
    SettlementChallengeRef {
        settlement_challenge_id: "challenge-r".to_string(),
        settlement_challenge_event_digest: "8".repeat(64),
    }
}

fn resolution_ref() -> SettlementChallengeResolutionRef {
    SettlementChallengeResolutionRef {
        settlement_challenge_resolution_id: "resolution-r".to_string(),
        settlement_challenge_resolution_event_digest: "9".repeat(64),
    }
}

fn correction_ref() -> SettlementCorrectionRef {
    SettlementCorrectionRef {
        settlement_correction_id: "correction-r".to_string(),
        settlement_correction_event_digest: "a".repeat(64),
    }
}

fn correction_posting_ref() -> SettlementCorrectionPostingRef {
    SettlementCorrectionPostingRef {
        settlement_correction_posting_id: "correction-posting-r".to_string(),
        settlement_correction_posting_digest: "b".repeat(64),
    }
}
