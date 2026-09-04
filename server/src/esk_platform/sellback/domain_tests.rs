use super::*;

pub(crate) fn fixture_policy(user: &str, source: &str) -> SellbackPolicy {
    let terms = "Synthetic local request test. No price, payment or return promise.";
    validate_policy(SellbackPolicyBody {
        schema: POLICY_SCHEMA.into(),
        revision: "synthetic-v1".into(),
        approval_digest: "a".repeat(64),
        source_fingerprint: source.into(),
        eligible_user_ids: vec![user.into()],
        min_request_base_units: "1".into(),
        max_request_base_units: "25000000".into(),
        max_open_requests_per_user: "3".into(),
        max_reserved_base_units_per_user: "50000000".into(),
        max_reserved_base_units_global: "100000000".into(),
        hold_mode: "on_submit".into(),
        cancel_mode: "owner_cancel_until_settlement".into(),
        expiry_mode: "none".into(),
        participation_effect: "not_modified_by_this_feature".into(),
        disabled_account_recovery_text: "Contact the test operator to restore account access."
            .into(),
        terms_text: terms.into(),
        terms_digest: text_digest(terms),
    })
    .unwrap()
}

fn input(policy: &SellbackPolicy) -> SellbackSubmitInput {
    SellbackSubmitInput {
        idempotency_key: "synthetic-request-1".into(),
        amount_base_units: 10,
        expected_snapshot_digest: "b".repeat(64),
        policy_digest: policy.policy_digest.clone(),
        terms_digest: policy.body.terms_digest.clone(),
    }
}

#[test]
fn no_configuration_provides_no_commercial_defaults() {
    assert_eq!(
        configuration_from_values(None, None),
        SellbackConfiguration::Disabled
    );
    assert_eq!(
        configuration_from_values(Some("disabled"), Some("broken")),
        SellbackConfiguration::Disabled
    );
    for mode in ["", "live", "paper", "enabled"] {
        assert_eq!(
            configuration_from_values(Some(mode), None),
            SellbackConfiguration::Invalid
        );
    }
    assert_eq!(
        configuration_from_values(Some("approved_requests"), None),
        SellbackConfiguration::Invalid
    );
    assert_eq!(
        configuration_from_values(Some("approved_requests"), Some("{}")),
        SellbackConfiguration::Invalid
    );
}

#[test]
fn canonical_policy_integrity_and_private_availability() {
    let policy = fixture_policy("user-a", &"c".repeat(64));
    let json = serde_json::to_string(&policy.body).unwrap();
    assert_eq!(
        configuration_from_values(Some("approved_requests"), Some(&json)),
        SellbackConfiguration::Enabled(policy.clone())
    );
    let configuration = SellbackConfiguration::Enabled(policy.clone());
    assert!(availability(&configuration, "user-a", Some(&"c".repeat(64))).new_requests_enabled);
    for (user, source, reason) in [
        ("user-b", Some("c".repeat(64)), "user_not_eligible"),
        ("user-a", None, "source_mismatch"),
    ] {
        let unavailable = availability(&configuration, user, source.as_deref());
        assert!(!unavailable.new_requests_enabled);
        assert!(unavailable.policy.is_none());
        assert_eq!(unavailable.reason, reason);
    }
    let mut changed = policy.clone();
    changed.body.max_open_requests_per_user = "4".into();
    assert_eq!(
        validate_policy_integrity(&changed)
            .unwrap_err()
            .downcast_ref(),
        Some(&SellbackError::Corrupt)
    );
}

#[test]
fn policy_exact_schema_limits_and_raw_utf8_terms_digest() {
    let policy = fixture_policy("user-a", &"c".repeat(64));
    let mut value = serde_json::to_value(&policy.body).unwrap();
    value["unknown"] = true.into();
    assert!(serde_json::from_value::<SellbackPolicyBody>(value).is_err());
    let json = serde_json::to_string(&policy.body).unwrap();
    assert!(serde_json::from_str::<SellbackPolicyBody>(&json.replacen(
        "{",
        "{\"revision\":\"duplicate\",",
        1
    ))
    .is_err());
    for mutate in [
        |p: &mut SellbackPolicyBody| p.eligible_user_ids.push("user-a".into()),
        |p: &mut SellbackPolicyBody| p.eligible_user_ids = vec!["local-owner".into()],
        |p: &mut SellbackPolicyBody| p.min_request_base_units = "0".into(),
        |p: &mut SellbackPolicyBody| p.max_reserved_base_units_global = "1".into(),
        |p: &mut SellbackPolicyBody| p.hold_mode = "no_hold".into(),
        |p: &mut SellbackPolicyBody| p.expiry_mode = "automatic".into(),
        |p: &mut SellbackPolicyBody| p.terms_text.push(' '),
        |p: &mut SellbackPolicyBody| p.disabled_account_recovery_text = " \n ".into(),
    ] {
        let mut body = policy.body.clone();
        mutate(&mut body);
        assert!(validate_policy(body).is_err());
    }
    let mut body = policy.body.clone();
    body.terms_text = "界".repeat(682);
    body.terms_digest = text_digest(&body.terms_text);
    assert!(validate_policy(body.clone()).is_ok());
    body.terms_text.push('界');
    body.terms_digest = text_digest(&body.terms_text);
    assert!(validate_policy(body).is_err());
}

#[test]
fn units_are_positive_i64_canonical_strings() {
    assert_eq!(positive_units("9223372036854775807").unwrap(), i64::MAX);
    for bad in [
        "",
        "0",
        "01",
        "+1",
        "-1",
        "1.0",
        "1e2",
        " 1",
        "9223372036854775808",
    ] {
        assert!(positive_units(bad).is_err(), "accepted {bad}");
    }
    let policy = fixture_policy("user-a", &"c".repeat(64));
    let mut json = serde_json::json!({"schema":SUBMIT_SCHEMA,"idempotency_key":"key",
        "amount_base_units":"1","expected_snapshot_digest":"b".repeat(64),
        "policy_digest":policy.policy_digest,"terms_digest":policy.body.terms_digest,
        "confirmation":SUBMIT_CONFIRMATION});
    assert!(validate_submit_body(serde_json::from_value(json.clone()).unwrap()).is_ok());
    json["amount_base_units"] = 1.into();
    assert!(serde_json::from_value::<SellbackSubmitBody>(json).is_err());
}

#[test]
fn request_digest_binds_every_input_and_user() {
    let policy = fixture_policy("user-a", &"c".repeat(64));
    let original = input(&policy);
    let digest = request_digest("user-a", &policy, &original).unwrap();
    let mut changed = original.clone();
    changed.amount_base_units += 1;
    assert_ne!(request_digest("user-a", &policy, &changed).unwrap(), digest);
    changed = original.clone();
    changed.idempotency_key.push('2');
    assert_ne!(request_digest("user-a", &policy, &changed).unwrap(), digest);
    changed = original.clone();
    changed.expected_snapshot_digest = "d".repeat(64);
    assert_ne!(request_digest("user-a", &policy, &changed).unwrap(), digest);
    for change_policy in [true, false] {
        changed = original.clone();
        if change_policy {
            changed.policy_digest = "d".repeat(64);
        } else {
            changed.terms_digest = "d".repeat(64);
        }
        assert!(request_digest("user-a", &policy, &changed).is_err());
    }
    assert!(request_digest("user-b", &policy, &original).is_err());
}

#[test]
fn stored_request_and_cancel_binding_fail_closed() {
    let policy = fixture_policy("user-a", &"c".repeat(64));
    let input = input(&policy);
    let mut record = SellbackRecord {
        request_id: format!("eskpsr_{}", "1".repeat(32)),
        user_id: "user-a".into(),
        request_digest: request_digest("user-a", &policy, &input).unwrap(),
        input,
        policy,
        created_at: "2026-09-04T03:00:00.500Z".into(),
        canceled_at: None,
        cancel_event_id: None,
    };
    validate_stored_request(&record).unwrap();
    record.canceled_at = Some("2026-09-04T03:00:00.499999999+00:00".into());
    record.cancel_event_id = Some(format!("eskpsc_{}", "2".repeat(32)));
    assert!(validate_stored_request(&record).is_err());
    record.canceled_at = Some("2026-09-04T03:00:00.5+00:00".into());
    validate_stored_request(&record).unwrap();
    record.input.amount_base_units += 1;
    assert!(validate_stored_request(&record).is_err());
}

#[test]
fn utc_time_comparison_is_not_lexical() {
    for good in [
        "2024-02-29T23:59:59Z",
        "2026-09-04T00:00:00.123456789+00:00",
    ] {
        assert!(valid_timestamp(good));
    }
    for bad in [
        "2025-02-29T00:00:00Z",
        "2026-09-04T24:00:00Z",
        "2026-09-04T00:00:00+08:00",
        "2026-09-04T00:00:00.Z",
        "2026-09-04T00:00:00.1234567890Z",
    ] {
        assert!(!valid_timestamp(bad));
    }
    assert!(timestamp_not_before(
        "2026-09-04T00:00:00.10+00:00",
        "2026-09-04T00:00:00.1Z"
    ));
    assert!(!timestamp_not_before(
        "2026-09-04T00:00:00.09Z",
        "2026-09-04T00:00:00.1+00:00"
    ));
    assert!(timestamp_not_before(
        "2026-09-04T00:00:01Z",
        "2026-09-04T00:00:00.9+00:00"
    ));
}

#[test]
fn cursor_and_identifiers_reject_aliases_and_extra_parts() {
    let id = format!("eskpsr_{}", "1".repeat(32));
    let cursor = format!("esbr1.{}.{}", "a".repeat(64), id);
    assert_eq!(parse_cursor(&cursor).unwrap().after_request_id, id);
    for bad in [
        format!("{cursor}.more"),
        cursor.replace("esbr1", "paper"),
        cursor.replace('a', "A"),
    ] {
        assert!(parse_cursor(&bad).is_err());
    }
    assert!(!valid_request_id("allocation_test"));
}
