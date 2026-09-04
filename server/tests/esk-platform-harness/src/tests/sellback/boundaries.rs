use super::*;

#[test]
fn replay_requires_every_original_semantic_field_even_when_current_policy_is_disabled() {
    let (fixture, _, config) = setup();
    let original = input(&fixture, "alice", "exact-payload", 1, &config);
    fixture
        .store
        .submit_esk_platform_sellback("alice", &token("alice"), &original, &config)
        .unwrap();
    for change in [
        |i: &mut SellbackSubmitInput| i.amount_base_units += 1,
        |i: &mut SellbackSubmitInput| i.expected_snapshot_digest = "a".repeat(64),
        |i: &mut SellbackSubmitInput| i.policy_digest = "b".repeat(64),
        |i: &mut SellbackSubmitInput| i.terms_digest = "c".repeat(64),
    ] {
        let mut changed = original.clone();
        change(&mut changed);
        error(
            fixture.store.submit_esk_platform_sellback(
                "alice",
                &token("alice"),
                &changed,
                &SellbackConfiguration::Disabled,
            ),
            SellbackError::Conflict,
        );
    }
    // Keys are scoped to their actual authenticated owner, not globally reserved.
    let bob = submit(&fixture, "bob", "exact-payload", 1, &config);
    assert_eq!(bob.request.user_id, "bob");
    assert_eq!(fixture.count("esk_platform_sellback_requests"), 2);
}

#[test]
fn explicit_min_max_personal_hold_and_open_count_limits_fail_without_new_rows() {
    let (fixture, _, config) = setup();
    let SellbackConfiguration::Enabled(mut policy) = config else {
        unreachable!()
    };
    policy.body.min_request_base_units = "2".into();
    policy.body.max_request_base_units = "6".into();
    policy.body.max_reserved_base_units_per_user = "8".into();
    policy.body.max_reserved_base_units_global = "20".into();
    policy.body.max_open_requests_per_user = "2".into();
    let config = SellbackConfiguration::Enabled(validate_policy(policy.body).unwrap());
    for amount in [1, 7] {
        let invalid = input(
            &fixture,
            "alice",
            &format!("invalid-{amount}"),
            amount,
            &config,
        );
        error(
            fixture
                .store
                .submit_esk_platform_sellback("alice", &token("alice"), &invalid, &config),
            SellbackError::LimitExceeded,
        );
    }
    let first = submit(&fixture, "alice", "first", 6, &config);
    let too_many_units = input(&fixture, "alice", "too-many-units", 3, &config);
    error(
        fixture.store.submit_esk_platform_sellback(
            "alice",
            &token("alice"),
            &too_many_units,
            &config,
        ),
        SellbackError::LimitExceeded,
    );
    submit(&fixture, "alice", "second", 2, &config);
    let too_many_requests = input(&fixture, "alice", "third", 2, &config);
    error(
        fixture.store.submit_esk_platform_sellback(
            "alice",
            &token("alice"),
            &too_many_requests,
            &config,
        ),
        SellbackError::LimitExceeded,
    );
    fixture
        .store
        .cancel_esk_platform_sellback("alice", &token("alice"), &first.request.request_id, &config)
        .unwrap();
    submit(&fixture, "alice", "third-after-release", 2, &config);
    let current = page(&fixture, "alice", &config);
    assert_eq!(
        (
            current.summary.reserved_base_units,
            current.summary.open_request_count,
            current.summary.request_count
        ),
        (4, 2, 3)
    );
}

#[test]
fn malformed_input_policy_or_terms_drift_and_wrong_eligibility_never_create_requests() {
    let (fixture, _, config) = setup();
    let original = input(&fixture, "alice", "invalid", 1, &config);
    for change in [
        |i: &mut SellbackSubmitInput| i.amount_base_units = 0,
        |i: &mut SellbackSubmitInput| i.amount_base_units = i64::MIN,
        |i: &mut SellbackSubmitInput| i.idempotency_key.clear(),
        |i: &mut SellbackSubmitInput| i.idempotency_key = "a".repeat(97),
        |i: &mut SellbackSubmitInput| i.expected_snapshot_digest = "A".repeat(64),
        |i: &mut SellbackSubmitInput| i.terms_digest = "short".into(),
    ] {
        let mut changed = original.clone();
        change(&mut changed);
        error(
            fixture
                .store
                .submit_esk_platform_sellback("alice", &token("alice"), &changed, &config),
            SellbackError::InvalidInput,
        );
    }
    for change in [
        |i: &mut SellbackSubmitInput| i.policy_digest = "a".repeat(64),
        |i: &mut SellbackSubmitInput| i.terms_digest = "b".repeat(64),
    ] {
        let mut changed = original.clone();
        change(&mut changed);
        error(
            fixture
                .store
                .submit_esk_platform_sellback("alice", &token("alice"), &changed, &config),
            SellbackError::PolicyChanged,
        );
    }
    let SellbackConfiguration::Enabled(policy) = config else {
        unreachable!()
    };
    let mut denied = policy.body.clone();
    denied.eligible_user_ids = vec!["bob".into()];
    let denied = SellbackConfiguration::Enabled(validate_policy(denied).unwrap());
    error(
        fixture
            .store
            .submit_esk_platform_sellback("alice", &token("alice"), &original, &denied),
        SellbackError::Ineligible,
    );
    let mut wrong_source = policy.body;
    wrong_source.source_fingerprint = "f".repeat(64);
    let wrong_source = SellbackConfiguration::Enabled(validate_policy(wrong_source).unwrap());
    error(
        fixture.store.submit_esk_platform_sellback(
            "alice",
            &token("alice"),
            &original,
            &wrong_source,
        ),
        SellbackError::PolicyChanged,
    );
    assert_eq!(fixture.count("esk_platform_sellback_requests"), 0);
}

#[test]
fn i64_maximum_formal_balance_can_be_fully_reserved_then_released_without_overflow_or_mint() {
    let fixture = Fixture::new();
    let formal = policy(i64::MAX);
    let mut body = super::super::body();
    body.amount = "9223372036854.775807".into();
    body.payment_amount = body.amount.clone();
    body.sale.payment_base_units_per_lot = "1".into();
    body.sale.esk_base_units_per_lot = "1".into();
    let allocation = crate::esk_asset::platform::prepare_input(&formal, body).unwrap();
    let prepared = fixture
        .store
        .prepare_esk_platform_allocation(&formal, &allocation, "admin-1", &token("admin-1"))
        .unwrap();
    super::super::record(&fixture, &formal, &prepared);
    let mut body = crate::esk_asset::platform::sellback::tests::fixture_policy(
        "alice",
        &formal.source_fingerprint,
    )
    .body;
    body.max_request_base_units = i64::MAX.to_string();
    body.max_reserved_base_units_per_user = i64::MAX.to_string();
    body.max_reserved_base_units_global = i64::MAX.to_string();
    let config = SellbackConfiguration::Enabled(validate_policy(body).unwrap());
    let original_digest = history::page(&fixture, "alice", 20, None)
        .unwrap()
        .snapshot_digest;
    let first = submit(&fixture, "alice", "max", i64::MAX, &config);
    assert_eq!(
        (
            first.summary.total_base_units,
            first.summary.reserved_base_units,
            first.summary.available_base_units
        ),
        (i64::MAX, i64::MAX, 0)
    );
    let overflow = input(&fixture, "alice", "overflow", 1, &config);
    error(
        fixture
            .store
            .submit_esk_platform_sellback("alice", &token("alice"), &overflow, &config),
        SellbackError::LimitExceeded,
    );
    let released = fixture
        .store
        .cancel_esk_platform_sellback("alice", &token("alice"), &first.request.request_id, &config)
        .unwrap();
    assert_eq!(
        (
            released.summary.total_base_units,
            released.summary.reserved_base_units,
            released.summary.available_base_units
        ),
        (i64::MAX, 0, i64::MAX)
    );
    assert_eq!(
        history::page(&fixture, "alice", 20, None)
            .unwrap()
            .snapshot_digest,
        original_digest
    );
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 1);
    assert_eq!(fixture.paper_total(), 123_000_000);
}
