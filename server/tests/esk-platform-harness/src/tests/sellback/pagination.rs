use super::*;

#[test]
fn full_totals_survive_twenty_record_page_boundaries_and_restart() {
    let (fixture, _, config) = setup();
    let SellbackConfiguration::Enabled(mut policy) = config else {
        unreachable!()
    };
    policy.body.max_open_requests_per_user = "100".into();
    let config = SellbackConfiguration::Enabled(validate_policy(policy.body).unwrap());
    for index in 0..21 {
        submit(&fixture, "alice", &format!("page-{index}"), 1, &config);
    }
    let first = page(&fixture, "alice", &config);
    assert_eq!((first.range_start, first.range_end), (1, 20));
    assert_eq!(first.requests.len(), 20);
    assert_eq!(
        (
            first.summary.request_count,
            first.summary.open_request_count,
            first.summary.reserved_base_units
        ),
        (21, 21, 21)
    );
    assert!(first.has_more);
    let reopened = fixture.store.clone();
    let last = reopened
        .esk_platform_sellback_page(
            "alice",
            &token("alice"),
            1,
            first.next_cursor.as_deref(),
            &config,
        )
        .unwrap();
    assert_eq!((last.range_start, last.range_end), (21, 21));
    assert_eq!(last.summary, first.summary);
    assert!(!last.has_more && last.next_cursor.is_none());
    let ids: Vec<_> = first
        .requests
        .iter()
        .chain(last.requests.iter())
        .map(|r| &r.request_id)
        .collect();
    assert!(ids.windows(2).all(|pair| pair[0] > pair[1]));
    for limit in [0, 21, usize::MAX] {
        error(
            fixture.store.esk_platform_sellback_page(
                "alice",
                &token("alice"),
                limit,
                None,
                &config,
            ),
            SellbackError::InvalidInput,
        );
    }
}

#[test]
fn cursors_bind_user_complete_journal_and_nonterminal_existing_anchor() {
    let (fixture, formal, config) = setup();
    for key in ["one", "two", "three"] {
        submit(&fixture, "alice", key, 1, &config);
    }
    let first = fixture
        .store
        .esk_platform_sellback_page("alice", &token("alice"), 1, None, &config)
        .unwrap();
    submit(&fixture, "bob", "bob-only", 1, &config);
    let stable = fixture
        .store
        .esk_platform_sellback_page(
            "alice",
            &token("alice"),
            2,
            first.next_cursor.as_deref(),
            &config,
        )
        .unwrap();
    assert_eq!(stable.summary, first.summary);
    error(
        fixture.store.esk_platform_sellback_page(
            "bob",
            &token("bob"),
            1,
            first.next_cursor.as_deref(),
            &config,
        ),
        SellbackError::SnapshotChanged,
    );
    let last_id = &stable.requests.last().unwrap().request_id;
    for id in [last_id.clone(), format!("eskpsr_{}", "0".repeat(32))] {
        let forged = format!("esbr1.{}.{}", first.summary.snapshot_digest, id);
        error(
            fixture.store.esk_platform_sellback_page(
                "alice",
                &token("alice"),
                1,
                Some(&forged),
                &config,
            ),
            SellbackError::SnapshotChanged,
        );
    }
    fixture
        .store
        .cancel_esk_platform_sellback("alice", &token("alice"), last_id, &config)
        .unwrap();
    error(
        fixture.store.esk_platform_sellback_page(
            "alice",
            &token("alice"),
            1,
            first.next_cursor.as_deref(),
            &config,
        ),
        SellbackError::SnapshotChanged,
    );
    let before_entry = fixture
        .store
        .esk_platform_sellback_page("alice", &token("alice"), 1, None, &config)
        .unwrap();
    history::post(&fixture, &formal, "alice", 9);
    error(
        fixture.store.esk_platform_sellback_page(
            "alice",
            &token("alice"),
            1,
            before_entry.next_cursor.as_deref(),
            &config,
        ),
        SellbackError::SnapshotChanged,
    );
    for cursor in ["", "esbr1.invalid", "ephp1.old", "esbr1.a.b.extra"] {
        error(
            fixture.store.esk_platform_sellback_page(
                "alice",
                &token("alice"),
                1,
                Some(cursor),
                &config,
            ),
            SellbackError::InvalidInput,
        );
    }
}

#[test]
fn paper_and_unrecorded_allocations_are_never_available_and_empty_reads_do_not_pin_policy() {
    let fixture = Fixture::new();
    let disabled = SellbackConfiguration::Disabled;
    let empty = page(&fixture, "alice", &disabled);
    assert_eq!(
        (
            empty.summary.total_base_units,
            empty.summary.reserved_base_units,
            empty.summary.available_base_units
        ),
        (0, 0, 0)
    );
    assert_eq!((empty.range_start, empty.range_end), (0, 0));
    assert!(empty.requests.is_empty() && !empty.has_more && empty.next_cursor.is_none());
    assert_eq!(fixture.count("esk_platform_policy"), 0);
    let formal = policy(100_000_000);
    super::super::prepare(&fixture, &formal);
    let approved = crate::esk_asset::platform::sellback::tests::fixture_policy(
        "alice",
        &formal.source_fingerprint,
    );
    let config = SellbackConfiguration::Enabled(approved);
    let attempt = input(&fixture, "alice", "pending-is-not-money", 1, &config);
    error(
        fixture
            .store
            .submit_esk_platform_sellback("alice", &token("alice"), &attempt, &config),
        SellbackError::InsufficientAvailable,
    );
    assert_eq!(fixture.count("esk_platform_sellback_requests"), 0);
    assert_eq!(fixture.paper_total(), 123_000_000);
}
