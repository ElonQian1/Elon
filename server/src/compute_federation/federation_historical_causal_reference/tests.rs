use serde_json::{json, Value};

use super::*;

const EXECUTION_GOLDEN_DIGEST: &str =
    "99fe71d443ca71f763d79c54e248f22357d799db3d6af058e060f4f0038c25b5";
const EXECUTION_GOLDEN_JSON: &str = concat!(
    r#"{"canonicalization":"rfc8785_jcs","digest_algorithm":"sha256","lineage":{"attempt_lease_source":{"fencing_generation":9,"lease_digest":"LeaseDigest-A","lease_id":"lease-1","lease_revision":8},"#,
    r#""capacity_claim":{"claim_digest":"ClaimDigest-A","claim_id":"claim-1","claim_revision":7},"#,
    r#""capacity_pool":{"capacity_epoch":2,"pool_digest":"PoolDigest-A","pool_id":"pool-1","pool_revision":3},"#,
    r#""execution_receipt":{"execution_receipt_digest":"ExecutionReceiptDigest-A","execution_receipt_id":"execution-receipt-1"},"#,
    r#""job":{"job_digest":"JobDigest-A","job_id":"job-1","job_revision":6},"#,
    r#""offer":{"offer_digest":"OfferDigest-A","offer_id":"offer-1","offer_version":4,"provider_id":"provider-1"},"#,
    r#""price_snapshot":{"price_snapshot_digest":"SnapshotDigest-A","price_snapshot_id":"snapshot-1"},"#,
    r#""provider":{"policy_revision":1,"provider_digest":"ProviderDigest-A","provider_id":"provider-1"},"#,
    r#""reservation":{"reservation_digest":"ReservationDigest-A","reservation_id":"reservation-1","reservation_revision":5}},"#,
    r#""lineage_digest":"99fe71d443ca71f763d79c54e248f22357d799db3d6af058e060f4f0038c25b5","lineage_kind":"execution_source_v1","schema":"compute_federation.core_historical_causal_reference.v1"}"#,
);

const SETTLEMENT_GOLDEN_DIGEST: &str =
    "161d46268e8bfd5a034fe8d4b1d780792d8d8e3a9fbbafeec628dab20de22d81";
const SETTLEMENT_GOLDEN_JSON: &str = concat!(
    r#"{"canonicalization":"rfc8785_jcs","digest_algorithm":"sha256","lineage":{"attempt_settlement":{"settlement_event_digest":"SettlementEventDigest-B","settlement_receipt_digest":"SettlementReceiptDigest-B","settlement_receipt_id":"settlement-receipt-1"},"#,
    r#""execution_lineage_digest":"99fe71d443ca71f763d79c54e248f22357d799db3d6af058e060f4f0038c25b5","#,
    r#""execution_receipt":{"execution_receipt_digest":"ExecutionReceiptDigest-A","execution_receipt_id":"execution-receipt-1"},"#,
    r#""finalization":{"finalization_event_digest":"FinalizationEventDigest-B","finalization_id":"finalization-1"},"#,
    r#""price_snapshot":{"price_snapshot_digest":"SnapshotDigest-A","price_snapshot_id":"snapshot-1"},"#,
    r#""provider":{"policy_revision":1,"provider_digest":"ProviderDigest-A","provider_id":"provider-1"},"#,
    r#""source_job":{"job_digest":"JobDigest-A","job_id":"job-1","job_revision":6},"#,
    r#""terminal_job":{"job_digest":"TerminalJobDigest-B","job_id":"job-1","job_revision":10},"#,
    r#""terminal_reservation":{"reservation_digest":"TerminalReservationDigest-B","reservation_id":"reservation-1","reservation_revision":11}},"#,
    r#""lineage_digest":"161d46268e8bfd5a034fe8d4b1d780792d8d8e3a9fbbafeec628dab20de22d81","lineage_kind":"settlement_source_v1","schema":"compute_federation.core_historical_causal_reference.v1"}"#,
);

#[test]
fn execution_source_has_literal_canonical_json_and_digest_golden() {
    let carrier = build_execution_source_carrier(execution_lineage()).unwrap();

    assert_eq!(
        carrier.lineage_kind(),
        FederationHistoricalLineageKindV1::ExecutionSourceV1
    );
    assert_eq!(carrier.lineage_digest(), EXECUTION_GOLDEN_DIGEST);
    assert_eq!(carrier.canonical_json().unwrap(), EXECUTION_GOLDEN_JSON);
    assert_eq!(
        canonical_federation_historical_causal_reference_json_and_digest(&carrier).unwrap(),
        (
            EXECUTION_GOLDEN_JSON.to_string(),
            EXECUTION_GOLDEN_DIGEST.to_string()
        )
    );
    assert_eq!(
        federation_historical_causal_reference_from_json(EXECUTION_GOLDEN_JSON).unwrap(),
        carrier
    );
    assert_eq!(
        federation_historical_causal_reference_from_json_bytes(EXECUTION_GOLDEN_JSON.as_bytes())
            .unwrap(),
        carrier
    );
}

#[test]
fn settlement_source_has_literal_canonical_json_and_digest_golden() {
    let carrier = build_settlement_source_carrier(settlement_lineage()).unwrap();

    assert_eq!(
        carrier.lineage_kind(),
        FederationHistoricalLineageKindV1::SettlementSourceV1
    );
    assert_eq!(carrier.lineage_digest(), SETTLEMENT_GOLDEN_DIGEST);
    assert_eq!(carrier.canonical_json().unwrap(), SETTLEMENT_GOLDEN_JSON);
    assert_eq!(
        canonical_federation_historical_causal_reference_json_and_digest(&carrier).unwrap(),
        (
            SETTLEMENT_GOLDEN_JSON.to_string(),
            SETTLEMENT_GOLDEN_DIGEST.to_string()
        )
    );
    assert_eq!(
        federation_historical_causal_reference_from_json(SETTLEMENT_GOLDEN_JSON).unwrap(),
        carrier
    );
}

#[test]
fn exact_envelope_and_profile_shapes_fail_closed() {
    for invalid in [
        mutate_execution(|value| value["schema"] = json!("wrong")),
        mutate_execution(|value| value["canonicalization"] = json!("serde_json")),
        mutate_execution(|value| value["digest_algorithm"] = json!("SHA-256")),
        mutate_execution(|value| value["lineage_kind"] = json!("settlement_source_v1")),
        mutate_execution(|value| value["lineage_kind"] = Value::Null),
        mutate_execution(|value| value["lineage"] = json!([])),
        mutate_execution(|value| value["lineage"] = Value::Null),
    ] {
        assert_rejected(&invalid);
    }

    let missing_digest = mutate_execution(|value| {
        value.as_object_mut().unwrap().remove("lineage_digest");
    });
    let unknown_top_level = mutate_execution(|value| {
        value["actor"] = json!("caller");
    });
    let unknown_profile_key = mutate_execution(|value| {
        value["lineage"]["attempt_settlement"] = json!({});
    });
    let missing_ref_key = mutate_execution(|value| {
        value["lineage"]["capacity_pool"]
            .as_object_mut()
            .unwrap()
            .remove("capacity_epoch");
    });
    for invalid in [
        missing_digest,
        unknown_top_level,
        unknown_profile_key,
        missing_ref_key,
    ] {
        assert_rejected(&invalid);
    }

    let duplicate_schema =
        EXECUTION_GOLDEN_JSON.replacen(r#""schema":"#, r#""schema":"duplicate","schema":"#, 1);
    assert_rejected(&duplicate_schema);
}

#[test]
fn noncanonical_or_non_utf8_input_bytes_fail_closed() {
    let leading_whitespace = format!(" {EXECUTION_GOLDEN_JSON}");
    let trailing_whitespace = format!("{EXECUTION_GOLDEN_JSON}\n");
    let reordered = EXECUTION_GOLDEN_JSON.replacen(
        r#"{"canonicalization":"rfc8785_jcs","digest_algorithm":"sha256","#,
        r#"{"digest_algorithm":"sha256","canonicalization":"rfc8785_jcs","#,
        1,
    );
    let escaped_equivalent = EXECUTION_GOLDEN_JSON.replace("provider-1", r"provider-\u0031");

    for invalid in [
        leading_whitespace,
        trailing_whitespace,
        reordered,
        escaped_equivalent,
    ] {
        assert_rejected(&invalid);
    }
    assert!(federation_historical_causal_reference_from_json_bytes(&[0xff]).is_err());
    assert!(
        federation_historical_causal_reference_from_json_bytes(&vec![
            b' ';
            FEDERATION_HISTORICAL_CAUSAL_REFERENCE_MAX_JSON_BYTES
                + 1
        ])
        .is_err()
    );
}

#[test]
fn all_integer_roles_enforce_positive_ijson_safe_numbers() {
    let invalid_numbers = [
        mutate_execution(|value| value["lineage"]["provider"]["policy_revision"] = json!(0)),
        mutate_execution(|value| value["lineage"]["capacity_pool"]["capacity_epoch"] = json!(0)),
        mutate_execution(|value| value["lineage"]["capacity_pool"]["pool_revision"] = json!(-1)),
        mutate_execution(|value| value["lineage"]["offer"]["offer_version"] = json!(4.0)),
        mutate_execution(|value| {
            value["lineage"]["job"]["job_revision"] = json!(9_007_199_254_740_992_u64)
        }),
        mutate_execution(|value| {
            value["lineage"]["reservation"]["reservation_revision"] = json!("5")
        }),
        mutate_execution(|value| value["lineage"]["capacity_claim"]["claim_revision"] = json!(0)),
        mutate_execution(|value| {
            value["lineage"]["attempt_lease_source"]["lease_revision"] = json!(0)
        }),
        mutate_execution(|value| {
            value["lineage"]["attempt_lease_source"]["fencing_generation"] = json!(0)
        }),
        mutate_settlement(|value| value["lineage"]["source_job"]["job_revision"] = json!(0)),
        mutate_settlement(|value| value["lineage"]["terminal_job"]["job_revision"] = json!(0)),
        mutate_settlement(|value| {
            value["lineage"]["terminal_reservation"]["reservation_revision"] = json!(0)
        }),
    ];
    for invalid in invalid_numbers {
        assert_rejected(&invalid);
    }

    assert_rejected(&EXECUTION_GOLDEN_JSON.replacen(
        r#""policy_revision":1"#,
        r#""policy_revision":1e0"#,
        1,
    ));
}

#[test]
fn ids_and_digests_keep_exact_json_types_and_carrier_digest_rules() {
    for invalid in [
        mutate_execution(|value| value["lineage"]["provider"]["provider_id"] = json!(1)),
        mutate_execution(|value| value["lineage"]["provider"]["provider_digest"] = Value::Null),
        mutate_execution(|value| value["lineage_digest"] = json!("0".repeat(64))),
        mutate_execution(|value| value["lineage_digest"] = json!("A".repeat(64))),
        mutate_settlement(|value| {
            value["lineage"]["execution_lineage_digest"] = json!("A".repeat(64))
        }),
        mutate_settlement(|value| {
            value["lineage"]["execution_lineage_digest"] = json!("0".repeat(63))
        }),
    ] {
        assert_rejected(&invalid);
    }
}

#[test]
fn native_owner_strings_remain_opaque_and_are_never_normalized() {
    let mut spaced = execution_lineage();
    spaced.provider.provider_digest = "  OwNeR-Digest  ".to_string();
    spaced.provider.provider_id = "caf\u{e9}".to_string();
    let spaced_carrier = build_execution_source_carrier(spaced).unwrap();
    let FederationHistoricalLineageV1::ExecutionSource(spaced_lineage) = spaced_carrier.lineage()
    else {
        panic!("execution builder returned the wrong profile")
    };
    assert_eq!(spaced_lineage.provider.provider_digest, "  OwNeR-Digest  ");
    assert_eq!(spaced_lineage.provider.provider_id, "caf\u{e9}");

    let mut decomposed = execution_lineage();
    decomposed.provider.provider_id = "cafe\u{301}".to_string();
    let decomposed_carrier = build_execution_source_carrier(decomposed).unwrap();
    assert_ne!(
        spaced_carrier.lineage_digest(),
        decomposed_carrier.lineage_digest()
    );
}

#[test]
fn kind_and_role_changes_produce_distinct_carrier_digests() {
    let execution = build_execution_source_carrier(execution_lineage()).unwrap();
    let settlement = build_settlement_source_carrier(settlement_lineage()).unwrap();
    assert_ne!(execution.lineage_digest(), settlement.lineage_digest());

    let mut swapped_lineage = settlement_lineage();
    std::mem::swap(
        &mut swapped_lineage.source_job,
        &mut swapped_lineage.terminal_job,
    );
    let swapped = build_settlement_source_carrier(swapped_lineage).unwrap();
    assert_ne!(settlement.lineage_digest(), swapped.lineage_digest());
}

fn assert_rejected(json: &str) {
    assert!(
        federation_historical_causal_reference_from_json(json).is_err(),
        "unexpectedly accepted: {json}"
    );
}

fn mutate_execution(mutator: impl FnOnce(&mut Value)) -> String {
    mutate(EXECUTION_GOLDEN_JSON, mutator)
}

fn mutate_settlement(mutator: impl FnOnce(&mut Value)) -> String {
    mutate(SETTLEMENT_GOLDEN_JSON, mutator)
}

fn mutate(golden: &str, mutator: impl FnOnce(&mut Value)) -> String {
    let mut value: Value = serde_json::from_str(golden).unwrap();
    mutator(&mut value);
    serde_json::to_string(&value).unwrap()
}

fn execution_lineage() -> ExecutionSourceLineageV1 {
    ExecutionSourceLineageV1 {
        execution_receipt: ExecutionReceiptRef {
            execution_receipt_id: "execution-receipt-1".to_string(),
            execution_receipt_digest: "ExecutionReceiptDigest-A".to_string(),
        },
        provider: ProviderVersionRef {
            provider_id: "provider-1".to_string(),
            policy_revision: 1,
            provider_digest: "ProviderDigest-A".to_string(),
        },
        capacity_pool: CapacityPoolVersionRef {
            pool_id: "pool-1".to_string(),
            capacity_epoch: 2,
            pool_revision: 3,
            pool_digest: "PoolDigest-A".to_string(),
        },
        offer: OfferVersionRef {
            provider_id: "provider-1".to_string(),
            offer_id: "offer-1".to_string(),
            offer_version: 4,
            offer_digest: "OfferDigest-A".to_string(),
        },
        price_snapshot: PriceSnapshotRef {
            price_snapshot_id: "snapshot-1".to_string(),
            price_snapshot_digest: "SnapshotDigest-A".to_string(),
        },
        job: JobVersionRef {
            job_id: "job-1".to_string(),
            job_revision: 6,
            job_digest: "JobDigest-A".to_string(),
        },
        reservation: ReservationVersionRef {
            reservation_id: "reservation-1".to_string(),
            reservation_revision: 5,
            reservation_digest: "ReservationDigest-A".to_string(),
        },
        capacity_claim: CapacityClaimVersionRef {
            claim_id: "claim-1".to_string(),
            claim_revision: 7,
            claim_digest: "ClaimDigest-A".to_string(),
        },
        attempt_lease_source: AttemptLeaseSourceRef {
            lease_id: "lease-1".to_string(),
            lease_revision: 8,
            lease_digest: "LeaseDigest-A".to_string(),
            fencing_generation: 9,
        },
    }
}

fn settlement_lineage() -> SettlementSourceLineageV1 {
    SettlementSourceLineageV1 {
        attempt_settlement: AttemptSettlementRef {
            settlement_receipt_id: "settlement-receipt-1".to_string(),
            settlement_receipt_digest: "SettlementReceiptDigest-B".to_string(),
            settlement_event_digest: "SettlementEventDigest-B".to_string(),
        },
        execution_receipt: ExecutionReceiptRef {
            execution_receipt_id: "execution-receipt-1".to_string(),
            execution_receipt_digest: "ExecutionReceiptDigest-A".to_string(),
        },
        execution_lineage_digest: EXECUTION_GOLDEN_DIGEST.to_string(),
        finalization: FinalizationRef {
            finalization_id: "finalization-1".to_string(),
            finalization_event_digest: "FinalizationEventDigest-B".to_string(),
        },
        price_snapshot: PriceSnapshotRef {
            price_snapshot_id: "snapshot-1".to_string(),
            price_snapshot_digest: "SnapshotDigest-A".to_string(),
        },
        provider: ProviderVersionRef {
            provider_id: "provider-1".to_string(),
            policy_revision: 1,
            provider_digest: "ProviderDigest-A".to_string(),
        },
        source_job: JobVersionRef {
            job_id: "job-1".to_string(),
            job_revision: 6,
            job_digest: "JobDigest-A".to_string(),
        },
        terminal_job: JobVersionRef {
            job_id: "job-1".to_string(),
            job_revision: 10,
            job_digest: "TerminalJobDigest-B".to_string(),
        },
        terminal_reservation: ReservationVersionRef {
            reservation_id: "reservation-1".to_string(),
            reservation_revision: 11,
            reservation_digest: "TerminalReservationDigest-B".to_string(),
        },
    }
}
