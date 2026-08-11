use crate::{
    compute_federation_broker_service::{
        self, control_plane_tests::BrokerFixture, FinishMyComputeRequest,
    },
    store::{ComputeBrokerFinishAction, Store},
};

#[test]
fn reserve_release_and_replay_survive_two_database_reopens() {
    let fixture = BrokerFixture::new();
    let quoted = fixture.create_quoted_job("restart");
    fixture
        .supply
        .store
        .billing_recharge(
            &fixture.consumer_id,
            100,
            "broker_restart",
            &fixture.supply.admin_id,
            None,
        )
        .unwrap();

    let reserve_request = fixture.reserve_request(&quoted, "restart", 20, 1);
    let reserved = compute_federation_broker_service::reserve_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        reserve_request.clone(),
    )
    .unwrap();
    assert!(!reserved.replayed);

    let database = fixture.supply.root.join("state.sqlite");
    let consumer_id = fixture.consumer_id.clone();
    let project_id = fixture.project_id.clone();
    let token_bucket_id = fixture.supply.token_bucket_id.clone();
    let concurrency_bucket_id = fixture.supply.concurrency_bucket_id.clone();
    let job_id = quoted.job.job_id.clone();
    let reservation_id = reserved.reservation_id.clone();
    let finish_request = FinishMyComputeRequest {
        idempotency_key: "finish-restart-release".into(),
        expected_reservation_revision: reserved.reservation_revision,
        expected_reservation_digest: reserved.reservation_digest.clone(),
    };

    drop(fixture);
    let reopened = Store::open(&database).unwrap();
    assert_eq!(
        reopened.billing_get_balance(&consumer_id).unwrap(),
        Some(90)
    );
    assert_capacity(
        &reopened,
        &token_bucket_id,
        &concurrency_bucket_id,
        80,
        20,
        3,
        1,
    );

    let current_job = compute_federation_broker_service::get_job_for_user(
        &reopened,
        &consumer_id,
        Some(&project_id),
        &job_id,
    )
    .unwrap();
    assert_eq!(current_job.job.status, "reserved");
    let current_reservation = compute_federation_broker_service::get_reservation_for_user(
        &reopened,
        &consumer_id,
        Some(&project_id),
        &reservation_id,
    )
    .unwrap();
    assert_eq!(current_reservation.reservation.status, "active");
    assert_eq!(
        current_reservation.reservation_digest,
        reserved.reservation_digest
    );

    let reserve_replay = compute_federation_broker_service::reserve_for_user(
        &reopened,
        &consumer_id,
        Some(&project_id),
        reserve_request,
    )
    .unwrap();
    assert!(reserve_replay.replayed);
    assert_eq!(
        reserve_replay.reservation_digest,
        reserved.reservation_digest
    );

    let released = compute_federation_broker_service::finish_for_user(
        &reopened,
        &consumer_id,
        Some(&project_id),
        reservation_id.clone(),
        ComputeBrokerFinishAction::Release,
        finish_request.clone(),
    )
    .unwrap();
    assert!(!released.replayed);
    assert_eq!(released.status, "released");
    assert_eq!(released.budget_refunded_fen, 10);
    assert_eq!(
        reopened.billing_get_balance(&consumer_id).unwrap(),
        Some(100)
    );
    assert_capacity(
        &reopened,
        &token_bucket_id,
        &concurrency_bucket_id,
        100,
        0,
        4,
        0,
    );

    drop(reopened);
    let second_reopen = Store::open(&database).unwrap();
    let terminal_job = compute_federation_broker_service::get_job_for_user(
        &second_reopen,
        &consumer_id,
        Some(&project_id),
        &job_id,
    )
    .unwrap();
    assert_eq!(terminal_job.job.status, "canceled");
    assert_eq!(terminal_job.revision, released.terminal_job.job_revision);
    assert_eq!(terminal_job.job_digest, released.terminal_job.job_digest);

    let terminal_reservation = compute_federation_broker_service::get_reservation_for_user(
        &second_reopen,
        &consumer_id,
        Some(&project_id),
        &reservation_id,
    )
    .unwrap();
    assert_eq!(terminal_reservation.reservation.status, "released");
    assert_eq!(
        terminal_reservation.reservation_digest,
        released.reservation_digest
    );
    assert_eq!(
        second_reopen.billing_get_balance(&consumer_id).unwrap(),
        Some(100)
    );
    assert_capacity(
        &second_reopen,
        &token_bucket_id,
        &concurrency_bucket_id,
        100,
        0,
        4,
        0,
    );

    let release_replay = compute_federation_broker_service::finish_for_user(
        &second_reopen,
        &consumer_id,
        Some(&project_id),
        reservation_id,
        ComputeBrokerFinishAction::Release,
        finish_request,
    )
    .unwrap();
    assert!(release_replay.replayed);
    assert_eq!(
        release_replay.reservation_digest,
        released.reservation_digest
    );
    assert_eq!(release_replay.terminal_job, released.terminal_job);
}

#[allow(clippy::too_many_arguments)]
fn assert_capacity(
    store: &Store,
    token_bucket_id: &str,
    concurrency_bucket_id: &str,
    token_available: i64,
    token_held: i64,
    concurrency_available: i64,
    concurrency_held: i64,
) {
    let tokens = store
        .compute_capacity_bucket_balance(token_bucket_id)
        .unwrap();
    let concurrency = store
        .compute_capacity_bucket_balance(concurrency_bucket_id)
        .unwrap();
    assert_eq!(tokens.available_units, token_available);
    assert_eq!(tokens.held_units, token_held);
    assert_eq!(concurrency.available_units, concurrency_available);
    assert_eq!(concurrency.held_units, concurrency_held);
}
