use super::*;
use std::sync::{Arc, Barrier};

fn cancel(
    fixture: &Fixture,
    policy: &PlatformPolicy,
    pending: &PlatformAllocationRecord,
) -> anyhow::Result<PlatformAllocationRecord> {
    fixture.store.cancel_esk_platform_allocation(
        policy,
        &pending.allocation_id,
        &pending.input.request_digest,
        "admin-1",
        &token("admin-1"),
    )
}

#[test]
fn cancel_is_idempotent_and_old_application_can_never_be_recorded() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let pending = prepare(&fixture, &policy);
    let canceled = cancel(&fixture, &policy, &pending).unwrap();
    assert!(canceled.canceled_at.is_some());
    assert!(canceled.recorded_at.is_none());
    assert!(!canceled.replayed);
    let replay = cancel(&fixture, &policy, &pending).unwrap();
    assert!(replay.replayed);
    assert_eq!(canceled.canceled_at, replay.canceled_at);
    assert_error(
        fixture.store.record_esk_platform_allocation(
            &policy,
            &pending.allocation_id,
            &pending.input.request_digest,
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::Conflict,
    );
    assert_eq!(fixture.count("esk_platform_cancellations"), 1);
    fixture.assert_empty_posting();
}

#[test]
fn canceled_payment_can_be_prepared_with_corrected_user_and_amount_then_recorded_once() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let wrong = prepare(&fixture, &policy);
    cancel(&fixture, &policy, &wrong).unwrap();
    let mut corrected = body();
    corrected.user_id = "bob".into();
    corrected.payment_amount = "40".into();
    corrected.amount = "20".into();
    corrected.payment_evidence_digest = "7".repeat(64);
    let corrected = prepare_input(&policy, corrected).unwrap();
    assert_eq!(wrong.input.payment_key, corrected.payment_key);
    let fresh = fixture
        .store
        .prepare_esk_platform_allocation(&policy, &corrected, "admin-1", &token("admin-1"))
        .unwrap();
    assert_ne!(wrong.allocation_id, fresh.allocation_id);
    assert!(fresh.canceled_at.is_none());
    assert!(!fresh.replayed);
    let replay = fixture
        .store
        .prepare_esk_platform_allocation(&policy, &corrected, "admin-1", &token("admin-1"))
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(fresh.allocation_id, replay.allocation_id);
    record(&fixture, &policy, &fresh);
    assert!(record(&fixture, &policy, &fresh).replayed);
    assert_error(
        fixture.store.record_esk_platform_allocation(
            &policy,
            &wrong.allocation_id,
            &wrong.input.request_digest,
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::Conflict,
    );
    assert_eq!(fixture.count("esk_platform_allocations"), 2);
    assert_eq!(fixture.count("esk_platform_cancellations"), 1);
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 1);
    assert_eq!(
        fixture
            .store
            .esk_platform_account("alice", 20)
            .unwrap()
            .total_base_units,
        0
    );
    assert_eq!(
        fixture
            .store
            .esk_platform_account("bob", 20)
            .unwrap()
            .total_base_units,
        20000000
    );
    assert_eq!(fixture.paper_total(), 123000000);
}

#[test]
fn recorded_allocation_is_not_cancelable_and_does_not_release_payment_key() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let pending = prepare(&fixture, &policy);
    record(&fixture, &policy, &pending);
    assert_error(cancel(&fixture, &policy, &pending), PlatformError::Conflict);
    let mut changed = body();
    changed.user_id = "bob".into();
    assert_error(
        fixture.store.prepare_esk_platform_allocation(
            &policy,
            &prepare_input(&policy, changed).unwrap(),
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::Conflict,
    );
    assert_eq!(fixture.count("esk_platform_cancellations"), 0);
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 1);
}

#[test]
fn invalid_digest_expired_session_or_policy_drift_cannot_cancel() {
    let fixture = Fixture::new();
    let first_policy = policy(100000000);
    let pending = prepare(&fixture, &first_policy);
    assert_error(
        fixture.store.cancel_esk_platform_allocation(
            &first_policy,
            &pending.allocation_id,
            &"0".repeat(64),
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::Conflict,
    );
    assert_error(
        cancel(&fixture, &policy(200000000), &pending),
        PlatformError::PolicyChanged,
    );
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE sessions SET expires_at='2000-01-01T00:00:00Z' WHERE id='admin-1'",
            [],
        )
        .unwrap();
    assert_error(
        cancel(&fixture, &first_policy, &pending),
        PlatformError::Unauthorized,
    );
    assert_eq!(fixture.count("esk_platform_cancellations"), 0);
    fixture.assert_empty_posting();
}

#[test]
fn unavailable_recipient_does_not_trap_a_bad_unrecorded_application() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let pending = prepare(&fixture, &policy);
    fixture
        .store
        .conn()
        .unwrap()
        .execute("UPDATE users SET status='disabled' WHERE id='alice'", [])
        .unwrap();
    assert!(cancel(&fixture, &policy, &pending)
        .unwrap()
        .canceled_at
        .is_some());
    assert_error(
        fixture.store.record_esk_platform_allocation(
            &policy,
            &pending.allocation_id,
            &pending.input.request_digest,
            "admin-1",
            &token("admin-1"),
        ),
        PlatformError::Conflict,
    );
    assert_eq!(fixture.count("esk_platform_ledger_entries"), 0);
}

#[test]
fn cancellation_and_recording_race_has_exactly_one_winner() {
    // Each iteration uses two independent SQLite connections and a simultaneous start.
    for reverse_spawn_order in [false, true] {
        let fixture = Fixture::new();
        let policy = policy(100000000);
        let pending = prepare(&fixture, &policy);
        let barrier = Arc::new(Barrier::new(2));
        let actions = if reverse_spawn_order {
            [true, false]
        } else {
            [false, true]
        };
        let handles: Vec<_> = actions
            .into_iter()
            .map(|is_cancel| {
                let (store, policy, pending, barrier) = (
                    fixture.store.clone(),
                    policy.clone(),
                    pending.clone(),
                    Arc::clone(&barrier),
                );
                std::thread::spawn(move || {
                    barrier.wait();
                    if is_cancel {
                        store.cancel_esk_platform_allocation(
                            &policy,
                            &pending.allocation_id,
                            &pending.input.request_digest,
                            "admin-1",
                            &token("admin-1"),
                        )
                    } else {
                        store.record_esk_platform_allocation(
                            &policy,
                            &pending.allocation_id,
                            &pending.input.request_digest,
                            "admin-1",
                            &token("admin-1"),
                        )
                    }
                })
            })
            .collect();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        for result in results.into_iter().filter(Result::is_err) {
            assert_error(result, PlatformError::Conflict);
        }
        let approvals = fixture.count("esk_platform_approvals");
        let cancellations = fixture.count("esk_platform_cancellations");
        assert_eq!(approvals + cancellations, 1);
        assert_eq!(fixture.count("esk_platform_ledger_entries"), approvals);
        assert_eq!(fixture.paper_total(), 123000000);
    }
}

#[test]
fn cancellation_is_append_only_survives_restart_and_repeated_migration() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let pending = prepare(&fixture, &policy);
    let canceled = cancel(&fixture, &policy, &pending).unwrap();
    let conn = fixture.store.conn().unwrap();
    for sql in [
        "DELETE FROM esk_platform_cancellations",
        "UPDATE esk_platform_cancellations SET allocation_id=allocation_id",
    ] {
        assert!(conn
            .execute(sql, [])
            .unwrap_err()
            .to_string()
            .contains("append-only"));
    }
    platform_migration::migration_v287(&conn).unwrap();
    let reopened = Store {
        path: fixture.path.clone(),
    };
    let replay = reopened
        .cancel_esk_platform_allocation(
            &policy,
            &pending.allocation_id,
            &pending.input.request_digest,
            "admin-1",
            &token("admin-1"),
        )
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.canceled_at, canceled.canceled_at);
    assert_eq!(fixture.count("esk_platform_cancellations"), 1);
}

#[test]
fn cancellation_failure_leaves_old_application_retryable_without_partial_receipt() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let pending = prepare(&fixture, &policy);
    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER fixture_fail_cancel AFTER INSERT ON esk_platform_cancellations
         BEGIN SELECT RAISE(ABORT,'synthetic-cancel-failure'); END;",
        )
        .unwrap();
    assert!(cancel(&fixture, &policy, &pending).is_err());
    assert_eq!(fixture.count("esk_platform_cancellations"), 0);
    fixture.assert_empty_posting();
    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch("DROP TRIGGER fixture_fail_cancel")
        .unwrap();
    assert!(record(&fixture, &policy, &pending).recorded_at.is_some());
}
