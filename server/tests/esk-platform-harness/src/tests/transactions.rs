use super::*;

#[test]
fn prepare_confirm_and_owner_read_use_actual_sql_without_changing_paper() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let prepared = prepare(&fixture, &policy);
    assert!(prepared.recorded_at.is_none());
    assert!(!prepared.replayed);
    assert_eq!(fixture.count("esk_platform_policy"), 1);
    assert_eq!(fixture.count("esk_platform_allocations"), 1);
    fixture.assert_empty_posting();
    let recorded = record(&fixture, &policy, &prepared);
    assert!(recorded.recorded_at.is_some());
    assert!(!recorded.replayed);
    let alice = fixture.store.esk_platform_account("alice", 20).unwrap();
    assert_eq!(alice.total_base_units, 10000000);
    assert_eq!(alice.entry_count, 1);
    assert_eq!(alice.entries[0].allocation_id, prepared.allocation_id);
    assert_eq!(alice.entries[0].amount_base_units, 10000000);
    assert_eq!(
        fixture
            .store
            .esk_platform_account("bob", 20)
            .unwrap()
            .total_base_units,
        0
    );
    assert_eq!(fixture.paper_total(), 123000000);
    assert_eq!(fixture.count("esk_asset_ledger_entries"), 1);
}

#[test]
fn exact_prepare_and_confirm_replay_keep_one_immutable_record() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let first = prepare(&fixture, &policy);
    let duplicate = prepare(&fixture, &policy);
    assert_eq!(first.allocation_id, duplicate.allocation_id);
    assert!(duplicate.replayed);
    assert_eq!(first.prepared_at, duplicate.prepared_at);
    let recorded = record(&fixture, &policy, &first);
    let replay = record(&fixture, &policy, &first);
    assert!(replay.replayed);
    assert_eq!(recorded.recorded_at, replay.recorded_at);
    let prepared_again = prepare(&fixture, &policy);
    assert!(prepared_again.replayed && prepared_again.recorded_at.is_some());
    assert_eq!(fixture.count("esk_platform_approvals"), 1);
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 1);
}

#[test]
fn same_payment_changed_user_amount_terms_or_evidence_conflicts() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    prepare(&fixture, &policy);
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
    let mut terms = body();
    terms.sale.terms_digest = "9".repeat(64);
    changes.push(terms);
    let mut history = body();
    history.history_evidence_digest = "a".repeat(64);
    changes.push(history);
    let mut review = body();
    review.review_reference = "another-review".into();
    changes.push(review);
    for changed in changes {
        let changed = prepare_input(&policy, changed).unwrap();
        assert_error(
            fixture.store.prepare_esk_platform_allocation(
                &policy,
                &changed,
                "admin-1",
                &token("admin-1"),
            ),
            PlatformError::Conflict,
        );
    }
    assert_eq!(fixture.count("esk_platform_allocations"), 1);
    fixture.assert_empty_posting();
}

#[test]
fn wrong_confirmation_digest_cannot_record() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let prepared = prepare(&fixture, &policy);
    assert_error(
        fixture.store.record_esk_platform_allocation(
            &policy,
            &prepared.allocation_id,
            &"0".repeat(64),
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::Conflict,
    );
    assert_error(
        fixture.store.record_esk_platform_allocation(
            &policy,
            "missing-allocation",
            &prepared.input.request_digest,
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::NotFound,
    );
    fixture.assert_empty_posting();
}

#[test]
fn first_policy_is_pinned_and_source_or_limit_changes_fail_closed() {
    let fixture = Fixture::new();
    let first_policy = policy(100000000);
    let prepared = prepare(&fixture, &first_policy);
    let changed_limit = policy(200000000);
    let mut new_source = source();
    new_source.namespace = "other.operator-ledger".into();
    let changed_source = validate_policy(PolicyBody {
        source: new_source,
        issuance_limit_base_units: "100000000".into(),
    })
    .unwrap();
    for changed in [changed_limit, changed_source] {
        assert_error(
            fixture.store.prepare_esk_platform_allocation(
                &changed,
                &input(&changed),
                "admin-1",
                &token("admin-1"),
            ),
            PlatformError::PolicyChanged,
        );
        assert_error(
            fixture.store.record_esk_platform_allocation(
                &changed,
                &prepared.allocation_id,
                &prepared.input.request_digest,
                "admin-1",
                &token("admin-1"),
            ),
            PlatformError::PolicyChanged,
        );
    }
    assert_eq!(fixture.count("esk_platform_policy"), 1);
    assert_eq!(fixture.count("esk_platform_allocations"), 1);
    fixture.assert_empty_posting();
}

#[test]
fn cumulative_policy_limit_is_enforced_at_record_not_by_pending_balance() {
    let fixture = Fixture::new();
    let policy = policy(15000000);
    let first = prepare(&fixture, &policy);
    let mut second = body();
    second.transfer_index = 1;
    second.user_id = "bob".into();
    let second_input = prepare_input(&policy, second).unwrap();
    let second = fixture
        .store
        .prepare_esk_platform_allocation(&policy, &second_input, "admin-1", &token("admin-1"))
        .unwrap();
    fixture.assert_empty_posting();
    record(&fixture, &policy, &first);
    assert_error(
        fixture.store.record_esk_platform_allocation(
            &policy,
            &second.allocation_id,
            &second.input.request_digest,
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::LimitExceeded,
    );
    assert_eq!(fixture.count("esk_platform_allocations"), 2);
    assert_eq!(fixture.count("esk_platform_approvals"), 1);
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 1);
    assert_eq!(
        fixture
            .store
            .esk_platform_account("bob", 20)
            .unwrap()
            .total_base_units,
        0
    );
}

#[test]
fn read_entry_limit_does_not_truncate_balance_totals() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    for index in 0..3 {
        let mut value = body();
        value.transfer_index = index;
        let value = prepare_input(&policy, value).unwrap();
        let prepared = fixture
            .store
            .prepare_esk_platform_allocation(&policy, &value, "admin-1", &token("admin-1"))
            .unwrap();
        record(&fixture, &policy, &prepared);
    }
    let account = fixture.store.esk_platform_account("alice", 1).unwrap();
    assert_eq!(account.total_base_units, 30000000);
    assert_eq!(account.entry_count, 3);
    assert_eq!(account.entries.len(), 1);
}
