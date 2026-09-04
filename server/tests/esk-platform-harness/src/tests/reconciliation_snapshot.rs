use super::reconciliation_snapshot_support::*;
use super::*;
use serde_json::json;
use std::sync::{Arc, Barrier};

#[test]
fn preparations_recording_cancellation_and_repreparation_keep_one_sorted_key() {
    let f = Fixture::new();
    let p = policy(100000000);
    let first = prepare(&f, &p);
    assert_counts(&snapshot(&f), 1, 0);
    record(&f, &p, &first);
    let mut second = body();
    second.transfer_index = 1;
    let input = prepare_input(&p, second).unwrap();
    let second = f
        .store
        .prepare_esk_platform_allocation(&p, &input, "admin-1", &token("admin-1"))
        .unwrap();
    let s = snapshot(&f);
    assert_counts(&s, 1, 1);
    assert_eq!(s.source_fingerprint, p.source_fingerprint);
    assert_eq!(s.policy_digest, p.policy_digest);
    let mut expected = vec![first.input.payment_key, second.input.payment_key.clone()];
    expected.sort();
    assert_eq!(s.used_payment_keys, expected);
    cancel(&f, &p, &second);
    assert_counts(&snapshot(&f), 0, 1);
    f.store
        .prepare_esk_platform_allocation(&p, &input, "admin-1", &token("admin-1"))
        .unwrap();
    assert_eq!(snapshot(&f).used_payment_keys, expected);
    assert_counts(&snapshot(&f), 1, 1);
    assert_eq!(f.paper_total(), 123000000);
}

#[test]
fn repeated_reads_change_no_sqlite_bytes_and_do_not_require_recipient_active() {
    let f = Fixture::new();
    let p = policy(100000000);
    let pending = prepare(&f, &p);
    record(&f, &p, &pending);
    f.store
        .conn()
        .unwrap()
        .execute("UPDATE users SET status='disabled' WHERE id='alice'", [])
        .unwrap();
    let before = fs::read(&f.path).unwrap();
    let first = snapshot(&f);
    let second = snapshot(&f);
    assert_eq!(first.used_payment_keys, second.used_payment_keys);
    assert_eq!(before, fs::read(&f.path).unwrap());
    assert_counts(&second, 0, 1);
    assert_eq!(f.paper_total(), 123000000);
    assert_eq!(f.count("esk_platform_ledger_entries"), 1);
    let observed = chrono::DateTime::parse_from_rfc3339(&second.observed_at).unwrap();
    assert_eq!(
        observed.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        second.observed_at
    );
}

#[test]
fn only_real_active_admin_or_owner_sessions_can_read() {
    let f = Fixture::new();
    prepare(&f, &policy(100000000));
    for actor in ["alice", "inactive-admin", "local-owner", "missing"] {
        assert_error(
            f.store
                .esk_platform_reconciliation_snapshot(actor, &token(actor)),
            PlatformError::Unauthorized,
        );
    }
    for credential in ["", " ", "wrong", &token("alice")] {
        assert_error(
            f.store
                .esk_platform_reconciliation_snapshot("admin-1", credential),
            PlatformError::Unauthorized,
        );
    }
    assert_counts(
        &f.store
            .esk_platform_reconciliation_snapshot("owner-1", &token("owner-1"))
            .unwrap(),
        1,
        0,
    );
}

#[test]
fn revoked_expired_malformed_or_downgraded_sessions_fail_without_business_writes() {
    for mutation in [
        "UPDATE sessions SET revoked_at='fixture' WHERE user_id='admin-1'",
        "UPDATE sessions SET expires_at='2000-01-01T00:00:00Z' WHERE user_id='admin-1'",
        "UPDATE sessions SET expires_at='not-a-date' WHERE user_id='admin-1'",
        "UPDATE users SET role='user' WHERE id='admin-1'",
        "UPDATE users SET status='disabled' WHERE id='admin-1'",
    ] {
        let f = Fixture::new();
        prepare(&f, &policy(100000000));
        f.store.conn().unwrap().execute_batch(mutation).unwrap();
        let before = fs::read(&f.path).unwrap();
        assert_error(
            f.store
                .esk_platform_reconciliation_snapshot("admin-1", &token("admin-1")),
            PlatformError::Unauthorized,
        );
        assert_eq!(before, fs::read(&f.path).unwrap());
    }
}

#[test]
fn absent_policy_is_not_an_empty_complete_history_but_all_canceled_is_empty() {
    let f = Fixture::new();
    assert_error(
        f.store
            .esk_platform_reconciliation_snapshot("admin-1", &token("admin-1")),
        PlatformError::InvalidPolicy,
    );
    let p = policy(100000000);
    let pending = prepare(&f, &p);
    cancel(&f, &p, &pending);
    assert_counts(&snapshot(&f), 0, 0);
    let conn = f.store.conn().unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys=OFF; DROP TRIGGER trg_esk_platform_policy_no_delete;
        DELETE FROM esk_platform_policy",
    )
    .unwrap();
    assert_error(
        f.store
            .esk_platform_reconciliation_snapshot("admin-1", &token("admin-1")),
        PlatformError::CorruptLedger,
    );
}

#[test]
fn corrupt_policy_or_pending_input_is_never_exported() {
    for mutation in [
        "DROP TRIGGER trg_esk_platform_policy_no_update; UPDATE esk_platform_policy SET source_json='{}'",
        "DROP TRIGGER trg_esk_platform_policy_no_update; UPDATE esk_platform_policy SET source_fingerprint=lower(hex(randomblob(32)))",
        "DROP TRIGGER trg_esk_platform_allocations_no_update; UPDATE esk_platform_allocations SET input_json='{}'",
        "DROP TRIGGER trg_esk_platform_allocations_no_update; UPDATE esk_platform_allocations SET amount_base_units=1",
    ] {
        let f = Fixture::new();
        prepare(&f, &policy(100000000));
        f.store.conn().unwrap().execute_batch(mutation).unwrap();
        assert_error(f.store.esk_platform_reconciliation_snapshot("admin-1", &token("admin-1")), PlatformError::CorruptLedger);
    }
}

#[test]
fn partial_recorded_ledger_is_rejected() {
    let f = Fixture::new();
    let p = policy(100000000);
    record(&f, &p, &prepare(&f, &p));
    f.store
        .conn()
        .unwrap()
        .execute_batch(
            "DROP TRIGGER trg_esk_platform_ledger_entries_no_delete;
        DELETE FROM esk_platform_ledger_entries",
        )
        .unwrap();
    assert_error(
        f.store
            .esk_platform_reconciliation_snapshot("admin-1", &token("admin-1")),
        PlatformError::CorruptLedger,
    );
}

#[test]
fn duplicate_current_payment_claims_are_not_silently_deduplicated() {
    let f = Fixture::new();
    prepare(&f, &policy(100000000));
    f.store
        .conn()
        .unwrap()
        .execute_batch(
            "DROP TRIGGER trg_esk_platform_payment_current;
        INSERT INTO esk_platform_allocations SELECT 'synthetic-duplicate',payment_key,policy_digest,
        user_id,amount_base_units,request_digest,input_json,prepared_by,prepared_at
        FROM esk_platform_allocations LIMIT 1",
        )
        .unwrap();
    assert_error(
        f.store
            .esk_platform_reconciliation_snapshot("admin-1", &token("admin-1")),
        PlatformError::CorruptLedger,
    );
}

#[test]
fn validly_bound_rows_exceeding_the_pinned_total_limit_fail_closed() {
    let f = Fixture::new();
    seed_pending(&f, &policy(15000000), 2);
    // Deliberately corrupt only the aggregate invariant; row bindings stay valid.
    f.store
        .conn()
        .unwrap()
        .execute_batch(
            "INSERT INTO esk_platform_approvals
        SELECT 'approval-'||allocation_id,allocation_id,request_digest,'admin-1','fixture'
        FROM esk_platform_allocations;
        INSERT INTO esk_platform_ledger_entries
        SELECT 'entry-'||allocation_id,allocation_id,'approval-'||allocation_id,user_id,
        amount_base_units,'fixture' FROM esk_platform_allocations",
        )
        .unwrap();
    assert_error(
        f.store
            .esk_platform_reconciliation_snapshot("admin-1", &token("admin-1")),
        PlatformError::CorruptLedger,
    );
}

#[test]
fn key_budget_allows_exact_limit_and_rejects_overflow_without_truncation() {
    for count in [
        PLATFORM_PAYMENT_SNAPSHOT_MAX_KEYS,
        PLATFORM_PAYMENT_SNAPSHOT_MAX_KEYS + 1,
    ] {
        let f = Fixture::new();
        seed_pending(&f, &policy(i64::MAX), count);
        let result = f
            .store
            .esk_platform_reconciliation_snapshot("admin-1", &token("admin-1"));
        if count == PLATFORM_PAYMENT_SNAPSHOT_MAX_KEYS {
            assert_counts(&result.unwrap(), count, 0);
        } else {
            assert_error(result, PlatformError::LimitExceeded);
        }
        assert_eq!(f.count("esk_platform_allocations"), count as i64);
        f.assert_empty_posting();
    }
}

#[test]
fn concurrent_recording_returns_a_whole_before_or_after_snapshot() {
    let f = Fixture::new();
    let p = policy(100000000);
    let pending = prepare(&f, &p);
    let barrier = Arc::new(Barrier::new(2));
    let (store, gate) = (f.store.clone(), Arc::clone(&barrier));
    let writer = std::thread::spawn(move || {
        gate.wait();
        store
            .record_esk_platform_allocation(
                &p,
                &pending.allocation_id,
                &pending.input.request_digest,
                "admin-1",
                &token("admin-1"),
            )
            .unwrap();
    });
    barrier.wait();
    let s = serde_json::to_value(snapshot(&f)).unwrap();
    writer.join().unwrap();
    assert_eq!(s["key_count"], "1");
    assert!(
        (s["prepared_count"] == "1" && s["recorded_count"] == "0")
            || (s["prepared_count"] == "0" && s["recorded_count"] == "1")
    );
    assert_counts(&snapshot(&f), 0, 1);
}

#[test]
fn actual_store_snapshot_roundtrips_through_node_cli_without_business_writes() {
    let f = Fixture::new();
    let p = policy(100000000);
    let pending = prepare(&f, &p);
    let before = fs::read(&f.path).unwrap();
    let blocked = cli_preview(&snapshot(&f), 2);
    assert!(blocked["preview"]["rows"][0]["reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("PAYMENT_ALREADY_USED")));
    assert_eq!(before, fs::read(&f.path).unwrap());
    cancel(&f, &p, &pending);
    assert_eq!(
        cli_preview(&snapshot(&f), 0)["preview"]["rows"][0]["status"],
        "review_ready"
    );
    let fresh = prepare(&f, &p);
    assert_eq!(
        cli_preview(&snapshot(&f), 2)["preview"]["rows"][0]["status"],
        "blocked"
    );
    record(&f, &p, &fresh);
    assert_eq!(
        cli_preview(&snapshot(&f), 2)["preview"]["rows"][0]["status"],
        "blocked"
    );
    assert_eq!(f.count("esk_platform_ledger_entries"), 1);
    assert_eq!(f.paper_total(), 123000000);
}
