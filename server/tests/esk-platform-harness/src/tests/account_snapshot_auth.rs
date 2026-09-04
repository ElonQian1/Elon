use super::*;

#[test]
fn prevalidated_session_cannot_read_account_after_revocation_expiry_or_rebinding() {
    for change in [
        "UPDATE sessions SET revoked_at='synthetic-revocation' WHERE id='alice'",
        "UPDATE sessions SET expires_at='2000-01-01T00:00:00Z' WHERE id='alice'",
        "UPDATE sessions SET user_id='bob' WHERE id='alice'",
    ] {
        let fixture = Fixture::new();
        let policy = policy(100000000);
        record(&fixture, &policy, &prepare(&fixture, &policy));
        fixture
            .store
            .validate_esk_platform_session("alice", &token("alice"))
            .unwrap();
        assert_eq!(
            fixture
                .store
                .esk_platform_account("alice", &token("alice"), 20)
                .unwrap()
                .total_base_units,
            10000000,
        );

        // Reproduce the HTTP precheck -> account read boundary using two actual
        // SQLite connections. All sessions and amounts belong to this fixture.
        fixture.store.conn().unwrap().execute(change, []).unwrap();
        assert_error(
            fixture
                .store
                .validate_esk_platform_session("alice", &token("alice")),
            PlatformError::Unauthorized,
        );
        assert_error(
            fixture
                .store
                .esk_platform_account("alice", &token("alice"), 20),
            PlatformError::Unauthorized,
        );
    }
}

#[test]
fn empty_account_rejects_corrupt_pinned_policy_after_valid_session_precheck() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    prepare(&fixture, &policy);
    fixture
        .store
        .validate_esk_platform_session("bob", &token("bob"))
        .unwrap();
    assert_eq!(
        fixture
            .store
            .esk_platform_account("bob", &token("bob"), 20)
            .unwrap()
            .entry_count,
        0,
    );

    // Corruption is injected only into a freshly-created synthetic SQLite file.
    let conn = fixture.store.conn().unwrap();
    conn.execute_batch("DROP TRIGGER trg_esk_platform_policy_no_update")
        .unwrap();
    conn.execute(
        "UPDATE esk_platform_policy SET source_fingerprint=?1",
        ["0".repeat(64)],
    )
    .unwrap();
    assert_error(
        fixture.store.esk_platform_account("bob", &token("bob"), 20),
        PlatformError::CorruptLedger,
    );
}

#[test]
fn account_requires_the_current_real_users_own_session() {
    let fixture = Fixture::new();
    for (user, session) in [
        ("alice", token("bob")),
        ("bob", token("alice")),
        ("alice", String::new()),
        ("alice", "   ".into()),
        ("alice", "synthetic-unknown-session".into()),
        ("missing-user", token("missing-user")),
        ("inactive-user", token("inactive-user")),
        ("local-owner", token("local-owner")),
    ] {
        assert_error(
            fixture.store.esk_platform_account(user, &session, 20),
            PlatformError::Unauthorized,
        );
    }
    for user in ["alice", "bob", "admin-1", "owner-1"] {
        let account = fixture
            .store
            .esk_platform_account(user, &token(user), 20)
            .unwrap();
        assert_eq!(account.total_base_units, 0);
        assert_eq!(account.entry_count, 0);
        assert!(account.updated_at.is_none() && account.entries.is_empty());
    }
    assert_eq!(fixture.count("esk_platform_policy"), 0);
}

#[test]
fn account_session_expiry_uses_real_time_not_lexical_order() {
    let fixture = Fixture::new();
    for (expiry, allowed) in [
        ("not-a-date", false),
        ("2026-09-04T10:00:00Z", false),
        ("2026-09-04T17:59:59+08:00", false),
        ("2026-09-04T03:00:00-08:00", true),
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
        let result = fixture
            .store
            .esk_platform_account("alice", &token("alice"), 20);
        if allowed {
            assert_eq!(result.unwrap().entry_count, 0);
        } else {
            assert_error(result, PlatformError::Unauthorized);
        }
    }
}

#[test]
fn account_is_the_first_history_page_with_complete_totals_and_legacy_limit_clamping() {
    for count in [0_u32, 1, 7] {
        let fixture = Fixture::new();
        let policy = policy(100000000);
        for index in 0..count {
            history::post(&fixture, &policy, "alice", index);
        }
        history::post(&fixture, &policy, "bob", 100);
        for limit in [0_usize, 1, 2, 20, 100, 101, usize::MAX] {
            let account = fixture
                .store
                .esk_platform_account("alice", &token("alice"), limit)
                .unwrap();
            let history = fixture
                .store
                .esk_platform_history("alice", &token("alice"), limit.clamp(1, 100), None)
                .unwrap();
            assert_eq!(account.total_base_units, i64::from(count) * 10000000);
            assert_eq!(account.entry_count, i64::from(count));
            assert_eq!(
                account.entries.len(),
                (count as usize).min(limit.clamp(1, 100))
            );
            assert_eq!(account.total_base_units, history.total_base_units);
            assert_eq!(account.entry_count, history.entry_count);
            assert_eq!(account.updated_at, history.updated_at);
            assert_eq!(account.entries.len(), history.entries.len());
            for (actual, expected) in account.entries.iter().zip(&history.entries) {
                assert_eq!(actual.entry_id, expected.entry_id);
                assert_eq!(actual.allocation_id, expected.allocation_id);
                assert_eq!(actual.amount_base_units, expected.amount_base_units);
                assert_eq!(actual.created_at, expected.created_at);
            }
        }
        assert_eq!(fixture.paper_total(), 123000000);
    }
}

#[test]
fn account_rejects_off_page_corruption_but_checks_session_first() {
    for corruption in [
        "DROP TRIGGER trg_esk_platform_approvals_no_delete; DELETE FROM esk_platform_approvals WHERE allocation_id=(SELECT allocation_id FROM esk_platform_ledger_entries ORDER BY entry_id LIMIT 1)",
        "DROP TRIGGER trg_esk_platform_allocations_no_update; UPDATE esk_platform_allocations SET input_json='{}' WHERE allocation_id=(SELECT allocation_id FROM esk_platform_ledger_entries ORDER BY entry_id LIMIT 1)",
    ] {
        let fixture = Fixture::new();
        let policy = policy(100000000);
        for index in 0..3 {
            history::post(&fixture, &policy, "alice", index);
        }
        let conn = fixture.store.conn().unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute_batch(corruption).unwrap();
        assert_error(
            fixture.store.esk_platform_account("alice", &token("bob"), 1),
            PlatformError::Unauthorized,
        );
        assert_error(
            fixture.store.esk_platform_account("alice", &token("alice"), 1),
            PlatformError::CorruptLedger,
        );
    }
}

#[test]
fn repeated_account_reads_and_denials_do_not_commit_any_persistent_write() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    history::post(&fixture, &policy, "alice", 0);
    let observer = fixture.store.conn().unwrap();
    let tables = [
        "users",
        "sessions",
        "esk_platform_policy",
        "esk_platform_allocations",
        "esk_platform_approvals",
        "esk_platform_ledger_entries",
        "esk_platform_cancellations",
        "esk_asset_ledger_entries",
    ];
    let counts = tables.map(|table| fixture.count(table));
    let before: i64 = observer
        .query_row("PRAGMA data_version", [], |row| row.get(0))
        .unwrap();
    for _ in 0..3 {
        assert_eq!(
            fixture
                .store
                .esk_platform_account("alice", &token("alice"), 20)
                .unwrap()
                .total_base_units,
            10000000
        );
        assert_error(
            fixture
                .store
                .esk_platform_account("alice", &token("bob"), 20),
            PlatformError::Unauthorized,
        );
    }
    let after: i64 = observer
        .query_row("PRAGMA data_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(before, after, "Store account read committed a write");
    assert_eq!(counts, tables.map(|table| fixture.count(table)));
    assert_eq!(fixture.paper_total(), 123000000);
}
