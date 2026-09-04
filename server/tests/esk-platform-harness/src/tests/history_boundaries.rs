use super::{
    history::{page, post},
    *,
};

#[test]
fn cursor_format_is_exact_ascii_bounded_and_never_coerced() {
    let valid = format!("ephp1.{}.eskp_entry_{}", "a".repeat(64), "b".repeat(32));
    let parsed = PlatformHistoryCursor::parse(&valid).unwrap();
    assert_eq!(parsed.snapshot_digest, "a".repeat(64));
    assert_eq!(
        parsed.after_entry_id,
        format!("eskp_entry_{}", "b".repeat(32))
    );
    for invalid in [
        String::new(),
        format!(" {valid}"),
        format!("{valid} "),
        valid.to_uppercase(),
        valid.replace("ephp1", "ephp2"),
        valid.replace("eskp_entry_", "eskp_entry-"),
        valid.replace('a', "g"),
        format!("{valid}.extra"),
        valid[..113].to_owned(),
        "é".repeat(57),
        "a".repeat(10000),
    ] {
        assert_error(
            PlatformHistoryCursor::parse(&invalid),
            PlatformError::InvalidInput,
        );
    }
}

#[test]
fn invalid_limit_and_cursor_are_rejected_in_store_before_database_access() {
    let store = Store {
        path: std::env::temp_dir()
            .join(format!("does-not-exist-{}", uuid::Uuid::new_v4()))
            .join("missing.sqlite"),
    };
    for limit in [0, 101, usize::MAX] {
        assert_error(
            store.esk_platform_history("alice", &token("alice"), limit, None),
            PlatformError::InvalidInput,
        );
    }
    assert_error(
        store.esk_platform_history("alice", &token("alice"), 20, Some("bad")),
        PlatformError::InvalidInput,
    );
}

#[test]
fn cross_user_digest_change_unknown_anchor_and_terminal_anchor_share_one_error() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    for index in 0..2 {
        post(&fixture, &policy, "alice", index);
    }
    post(&fixture, &policy, "bob", 2);
    let first = page(&fixture, "alice", 1, None).unwrap();
    let cursor = first.next_cursor.as_deref().unwrap();
    assert_error(
        page(&fixture, "bob", 1, Some(cursor)),
        PlatformError::HistoryChanged,
    );
    let unknown = format!(
        "ephp1.{}.eskp_entry_{}",
        first.snapshot_digest,
        "0".repeat(32)
    );
    assert_error(
        page(&fixture, "alice", 1, Some(&unknown)),
        PlatformError::HistoryChanged,
    );
    let changed = format!("ephp1.{}.{}", "0".repeat(64), first.entries[0].entry_id);
    assert_error(
        page(&fixture, "alice", 1, Some(&changed)),
        PlatformError::HistoryChanged,
    );
    let last = page(&fixture, "alice", 1, Some(cursor)).unwrap();
    let terminal = format!(
        "ephp1.{}.{}",
        first.snapshot_digest, last.entries[0].entry_id
    );
    assert_error(
        page(&fixture, "alice", 1, Some(&terminal)),
        PlatformError::HistoryChanged,
    );
}

#[test]
fn actual_user_and_live_session_are_rechecked_for_each_page() {
    let fixture = Fixture::new();
    for (user, supplied) in [
        ("alice", token("bob")),
        ("missing", token("missing")),
        ("inactive-user", token("inactive-user")),
        ("local-owner", token("local-owner")),
        ("alice", String::new()),
    ] {
        assert_error(
            fixture
                .store
                .esk_platform_history(user, &supplied, 20, None),
            PlatformError::Unauthorized,
        );
    }
    let policy = policy(100000000);
    for index in 0..2 {
        post(&fixture, &policy, "alice", index);
    }
    let first = page(&fixture, "alice", 1, None).unwrap();
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE sessions SET revoked_at='fixture' WHERE id='alice'",
            [],
        )
        .unwrap();
    assert_error(
        page(&fixture, "alice", 1, first.next_cursor.as_deref()),
        PlatformError::Unauthorized,
    );
}

#[test]
fn malformed_exact_boundary_and_offset_expired_sessions_fail_closed() {
    let fixture = Fixture::new();
    for expiry in [
        "not-a-date",
        "2026-09-04T10:00:00Z",
        "2026-09-04T17:59:59+08:00",
    ] {
        fixture
            .store
            .conn()
            .unwrap()
            .execute(
                "UPDATE sessions SET expires_at=?1 WHERE id='alice'",
                [expiry],
            )
            .unwrap();
        assert_error(
            page(&fixture, "alice", 20, None),
            PlatformError::Unauthorized,
        );
    }
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE sessions SET expires_at='2026-09-04T03:00:00-08:00' WHERE id='alice'",
            [],
        )
        .unwrap();
    assert_eq!(page(&fixture, "alice", 20, None).unwrap().entry_count, 0);
}

#[test]
fn missing_approval_or_corrupt_binding_cannot_be_hidden_outside_requested_page() {
    for corruption in [
        "DROP TRIGGER trg_esk_platform_approvals_no_delete; DELETE FROM esk_platform_approvals WHERE allocation_id=(SELECT allocation_id FROM esk_platform_ledger_entries ORDER BY entry_id LIMIT 1)",
        "DROP TRIGGER trg_esk_platform_allocations_no_update; UPDATE esk_platform_allocations SET input_json='{}' WHERE allocation_id=(SELECT allocation_id FROM esk_platform_ledger_entries ORDER BY entry_id LIMIT 1)",
    ] {
        let fixture = Fixture::new();
        let policy = policy(100000000);
        for index in 0..3 { post(&fixture, &policy, "alice", index); }
        let conn = fixture.store.conn().unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute_batch(corruption).unwrap();
        assert_error(page(&fixture, "alice", 1, None), PlatformError::CorruptLedger);
    }
}

#[test]
fn pinned_policy_is_validated_even_for_an_empty_account() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    prepare(&fixture, &policy);
    assert_eq!(page(&fixture, "bob", 20, None).unwrap().entry_count, 0);
    let conn = fixture.store.conn().unwrap();
    conn.execute_batch("DROP TRIGGER trg_esk_platform_policy_no_update")
        .unwrap();
    conn.execute(
        "UPDATE esk_platform_policy SET source_fingerprint=?1",
        ["0".repeat(64)],
    )
    .unwrap();
    assert_error(
        page(&fixture, "bob", 20, None),
        PlatformError::CorruptLedger,
    );
}

#[test]
fn backdated_new_record_changes_digest_even_when_it_sorts_after_anchor() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    for index in 0..2 {
        post(&fixture, &policy, "alice", index);
    }
    let first = page(&fixture, "alice", 1, None).unwrap();
    let added = post(&fixture, &policy, "alice", 2);
    let conn = fixture.store.conn().unwrap();
    conn.execute_batch("DROP TRIGGER trg_esk_platform_approvals_no_update; DROP TRIGGER trg_esk_platform_ledger_entries_no_update;").unwrap();
    conn.execute("UPDATE esk_platform_approvals SET created_at='2025-01-01T00:00:00Z' WHERE allocation_id=?1", [&added.allocation_id]).unwrap();
    conn.execute("UPDATE esk_platform_ledger_entries SET created_at='2025-01-01T00:00:00Z' WHERE allocation_id=?1", [&added.allocation_id]).unwrap();
    assert_error(
        page(&fixture, "alice", 1, first.next_cursor.as_deref()),
        PlatformError::HistoryChanged,
    );
    let latest = page(&fixture, "alice", 100, None).unwrap();
    assert_eq!(
        latest.entries.last().unwrap().allocation_id,
        added.allocation_id
    );
}

#[test]
fn history_can_read_i64_maximum_and_is_table_read_only() {
    let fixture = Fixture::new();
    let policy = policy(i64::MAX);
    let mut value = body();
    value.amount = "9223372036854.775807".into();
    value.payment_amount = value.amount.clone();
    value.sale.payment_base_units_per_lot = "1".into();
    value.sale.esk_base_units_per_lot = "1".into();
    let prepared = fixture
        .store
        .prepare_esk_platform_allocation(
            &policy,
            &prepare_input(&policy, value).unwrap(),
            "admin-1",
            &token("admin-1"),
        )
        .unwrap();
    record(&fixture, &policy, &prepared);
    let tables = [
        "esk_platform_policy",
        "esk_platform_allocations",
        "esk_platform_approvals",
        "esk_platform_ledger_entries",
        "esk_platform_cancellations",
        "esk_asset_ledger_entries",
        "users",
        "sessions",
    ];
    let before = tables.map(|name| fixture.count(name));
    let first = page(&fixture, "alice", 1, None).unwrap();
    assert_eq!(first.total_base_units, i64::MAX);
    assert_eq!((first.range_start, first.range_end), (1, 1));
    assert!(!first.has_more);
    assert_eq!(
        first.snapshot_digest,
        page(&fixture, "alice", 100, None).unwrap().snapshot_digest
    );
    assert_eq!(before, tables.map(|name| fixture.count(name)));
    assert_eq!(fixture.paper_total(), 123000000);
}
