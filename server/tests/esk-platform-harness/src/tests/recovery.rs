use super::*;
use std::sync::{Arc, Barrier};

#[test]
fn independent_connections_confirm_once_under_concurrency() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let prepared = prepare(&fixture, &policy);
    let barrier = Arc::new(Barrier::new(4));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let (store, policy, prepared, barrier) = (
                fixture.store.clone(),
                policy.clone(),
                prepared.clone(),
                Arc::clone(&barrier),
            );
            std::thread::spawn(move || {
                barrier.wait();
                store.record_esk_platform_allocation(
                    &policy,
                    &prepared.allocation_id,
                    &prepared.input.request_digest,
                    "admin-1",
                    &token("admin-1"),
                )
            })
        })
        .collect();
    let records: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap().unwrap())
        .collect();
    assert_eq!(records.iter().filter(|value| !value.replayed).count(), 1);
    assert_eq!(fixture.count("esk_platform_approvals"), 1);
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 1);
    assert_eq!(
        fixture
            .store
            .esk_platform_account("alice", &token("alice"), 20)
            .unwrap()
            .total_base_units,
        10000000
    );
    assert_eq!(fixture.paper_total(), 123000000);
}

#[test]
fn independent_connections_cannot_race_past_global_limit() {
    let fixture = Fixture::new();
    let policy = policy(15000000);
    let first = prepare(&fixture, &policy);
    let mut second = body();
    second.transfer_index = 1;
    second.user_id = "bob".into();
    let second = fixture
        .store
        .prepare_esk_platform_allocation(
            &policy,
            &prepare_input(&policy, second).unwrap(),
            "admin-1",
            &token("admin-1"),
        )
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = [first, second]
        .into_iter()
        .map(|prepared| {
            let (store, policy, barrier) =
                (fixture.store.clone(), policy.clone(), Arc::clone(&barrier));
            std::thread::spawn(move || {
                barrier.wait();
                store.record_esk_platform_allocation(
                    &policy,
                    &prepared.allocation_id,
                    &prepared.input.request_digest,
                    "admin-1",
                    &token("admin-1"),
                )
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    for result in results.into_iter().filter(Result::is_err) {
        assert_error(result, PlatformError::LimitExceeded);
    }
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 1);
}

#[test]
fn preparation_insert_failure_rolls_back_initial_policy_pin() {
    let fixture = Fixture::new();
    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER fixture_fail_prepare BEFORE INSERT ON esk_platform_allocations
         BEGIN SELECT RAISE(ABORT,'synthetic-prepare-failure'); END;",
        )
        .unwrap();
    let policy = policy(100000000);
    assert!(fixture
        .store
        .prepare_esk_platform_allocation(&policy, &input(&policy), "admin-1", &token("admin-1"))
        .is_err());
    assert_eq!(fixture.count("esk_platform_policy"), 0);
    assert_eq!(fixture.count("esk_platform_allocations"), 0);
    fixture.assert_empty_posting();
    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch("DROP TRIGGER fixture_fail_prepare")
        .unwrap();
    assert!(!prepare(&fixture, &policy).replayed);
}

#[test]
fn approval_or_ledger_failure_rolls_back_entire_posting_and_can_retry() {
    for table in ["esk_platform_approvals", "esk_platform_ledger_entries"] {
        let fixture = Fixture::new();
        let policy = policy(100000000);
        let prepared = prepare(&fixture, &policy);
        fixture
            .store
            .conn()
            .unwrap()
            .execute_batch(&format!(
                "CREATE TRIGGER fixture_fail_post BEFORE INSERT ON {table}
             BEGIN SELECT RAISE(ABORT,'synthetic-posting-failure'); END;",
            ))
            .unwrap();
        assert!(fixture
            .store
            .record_esk_platform_allocation(
                &policy,
                &prepared.allocation_id,
                &prepared.input.request_digest,
                "admin-1",
                &token("admin-1")
            )
            .is_err());
        fixture.assert_empty_posting();
        assert_eq!(fixture.count("esk_platform_allocations"), 1);
        fixture
            .store
            .conn()
            .unwrap()
            .execute_batch("DROP TRIGGER fixture_fail_post")
            .unwrap();
        assert!(record(&fixture, &policy, &prepared).recorded_at.is_some());
        assert_eq!(fixture.count("esk_platform_ledger_entries"), 1);
    }
}

#[test]
fn final_session_recheck_rolls_back_when_session_changes_inside_posting() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let prepared = prepare(&fixture, &policy);
    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER fixture_revoke_during_post AFTER INSERT ON esk_platform_ledger_entries
         BEGIN UPDATE sessions SET revoked_at='fixture' WHERE id='admin-1'; END;",
        )
        .unwrap();
    assert_error(
        fixture.store.record_esk_platform_allocation(
            &policy,
            &prepared.allocation_id,
            &prepared.input.request_digest,
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::Unauthorized,
    );
    fixture.assert_empty_posting();
    let revoked: Option<String> = fixture
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT revoked_at FROM sessions WHERE id='admin-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        revoked.is_none(),
        "the whole synthetic transaction must roll back"
    );
}

#[test]
fn append_only_triggers_reject_even_noop_updates_and_deletes() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let prepared = prepare(&fixture, &policy);
    record(&fixture, &policy, &prepared);
    let conn = fixture.store.conn().unwrap();
    for (table, key) in [
        ("esk_platform_policy", "singleton"),
        ("esk_platform_allocations", "allocation_id"),
        ("esk_platform_approvals", "approval_id"),
        ("esk_platform_ledger_entries", "entry_id"),
    ] {
        for sql in [
            format!("UPDATE {table} SET {key}={key}"),
            format!("DELETE FROM {table}"),
        ] {
            let error = conn.execute(&sql, []).unwrap_err();
            assert!(error.to_string().contains("append-only"), "{error}");
        }
        assert_eq!(fixture.count(table), 1);
    }
    assert_eq!(fixture.paper_total(), 123000000);
}

#[test]
fn reopened_database_and_repeated_migration_keep_receipts_and_totals() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let prepared = prepare(&fixture, &policy);
    let recorded = record(&fixture, &policy, &prepared);
    let before = fixture
        .store
        .esk_platform_account("alice", &token("alice"), 20)
        .unwrap();
    let reopened = Store {
        path: fixture.path.clone(),
    };
    platform_migration::migration_v287(&reopened.conn().unwrap()).unwrap();
    platform_migration::migration_v287(&reopened.conn().unwrap()).unwrap();
    let replay = reopened
        .record_esk_platform_allocation(
            &policy,
            &prepared.allocation_id,
            &prepared.input.request_digest,
            "admin-1",
            &token("admin-1"),
        )
        .unwrap();
    let after = reopened
        .esk_platform_account("alice", &token("alice"), 20)
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(recorded.recorded_at, replay.recorded_at);
    assert_eq!(before.total_base_units, after.total_base_units);
    assert_eq!(before.entries[0].entry_id, after.entries[0].entry_id);
    assert_eq!(before.updated_at, after.updated_at);
    assert_eq!(fixture.paper_total(), 123000000);
}

#[test]
fn orphan_approval_is_detected_as_corruption_not_available_balance() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let prepared = prepare(&fixture, &policy);
    fixture.store.conn().unwrap().execute(
        "INSERT INTO esk_platform_approvals(approval_id,allocation_id,request_digest,approved_by,created_at)
         VALUES('synthetic-orphan',?1,?2,'admin-1','fixture')",
        params![prepared.allocation_id, prepared.input.request_digest],
    ).unwrap();
    assert_error(
        fixture
            .store
            .esk_platform_account("alice", &token("alice"), 20),
        PlatformError::CorruptLedger,
    );
    assert_error(
        fixture.store.record_esk_platform_allocation(
            &policy,
            &prepared.allocation_id,
            &prepared.input.request_digest,
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::CorruptLedger,
    );
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 0);
}

#[test]
fn maximum_account_plus_one_micro_unit_is_rejected_without_wraparound() {
    let fixture = Fixture::new();
    let policy = policy(i64::MAX);
    let mut value = body();
    value.amount = "9223372036854.775807".into();
    value.payment_amount = "9223372036854.775807".into();
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
    let mut micro = body();
    micro.transfer_index = 1;
    micro.amount = "0.000001".into();
    micro.payment_amount = "0.000002".into();
    let pending = fixture
        .store
        .prepare_esk_platform_allocation(
            &policy,
            &prepare_input(&policy, micro).unwrap(),
            "admin-1",
            &token("admin-1"),
        )
        .unwrap();
    assert_error(
        fixture.store.record_esk_platform_allocation(
            &policy,
            &pending.allocation_id,
            &pending.input.request_digest,
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::LimitExceeded,
    );
    assert_eq!(
        fixture
            .store
            .esk_platform_account("alice", &token("alice"), 20)
            .unwrap()
            .total_base_units,
        i64::MAX
    );
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 1);
}
