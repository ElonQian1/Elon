use std::sync::{Arc, Barrier};

use crate::{
    compute_federation_broker_service::{
        self, control_plane_tests::BrokerFixture, ReserveMyComputeRequest,
    },
    store::{ComputeBrokerReservationReceipt, Store},
};

#[test]
fn identical_concurrent_reserves_commit_once_and_replay_once() {
    let fixture = BrokerFixture::new();
    let quoted = fixture.create_quoted_job("concurrent-replay");
    fixture
        .supply
        .store
        .billing_recharge(
            &fixture.consumer_id,
            100,
            "broker_concurrency",
            &fixture.supply.admin_id,
            None,
        )
        .unwrap();
    let request = fixture.reserve_request(&quoted, "concurrent-replay", 20, 1);

    let results = race_reserves(&fixture, [request.clone(), request]);
    let receipts = results.into_iter().map(Result::unwrap).collect::<Vec<_>>();
    assert_eq!(
        receipts.iter().filter(|receipt| !receipt.replayed).count(),
        1
    );
    assert_eq!(
        receipts.iter().filter(|receipt| receipt.replayed).count(),
        1
    );
    assert_eq!(receipts[0].reservation_id, receipts[1].reservation_id);
    assert_eq!(
        receipts[0].reservation_digest,
        receipts[1].reservation_digest
    );
    assert_single_reservation_effect(&fixture, &quoted.job.job_id);
}

#[test]
fn competing_reservations_for_one_quoted_job_allow_only_one_winner() {
    let fixture = BrokerFixture::new();
    let quoted = fixture.create_quoted_job("concurrent-conflict");
    fixture
        .supply
        .store
        .billing_recharge(
            &fixture.consumer_id,
            100,
            "broker_concurrency",
            &fixture.supply.admin_id,
            None,
        )
        .unwrap();
    let first = fixture.reserve_request(&quoted, "concurrent-a", 20, 1);
    let mut second = first.clone();
    second.reservation_id = format!("reservation-concurrent-b-{}", fixture.consumer_id);
    second.idempotency_key = "reserve-concurrent-b".into();

    let results = race_reserves(&fixture, [first, second]);
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    let rejection = results
        .into_iter()
        .find_map(Result::err)
        .expect("one competing reservation must be rejected");
    assert!(
        rejection.contains("当前 quoted Job 精确版本"),
        "{rejection}"
    );
    assert_single_reservation_effect(&fixture, &quoted.job.job_id);
}

fn race_reserves(
    fixture: &BrokerFixture,
    requests: [ReserveMyComputeRequest; 2],
) -> [Result<ComputeBrokerReservationReceipt, String>; 2] {
    let database = fixture.supply.root.join("state.sqlite");
    let stores = [
        Store::open(&database).unwrap(),
        Store::open(&database).unwrap(),
    ];
    let barrier = Arc::new(Barrier::new(2));
    let consumer_id = fixture.consumer_id.clone();
    let project_id = fixture.project_id.clone();
    let mut handles = stores
        .into_iter()
        .zip(requests)
        .map(|(store, request)| {
            let barrier = Arc::clone(&barrier);
            let consumer_id = consumer_id.clone();
            let project_id = project_id.clone();
            std::thread::spawn(move || {
                barrier.wait();
                compute_federation_broker_service::reserve_for_user(
                    &store,
                    &consumer_id,
                    Some(&project_id),
                    request,
                )
                .map_err(|error| format!("{error:#}"))
            })
        })
        .collect::<Vec<_>>();
    let second = handles.pop().unwrap().join().unwrap();
    let first = handles.pop().unwrap().join().unwrap();
    [first, second]
}

fn assert_single_reservation_effect(fixture: &BrokerFixture, job_id: &str) {
    assert_eq!(
        fixture
            .supply
            .store
            .billing_get_balance(&fixture.consumer_id)
            .unwrap(),
        Some(90)
    );
    fixture.assert_capacity(80, 20, 3, 1);
    let job = compute_federation_broker_service::get_job_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        job_id,
    )
    .unwrap();
    assert_eq!(job.job.status, "reserved");
    let reservations = compute_federation_broker_service::list_reservations_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        10,
    )
    .unwrap();
    assert_eq!(reservations.len(), 1);
    assert_eq!(reservations[0].reservation.status, "active");
}
