use std::sync::{Arc, Barrier};

use rusqlite::Connection;

use crate::compute_attempt_terminal_migration;

use super::compute_attempt_consumer_reviews::{
    ReviewComputeAttemptTerminalCandidateRequest, CONSUMER_REVIEW_ACCEPTED,
};

mod support;

use support::{
    drop_final_usage_triggers, insert_drifted_usage, trigger_count, LiveAttemptFixture,
    CANDIDATE_HEAD_TRIGGER, USAGE_SEAL_TRIGGER,
};

#[test]
fn final_usage_fence_seals_new_writes_but_preserves_exact_replays() {
    let fixture = LiveAttemptFixture::new("seal");
    assert_eq!(trigger_count(&fixture.path, USAGE_SEAL_TRIGGER), 1);
    assert_eq!(trigger_count(&fixture.path, CANDIDATE_HEAD_TRIGGER), 1);

    let usage_request = fixture.usage_request(1, 5, "usage-seal-1");
    let usage = fixture
        .broker
        .supply
        .store
        .declare_compute_attempt_usage(&usage_request)
        .unwrap();
    let candidate_request = fixture.candidate_request(&usage, "candidate-seal");
    let candidate = fixture
        .broker
        .supply
        .store
        .declare_compute_attempt_terminal_candidate(&candidate_request)
        .unwrap();

    assert!(
        fixture
            .broker
            .supply
            .store
            .declare_compute_attempt_usage(&usage_request)
            .unwrap()
            .replayed
    );
    assert!(
        fixture
            .broker
            .supply
            .store
            .declare_compute_attempt_terminal_candidate(&candidate_request)
            .unwrap()
            .replayed
    );
    assert_eq!(
        fixture
            .broker
            .supply
            .store
            .compute_attempt_terminal_candidate(&fixture.lease.lease.lease_id)
            .unwrap()
            .terminal_candidate_id,
        candidate.terminal_candidate_id
    );

    let new_usage = fixture.usage_request(2, 7, "usage-seal-2");
    let error = fixture
        .broker
        .supply
        .store
        .declare_compute_attempt_usage(&new_usage)
        .unwrap_err();
    assert!(format!("{error:#}").contains("已封口"));

    let mut reused_key = fixture.usage_request(2, 8, "usage-seal-1");
    reused_key.executor_usage_ref = "usage://different".into();
    assert!(fixture
        .broker
        .supply
        .store
        .declare_compute_attempt_usage(&reused_key)
        .is_err());

    let mut reused_sequence = fixture.usage_request(1, 8, "usage-seal-different");
    reused_sequence.executor_usage_ref = "usage://different-sequence".into();
    assert!(fixture
        .broker
        .supply
        .store
        .declare_compute_attempt_usage(&reused_sequence)
        .is_err());
}

#[test]
fn usage_append_and_terminal_candidate_race_linearize_across_connections() {
    let fixture = LiveAttemptFixture::new("race");
    let first_usage = fixture.declare_usage(1, 5, "usage-race-1");
    let candidate_request = fixture.candidate_request(&first_usage, "candidate-race");
    let usage_request = fixture.usage_request(2, 7, "usage-race-2");
    let candidate_store = fixture.open_peer();
    let usage_store = fixture.open_peer();
    let barrier = Arc::new(Barrier::new(3));

    let candidate_barrier = barrier.clone();
    let candidate_thread = std::thread::spawn(move || {
        candidate_barrier.wait();
        candidate_store.declare_compute_attempt_terminal_candidate(&candidate_request)
    });
    let usage_barrier = barrier.clone();
    let usage_thread = std::thread::spawn(move || {
        usage_barrier.wait();
        usage_store.declare_compute_attempt_usage(&usage_request)
    });
    barrier.wait();

    let candidate_result = candidate_thread.join().unwrap();
    let usage_result = usage_thread.join().unwrap();
    assert_ne!(candidate_result.is_ok(), usage_result.is_ok());

    let latest = fixture
        .broker
        .supply
        .store
        .latest_compute_attempt_usage_declaration(&fixture.lease.lease.lease_id)
        .unwrap();
    match fixture
        .broker
        .supply
        .store
        .compute_attempt_terminal_candidate(&fixture.lease.lease.lease_id)
    {
        Ok(candidate) => {
            assert_eq!(candidate.final_usage_snapshot_id, latest.snapshot_id);
            assert_eq!(candidate.final_usage_sequence_no, latest.sequence_no);
        }
        Err(_) => assert_eq!(latest.sequence_no, 2),
    }
}

#[test]
fn v226_migration_rejects_legacy_drift_without_installing_partial_triggers() {
    let fixture = LiveAttemptFixture::new("legacy-drift");
    let usage = fixture.declare_usage(1, 5, "usage-legacy-drift");
    fixture.declare_candidate(&usage, "candidate-legacy-drift");
    let path = fixture.path.clone();
    let lease_id = fixture.lease.lease.lease_id.clone();
    drop(fixture);

    let connection = Connection::open(&path).unwrap();
    drop_final_usage_triggers(&connection);
    insert_drifted_usage(&connection, &lease_id);
    let error = compute_attempt_terminal_migration::migration_v226(&connection).unwrap_err();
    assert!(format!("{error:#}").contains("do not bind the exact current usage head"));
    drop(connection);
    assert_eq!(trigger_count(&path, USAGE_SEAL_TRIGGER), 0);
    assert_eq!(trigger_count(&path, CANDIDATE_HEAD_TRIGGER), 0);
}

#[test]
fn v226_migration_installs_both_triggers_for_consistent_legacy_history() {
    let fixture = LiveAttemptFixture::new("legacy-clean");
    let usage = fixture.declare_usage(1, 5, "usage-legacy-clean");
    fixture.declare_candidate(&usage, "candidate-legacy-clean");
    let path = fixture.path.clone();
    drop(fixture);

    let connection = Connection::open(&path).unwrap();
    drop_final_usage_triggers(&connection);
    compute_attempt_terminal_migration::migration_v226(&connection).unwrap();
    drop(connection);
    assert_eq!(trigger_count(&path, USAGE_SEAL_TRIGGER), 1);
    assert_eq!(trigger_count(&path, CANDIDATE_HEAD_TRIGGER), 1);
}

#[test]
fn historical_drift_blocks_candidate_reads_and_consumer_review_writes() {
    let fixture = LiveAttemptFixture::new("read-drift");
    let usage = fixture.declare_usage(1, 5, "usage-read-drift");
    let candidate = fixture.declare_candidate(&usage, "candidate-read-drift");
    {
        let connection = Connection::open(&fixture.path).unwrap();
        connection
            .execute(&format!("DROP TRIGGER {USAGE_SEAL_TRIGGER}"), [])
            .unwrap();
        insert_drifted_usage(&connection, &fixture.lease.lease.lease_id);
    }

    let read_result = fixture
        .broker
        .supply
        .store
        .compute_attempt_terminal_candidate(&fixture.lease.lease.lease_id);
    assert!(read_result.is_err());

    let review = fixture
        .broker
        .supply
        .store
        .review_compute_attempt_terminal_candidate(&ReviewComputeAttemptTerminalCandidateRequest {
            lease_id: fixture.lease.lease.lease_id.clone(),
            expected_terminal_candidate_id: candidate.terminal_candidate_id,
            expected_terminal_candidate_event_digest: candidate.event_digest,
            decision: CONSUMER_REVIEW_ACCEPTED.into(),
            reason_code: "accepted".into(),
            consumer_review_ref: "review://must-not-persist".into(),
            evidence_refs: Vec::new(),
            idempotency_key: "review-read-drift".into(),
            reviewed_by_user_id: fixture.broker.consumer_id.clone(),
        });
    assert!(review.is_err());
    let review_count: i64 = fixture
        .broker
        .supply
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM compute_attempt_consumer_reviews WHERE lease_id=?1",
            [&fixture.lease.lease.lease_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(review_count, 0);
}
