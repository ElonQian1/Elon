use std::sync::{Arc, Barrier};

use crate::{
    open_commerce_adapter_claim_service,
    open_commerce_adapter_claim_tests::{claim_enabled_credential, fixture},
    store::Store,
};

#[test]
fn independent_workers_claim_one_terminal_order_exactly_once() {
    const WORKERS: usize = 8;

    let fixture = fixture();
    let credential = claim_enabled_credential(&fixture);
    let stores = (0..WORKERS)
        .map(|_| Store::open(&fixture.path).unwrap())
        .collect::<Vec<_>>();
    let barrier = Arc::new(Barrier::new(WORKERS));
    let handles = stores
        .into_iter()
        .map(|store| {
            let barrier = Arc::clone(&barrier);
            let credential = credential.clone();
            std::thread::spawn(move || {
                barrier.wait();
                open_commerce_adapter_claim_service::claim_next(&store, &credential, 300)
                    .map_err(|error| format!("{error:#}"))
            })
        })
        .collect::<Vec<_>>();
    let polls = handles
        .into_iter()
        .map(|handle| handle.join().expect("claim worker should not panic"))
        .collect::<Result<Vec<_>, _>>()
        .expect("all independent claim workers should complete without a database error");

    assert_eq!(polls.iter().filter(|poll| poll.claimed).count(), 1);
    assert_eq!(polls.iter().filter(|poll| poll.issue.is_some()).count(), 1);
    let winner = polls
        .into_iter()
        .find_map(|poll| poll.issue)
        .expect("one worker should own the lease");
    assert_eq!(winner.claim.invocation_id, fixture.invocation_id);
    assert_eq!(winner.claim.attempt_no, 1);

    let claims = fixture
        .store
        .list_project_open_commerce_adapter_handoff_claims(&fixture.project_id, 20)
        .unwrap();
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].status, "active");
    assert_eq!(claims[0].id, winner.claim.id);
}
