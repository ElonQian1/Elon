use super::*;
use crate::esk_asset::platform::payment_identity::{payment_key, source_fingerprint};

#[test]
fn payment_identity_matches_existing_javascript_synthetic_vector() {
    // Produced by scripts/esk-paid-reconciliation/identity.js from the committed fixture.
    assert_eq!(
        source_fingerprint(&source()).unwrap(),
        "6f7b787b0f4451feba5ea00e4d1cf5be6f039ff026ae470768155afb0c902a64"
    );
    assert_eq!(
        payment_key(&source(), &"a".repeat(64), 0).unwrap(),
        "9010aeb92a6ec2a5ca08919cd3593d1ae9882048018bf12b38cc9bbe8e8c7be3"
    );
    assert_eq!(
        payment_key(&source(), &"a".repeat(64), 0).unwrap(),
        payment_key(&source(), &format!("0X{}", "A".repeat(64)), 0).unwrap()
    );
    assert_ne!(
        payment_key(&source(), &"a".repeat(64), 0).unwrap(),
        payment_key(&source(), &"a".repeat(64), 1).unwrap()
    );
}

#[test]
fn hex_asset_aliases_normalize_but_opaque_references_remain_case_sensitive() {
    let mut first = source();
    first.asset_reference = "0XAb".into();
    let mut second = first.clone();
    second.asset_reference = format!("0x{}ab", "0".repeat(62));
    assert_eq!(
        source_fingerprint(&first).unwrap(),
        source_fingerprint(&second).unwrap()
    );
    first.reference_format = "opaque".into();
    assert_ne!(
        payment_key(&first, "Credit-ABC", 0).unwrap(),
        payment_key(&first, "Credit-abc", 0).unwrap()
    );
}

#[test]
fn policy_rejects_invalid_source_and_nonpositive_or_overflow_limits() {
    for limit in ["0", "-1", "+1", "01", "1.0", "1e6", "9223372036854775808"] {
        assert_error(
            validate_policy(PolicyBody {
                source: source(),
                issuance_limit_base_units: limit.into(),
            }),
            PlatformError::InvalidPolicy,
        );
    }
    let mut invalid = source();
    invalid.asset_symbol = "BTC".into();
    assert!(validate_policy(PolicyBody {
        source: invalid,
        issuance_limit_base_units: "10000000".into()
    })
    .is_err());
    let mut invalid = source();
    invalid.decimals = 19;
    assert!(validate_policy(PolicyBody {
        source: invalid,
        issuance_limit_base_units: "10000000".into()
    })
    .is_err());
}

#[test]
fn absent_configuration_is_disabled_and_invalid_modes_cannot_open_writes() {
    use crate::esk_asset::platform::validation::policy_from_values;
    assert_error(policy_from_values(None, None), PlatformError::Disabled);
    assert_error(
        policy_from_values(Some("disabled"), Some("malformed")),
        PlatformError::Disabled,
    );
    for mode in ["paper", "live", "true", " platform_recorded"] {
        assert_error(
            policy_from_values(Some(mode), None),
            PlatformError::InvalidPolicy,
        );
    }
    assert_error(
        policy_from_values(Some("platform_recorded"), None),
        PlatformError::InvalidPolicy,
    );
    assert_error(
        policy_from_values(Some("platform_recorded"), Some("{}")),
        PlatformError::InvalidPolicy,
    );
    let json = serde_json::to_string(&PolicyBody {
        source: source(),
        issuance_limit_base_units: "100000000".into(),
    })
    .unwrap();
    assert_eq!(
        policy_from_values(Some("platform_recorded"), Some(&json))
            .unwrap()
            .policy_digest,
        policy(100000000).policy_digest
    );
}

#[test]
fn normalization_preserves_request_identity_and_never_stores_raw_payment_reference() {
    let policy = policy(100000000);
    let expected = input(&policy);
    let mut equivalent = body();
    equivalent.amount = "10".into();
    equivalent.payment_amount = "20.0".into();
    equivalent.external_payment_reference = format!("0x{}", "A".repeat(64));
    assert_eq!(prepare_input(&policy, equivalent).unwrap(), expected);
    let encoded = serde_json::to_value(&expected).unwrap();
    assert!(encoded.get("external_payment_reference").is_none());
    assert!(encoded.get("review_reference").is_none());
    assert!(!encoded.to_string().contains("synthetic-review"));
    assert_eq!(expected.amount_base_units, 10000000);
    assert_eq!(expected.payment_base_units, "20000000");
}

#[test]
fn amounts_reject_precision_signs_exponent_and_overflow_without_rounding() {
    let policy = policy(i64::MAX);
    for amount in [
        "0",
        "-1",
        "+1",
        "01",
        "1e1",
        " 10",
        "10 ",
        "10.0000001",
        "9223372036854.775808",
    ] {
        let mut value = body();
        value.amount = amount.into();
        assert!(prepare_input(&policy, value).is_err(), "accepted {amount}");
    }
    for amount in [
        "0",
        "-20",
        "+20",
        "020",
        "2e1",
        "20.0000001",
        "340282366920938463463374607431768211456",
    ] {
        let mut value = body();
        value.payment_amount = amount.into();
        assert!(prepare_input(&policy, value).is_err(), "accepted {amount}");
    }
}

#[test]
fn exact_sale_ratio_and_required_review_materials_are_not_inferred() {
    let policy = policy(100000000);
    let mut mismatch = body();
    mismatch.amount = "11".into();
    assert!(prepare_input(&policy, mismatch).is_err());
    let mut fraction = body();
    fraction.payment_amount = "0.000003".into();
    fraction.amount = "0.000001".into();
    assert!(prepare_input(&policy, fraction).is_err());
    let mut unconfirmed = body();
    unconfirmed.history_complete = false;
    assert!(prepare_input(&policy, unconfirmed).is_err());
    for purpose in ["service_purchase", "quant_subscription", "unconfirmed"] {
        let mut value = body();
        value.commercial_purpose = purpose.into();
        assert!(prepare_input(&policy, value).is_err());
    }
    let mut missing = body();
    missing.consent_digest.clear();
    assert!(prepare_input(&policy, missing).is_err());
    let mut missing = body();
    missing.payment_evidence_digest = "not-a-digest".into();
    assert!(prepare_input(&policy, missing).is_err());
}

#[test]
fn reduction_handles_u128_payment_and_i64_esk_boundary_without_intermediate_overflow() {
    let maximum = u128::MAX.to_string();
    let mut source = source();
    source.decimals = 0;
    let policy = validate_policy(PolicyBody {
        source,
        issuance_limit_base_units: i64::MAX.to_string(),
    })
    .unwrap();
    let mut value = body();
    value.payment_amount = maximum.clone();
    value.amount = "9223372036854.775807".into();
    value.sale.payment_base_units_per_lot = maximum;
    value.sale.esk_base_units_per_lot = i64::MAX.to_string();
    assert_eq!(
        prepare_input(&policy, value).unwrap().amount_base_units,
        i64::MAX
    );
}

#[test]
fn same_payment_modified_user_quantity_or_evidence_changes_request_not_payment_key() {
    let policy = policy(100000000);
    let expected = input(&policy);
    let mut changes = Vec::new();
    let mut user = body();
    user.user_id = "bob".into();
    changes.push(user);
    let mut amount = body();
    amount.amount = "20".into();
    amount.payment_amount = "40".into();
    changes.push(amount);
    let mut evidence = body();
    evidence.payment_evidence_digest = "7".repeat(64);
    changes.push(evidence);
    let mut consent = body();
    consent.consent_digest = "8".repeat(64);
    changes.push(consent);
    let mut history = body();
    history.history_evidence_digest = "9".repeat(64);
    changes.push(history);
    let mut terms = body();
    terms.sale.terms_digest = "a".repeat(64);
    changes.push(terms);
    for value in changes {
        let changed = prepare_input(&policy, value).unwrap();
        assert_eq!(changed.payment_key, expected.payment_key);
        assert_ne!(changed.request_digest, expected.request_digest);
    }
}

#[test]
fn prepared_struct_tampering_and_unknown_or_wrong_type_json_are_rejected() {
    let policy = policy(100000000);
    let mut changed = input(&policy);
    changed.amount_base_units += 1;
    assert!(validate_prepared_input(&policy, &changed).is_err());
    let mut changed = input(&policy);
    changed.source_fingerprint = "a".repeat(64);
    assert!(validate_prepared_input(&policy, &changed).is_err());
    let mut unknown = body_json();
    unknown["unknown"] = serde_json::json!("forbidden");
    assert!(serde_json::from_value::<PrepareBody>(unknown).is_err());
    let mut wrong_type = body_json();
    wrong_type["amount"] = serde_json::json!(10);
    assert!(serde_json::from_value::<PrepareBody>(wrong_type).is_err());
    let source = serde_json::to_value(source()).unwrap();
    assert!(serde_json::from_value::<PolicyBody>(serde_json::json!({
        "source":source, "issuance_limit_base_units":100000000,
    }))
    .is_err());
}
