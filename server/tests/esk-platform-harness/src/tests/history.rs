use super::*;

pub(super) fn post(
    fixture: &Fixture,
    policy: &PlatformPolicy,
    user: &str,
    index: u32,
) -> PlatformAllocationRecord {
    let mut value = body();
    value.user_id = user.to_owned();
    value.transfer_index = index;
    let input = prepare_input(policy, value).unwrap();
    let prepared = fixture
        .store
        .prepare_esk_platform_allocation(policy, &input, "admin-1", &token("admin-1"))
        .unwrap();
    record(fixture, policy, &prepared)
}

pub(super) fn page(
    fixture: &Fixture,
    user: &str,
    limit: usize,
    cursor: Option<&str>,
) -> anyhow::Result<PlatformHistoryPage> {
    fixture
        .store
        .esk_platform_history(user, &token(user), limit, cursor)
}

#[test]
fn empty_snapshot_is_user_bound_and_does_not_pin_policy_or_touch_paper() {
    let fixture = Fixture::new();
    let alice = page(&fixture, "alice", 20, None).unwrap();
    let bob = page(&fixture, "bob", 20, None).unwrap();
    assert_eq!(alice.total_base_units, 0);
    assert_eq!(alice.entry_count, 0);
    assert_eq!((alice.range_start, alice.range_end), (0, 0));
    assert!(alice.entries.is_empty() && alice.updated_at.is_none());
    assert!(!alice.has_more && alice.next_cursor.is_none());
    assert_eq!(alice.snapshot_digest.len(), 64);
    assert_ne!(alice.snapshot_digest, bob.snapshot_digest);
    assert_eq!(fixture.count("esk_platform_policy"), 0);
    fixture.assert_empty_posting();
    let policy = policy(100000000);
    prepare(&fixture, &policy);
    let pinned = page(&fixture, "alice", 20, None).unwrap();
    assert_ne!(alice.snapshot_digest, pinned.snapshot_digest);
    assert_eq!(pinned.entry_count, 0);
}

#[test]
fn every_page_retains_full_totals_ranges_and_strict_same_timestamp_id_order() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    for index in 0..7 {
        post(&fixture, &policy, "alice", index);
    }
    let first = page(&fixture, "alice", 3, None).unwrap();
    let second = page(&fixture, "alice", 3, first.next_cursor.as_deref()).unwrap();
    let third = page(&fixture, "alice", 3, second.next_cursor.as_deref()).unwrap();
    assert_eq!((first.range_start, first.range_end), (1, 3));
    assert_eq!((second.range_start, second.range_end), (4, 6));
    assert_eq!((third.range_start, third.range_end), (7, 7));
    assert!(first.has_more && second.has_more);
    assert!(!third.has_more && third.next_cursor.is_none());
    for current in [&first, &second, &third] {
        assert_eq!(current.total_base_units, 70000000);
        assert_eq!(current.entry_count, 7);
        assert_eq!(current.updated_at.as_deref(), Some("2026-09-04T10:00:00Z"));
        assert_eq!(current.snapshot_digest, first.snapshot_digest);
    }
    let ids: Vec<_> = [&first, &second, &third]
        .into_iter()
        .flat_map(|current| current.entries.iter().map(|entry| entry.entry_id.clone()))
        .collect();
    assert_eq!(ids.len(), 7);
    assert!(ids.windows(2).all(|pair| pair[0] > pair[1]));
    let account = fixture.store.esk_platform_account("alice", 100).unwrap();
    assert_eq!(
        ids,
        account
            .entries
            .into_iter()
            .map(|entry| entry.entry_id)
            .collect::<Vec<_>>()
    );
    let replay = page(&fixture, "alice", 3, first.next_cursor.as_deref()).unwrap();
    assert_eq!(replay.entries[0].entry_id, second.entries[0].entry_id);
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 7);
    assert_eq!(fixture.paper_total(), 123000000);
}

#[test]
fn changing_page_limit_does_not_change_snapshot_and_can_continue_with_new_limit() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    for index in 0..4 {
        post(&fixture, &policy, "alice", index);
    }
    let first = page(&fixture, "alice", 1, None).unwrap();
    let whole = page(&fixture, "alice", 100, None).unwrap();
    assert_eq!(first.snapshot_digest, whole.snapshot_digest);
    let rest = page(&fixture, "alice", 100, first.next_cursor.as_deref()).unwrap();
    assert_eq!((rest.range_start, rest.range_end), (2, 4));
    assert_eq!(rest.entries.len(), 3);
    assert_eq!(rest.snapshot_digest, first.snapshot_digest);
    assert!(!rest.has_more && rest.next_cursor.is_none());
}

#[test]
fn page_cap_is_one_hundred_with_complete_totals_above_cap() {
    let fixture = Fixture::new();
    let policy = policy(2000000000);
    for index in 0..101 {
        post(&fixture, &policy, "alice", index);
    }
    let first = page(&fixture, "alice", 100, None).unwrap();
    assert_eq!(first.entries.len(), 100);
    assert_eq!(first.entry_count, 101);
    assert_eq!(first.total_base_units, 1010000000);
    assert_eq!((first.range_start, first.range_end), (1, 100));
    let last = page(&fixture, "alice", 100, first.next_cursor.as_deref()).unwrap();
    assert_eq!((last.range_start, last.range_end), (101, 101));
    assert!(!last.has_more && last.next_cursor.is_none());
}

#[test]
fn new_own_entry_invalidates_cursor_but_another_users_entry_does_not() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    post(&fixture, &policy, "alice", 0);
    post(&fixture, &policy, "alice", 1);
    let first = page(&fixture, "alice", 1, None).unwrap();
    post(&fixture, &policy, "bob", 2);
    let stable = page(&fixture, "alice", 1, first.next_cursor.as_deref()).unwrap();
    assert_eq!(stable.snapshot_digest, first.snapshot_digest);
    post(&fixture, &policy, "alice", 3);
    assert_error(
        page(&fixture, "alice", 1, first.next_cursor.as_deref()),
        PlatformError::HistoryChanged,
    );
    assert_ne!(
        page(&fixture, "alice", 1, None).unwrap().snapshot_digest,
        first.snapshot_digest
    );
}

#[test]
fn prepared_canceled_and_exact_record_replay_do_not_change_recorded_snapshot() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let recorded = post(&fixture, &policy, "alice", 0);
    let first = page(&fixture, "alice", 1, None).unwrap();
    let mut value = body();
    value.transfer_index = 1;
    let pending = fixture
        .store
        .prepare_esk_platform_allocation(
            &policy,
            &prepare_input(&policy, value).unwrap(),
            "admin-1",
            &token("admin-1"),
        )
        .unwrap();
    assert_eq!(
        page(&fixture, "alice", 1, None).unwrap().snapshot_digest,
        first.snapshot_digest
    );
    fixture
        .store
        .cancel_esk_platform_allocation(
            &policy,
            &pending.allocation_id,
            &pending.input.request_digest,
            "admin-1",
            &token("admin-1"),
        )
        .unwrap();
    record(&fixture, &policy, &recorded);
    assert_eq!(
        page(&fixture, "alice", 1, None).unwrap().snapshot_digest,
        first.snapshot_digest
    );
}

#[test]
fn reopening_store_preserves_snapshot_and_cursor() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    for index in 0..3 {
        post(&fixture, &policy, "alice", index);
    }
    let first = page(&fixture, "alice", 1, None).unwrap();
    let reopened = Store {
        path: fixture.store.path.clone(),
    };
    let second = reopened
        .esk_platform_history("alice", &token("alice"), 1, first.next_cursor.as_deref())
        .unwrap();
    assert_eq!(second.snapshot_digest, first.snapshot_digest);
    assert_eq!((second.range_start, second.range_end), (2, 2));
}
