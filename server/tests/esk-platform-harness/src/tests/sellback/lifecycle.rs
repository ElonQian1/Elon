use super::*;

#[test]
fn submit_cancel_replay_preserve_formal_and_paper_ledgers() {
    let (fixture, _, config) = setup();
    let original = history::page(&fixture, "alice", 20, None).unwrap();
    let request = input(&fixture, "alice", "first", 7_000_000, &config);
    let first = fixture
        .store
        .submit_esk_platform_sellback("alice", &token("alice"), &request, &config)
        .unwrap();
    assert!(!first.replayed);
    assert_eq!(
        (
            first.summary.total_base_units,
            first.summary.reserved_base_units,
            first.summary.available_base_units
        ),
        (10_000_000, 7_000_000, 3_000_000)
    );
    let replay = fixture
        .store
        .submit_esk_platform_sellback(
            "alice",
            &token("alice"),
            &request,
            &SellbackConfiguration::Invalid,
        )
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.request, first.request);
    let mut changed = request.clone();
    changed.amount_base_units += 1;
    error(
        fixture
            .store
            .submit_esk_platform_sellback("alice", &token("alice"), &changed, &config),
        SellbackError::Conflict,
    );
    let canceled = fixture
        .store
        .cancel_esk_platform_sellback(
            "alice",
            &token("alice"),
            &first.request.request_id,
            &SellbackConfiguration::Disabled,
        )
        .unwrap();
    assert!(!canceled.replayed && canceled.request.canceled_at.is_some());
    assert_eq!(
        (
            canceled.summary.reserved_base_units,
            canceled.summary.available_base_units
        ),
        (0, 10_000_000)
    );
    let again = fixture
        .store
        .cancel_esk_platform_sellback("alice", &token("alice"), &first.request.request_id, &config)
        .unwrap();
    assert!(again.replayed);
    assert_eq!(again.request, canceled.request);
    let replay = fixture
        .store
        .submit_esk_platform_sellback("alice", &token("alice"), &request, &config)
        .unwrap();
    assert!(replay.replayed && replay.request.canceled_at.is_some());
    assert_eq!(replay.summary.reserved_base_units, 0);
    assert_eq!(
        history::page(&fixture, "alice", 20, None)
            .unwrap()
            .snapshot_digest,
        original.snapshot_digest
    );
    assert_eq!(fixture.count("esk_platform_sellback_requests"), 1);
    assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 1);
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 2);
    assert_eq!(fixture.paper_total(), 123_000_000);
}

#[test]
fn lookup_is_private_readonly_and_returns_original_key_after_policy_change() {
    let (fixture, _, config) = setup();
    let first = submit(&fixture, "alice", "unknown-result", 1_000_000, &config);
    let found = fixture
        .store
        .lookup_esk_platform_sellback(
            "alice",
            &token("alice"),
            "unknown-result",
            &SellbackConfiguration::Invalid,
        )
        .unwrap();
    assert!(found.replayed);
    assert_eq!(found.request, first.request);
    error(
        fixture
            .store
            .lookup_esk_platform_sellback("bob", &token("bob"), "unknown-result", &config),
        SellbackError::NotFound,
    );
    error(
        fixture.store.esk_platform_sellback_request(
            "bob",
            &token("bob"),
            &first.request.request_id,
            &config,
        ),
        SellbackError::NotFound,
    );
    error(
        fixture.store.cancel_esk_platform_sellback(
            "bob",
            &token("bob"),
            &first.request.request_id,
            &config,
        ),
        SellbackError::NotFound,
    );
    assert_eq!(fixture.count("esk_platform_sellback_requests"), 1);
    assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 0);
}

#[test]
fn disabled_new_requests_and_tighter_policy_do_not_trap_existing_holds() {
    let (fixture, _, config) = setup();
    let new = input(&fixture, "alice", "disabled", 1, &config);
    for disabled in [
        SellbackConfiguration::Disabled,
        SellbackConfiguration::Invalid,
    ] {
        error(
            fixture
                .store
                .submit_esk_platform_sellback("alice", &token("alice"), &new, &disabled),
            SellbackError::Disabled,
        );
    }
    let first = submit(&fixture, "alice", "held", 7_000_000, &config);
    let SellbackConfiguration::Enabled(mut policy) = config else {
        unreachable!()
    };
    policy.body.max_request_base_units = "1".into();
    policy.body.max_reserved_base_units_per_user = "1".into();
    policy.body.max_reserved_base_units_global = "1".into();
    let tight = SellbackConfiguration::Enabled(validate_policy(policy.body).unwrap());
    assert_eq!(
        page(&fixture, "alice", &tight).summary.reserved_base_units,
        7_000_000
    );
    let next = input(&fixture, "alice", "new", 1, &tight);
    error(
        fixture
            .store
            .submit_esk_platform_sellback("alice", &token("alice"), &next, &tight),
        SellbackError::LimitExceeded,
    );
    let released = fixture
        .store
        .cancel_esk_platform_sellback("alice", &token("alice"), &first.request.request_id, &tight)
        .unwrap();
    assert_eq!(released.summary.reserved_base_units, 0);
}

#[test]
fn policy_revision_and_configuration_state_are_bound_to_pagination_snapshot() {
    let (fixture, _, config) = setup();
    submit(&fixture, "alice", "first", 1, &config);
    submit(&fixture, "alice", "second", 1, &config);
    let first = fixture
        .store
        .esk_platform_sellback_page("alice", &token("alice"), 1, None, &config)
        .unwrap();
    let SellbackConfiguration::Enabled(mut policy) = config else {
        unreachable!()
    };
    policy.body.revision = "synthetic-v2".into();
    let changed = SellbackConfiguration::Enabled(validate_policy(policy.body).unwrap());
    for current in [
        changed,
        SellbackConfiguration::Disabled,
        SellbackConfiguration::Invalid,
    ] {
        let current_page = page(&fixture, "alice", &current);
        assert_ne!(
            current_page.summary.snapshot_digest,
            first.summary.snapshot_digest
        );
        assert_eq!(
            current_page.summary.reserved_base_units,
            first.summary.reserved_base_units
        );
        error(
            fixture.store.esk_platform_sellback_page(
                "alice",
                &token("alice"),
                1,
                first.next_cursor.as_deref(),
                &current,
            ),
            SellbackError::SnapshotChanged,
        );
    }
}

#[test]
fn cancellation_rejects_clock_regression_or_impossible_timestamp_without_audit_write() {
    for timestamp in ["2026-09-04T10:00:00.001Z", "2026-02-30T10:00:00Z"] {
        let (fixture, _, config) = setup();
        let request = submit(&fixture, "alice", "clock", 1, &config);
        // Synthetic corruption only: production tables reject updates.
        let conn = fixture.store.conn().unwrap();
        conn.execute_batch("DROP TRIGGER trg_esk_platform_sellback_requests_no_update")
            .unwrap();
        conn.execute(
            "UPDATE esk_platform_sellback_requests SET created_at = ?1 WHERE request_id = ?2",
            rusqlite::params![timestamp, request.request.request_id],
        )
        .unwrap();
        error(
            fixture.store.cancel_esk_platform_sellback(
                "alice",
                &token("alice"),
                &request.request.request_id,
                &config,
            ),
            SellbackError::Corrupt,
        );
        assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 0);
        assert_eq!(fixture.count("esk_platform_sellback_requests"), 1);
    }
}
