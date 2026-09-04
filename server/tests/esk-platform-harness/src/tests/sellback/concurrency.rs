use super::*;
use std::sync::{Arc, Barrier};

#[test]
fn independent_connections_replay_one_request_and_one_cancellation() {
    let (fixture, _, config) = setup();
    let input = input(&fixture, "alice", "same-key", 7_000_000, &config);
    let barrier = Arc::new(Barrier::new(4));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let (store, config, input, barrier) = (
                fixture.store.clone(),
                config.clone(),
                input.clone(),
                barrier.clone(),
            );
            std::thread::spawn(move || {
                barrier.wait();
                store.submit_esk_platform_sellback("alice", &token("alice"), &input, &config)
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().unwrap().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|r| !r.replayed).count(), 1);
    assert!(results
        .iter()
        .all(|r| r.request.request_id == results[0].request.request_id));
    assert_eq!(fixture.count("esk_platform_sellback_requests"), 1);
    let barrier = Arc::new(Barrier::new(4));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let (store, config, id, barrier) = (
                fixture.store.clone(),
                config.clone(),
                results[0].request.request_id.clone(),
                barrier.clone(),
            );
            std::thread::spawn(move || {
                barrier.wait();
                store.cancel_esk_platform_sellback("alice", &token("alice"), &id, &config)
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().unwrap().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|r| !r.replayed).count(), 1);
    assert!(results
        .iter()
        .all(|r| r.request.cancel_event_id == results[0].request.cancel_event_id));
    assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 1);
    assert_eq!(
        page(&fixture, "alice", &config)
            .summary
            .available_base_units,
        10_000_000
    );
}

#[test]
fn different_keys_competing_on_one_snapshot_never_overreserve() {
    let (fixture, _, config) = setup();
    let barrier = Arc::new(Barrier::new(2));
    let inputs = [
        input(&fixture, "alice", "one", 7_000_000, &config),
        input(&fixture, "alice", "two", 7_000_000, &config),
    ];
    let handles: Vec<_> = inputs
        .into_iter()
        .map(|input| {
            let (store, config, barrier) = (fixture.store.clone(), config.clone(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                store.submit_esk_platform_sellback("alice", &token("alice"), &input, &config)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    for failed in results.into_iter().filter(Result::is_err) {
        error(failed, SellbackError::SnapshotChanged);
    }
    let retry = input(&fixture, "alice", "fresh-attempt", 7_000_000, &config);
    error(
        fixture
            .store
            .submit_esk_platform_sellback("alice", &token("alice"), &retry, &config),
        SellbackError::InsufficientAvailable,
    );
    assert_eq!(
        page(&fixture, "alice", &config).summary.reserved_base_units,
        7_000_000
    );
}

#[test]
fn different_users_cannot_race_past_global_cap() {
    let (fixture, _, config) = setup();
    let SellbackConfiguration::Enabled(mut policy) = config else {
        unreachable!()
    };
    policy.body.max_request_base_units = "6000000".into();
    policy.body.max_reserved_base_units_per_user = "6000000".into();
    policy.body.max_reserved_base_units_global = "6000000".into();
    let config = SellbackConfiguration::Enabled(validate_policy(policy.body).unwrap());
    let barrier = Arc::new(Barrier::new(2));
    let inputs: Vec<_> = ["alice", "bob"]
        .into_iter()
        .map(|u| {
            (
                u,
                input(&fixture, u, "same-key-different-owner", 4_000_000, &config),
            )
        })
        .collect();
    let handles: Vec<_> = inputs
        .into_iter()
        .map(|(user, input)| {
            let (store, config, barrier) = (fixture.store.clone(), config.clone(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                store.submit_esk_platform_sellback(user, &token(user), &input, &config)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    for failed in results.into_iter().filter(Result::is_err) {
        error(failed, SellbackError::LimitExceeded);
    }
    assert_eq!(
        page(&fixture, "alice", &config).summary.reserved_base_units
            + page(&fixture, "bob", &config).summary.reserved_base_units,
        4_000_000
    );
}

#[test]
fn old_policy_holds_still_count_towards_new_policy_global_cap_until_canceled() {
    let (fixture, _, config) = setup();
    let old = submit(&fixture, "bob", "old-policy", 7_000_000, &config);
    let SellbackConfiguration::Enabled(mut policy) = config else {
        unreachable!()
    };
    policy.body.revision = "new-policy".into();
    policy.body.max_request_base_units = "8000000".into();
    policy.body.max_reserved_base_units_per_user = "8000000".into();
    policy.body.max_reserved_base_units_global = "8000000".into();
    let config = SellbackConfiguration::Enabled(validate_policy(policy.body).unwrap());
    let new = input(&fixture, "alice", "new-policy", 2_000_000, &config);
    error(
        fixture
            .store
            .submit_esk_platform_sellback("alice", &token("alice"), &new, &config),
        SellbackError::LimitExceeded,
    );
    fixture
        .store
        .cancel_esk_platform_sellback(
            "bob",
            &token("bob"),
            &old.request.request_id,
            &SellbackConfiguration::Invalid,
        )
        .unwrap();
    assert!(
        !fixture
            .store
            .submit_esk_platform_sellback("alice", &token("alice"), &new, &config)
            .unwrap()
            .replayed
    );
}

#[test]
fn cancellation_racing_new_submission_is_serial_and_never_mints() {
    let (fixture, _, config) = setup();
    let held = submit(&fixture, "alice", "held", 7_000_000, &config);
    let new = input(&fixture, "alice", "new", 3_000_000, &config);
    let before = history::page(&fixture, "alice", 20, None).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let (store, cancel_config, cancel_barrier) =
        (fixture.store.clone(), config.clone(), barrier.clone());
    let cancel = std::thread::spawn(move || {
        cancel_barrier.wait();
        store.cancel_esk_platform_sellback(
            "alice",
            &token("alice"),
            &held.request.request_id,
            &cancel_config,
        )
    });
    let (store, submit_config) = (fixture.store.clone(), config.clone());
    let submit = std::thread::spawn(move || {
        barrier.wait();
        store.submit_esk_platform_sellback("alice", &token("alice"), &new, &submit_config)
    });
    cancel.join().unwrap().unwrap();
    match submit.join().unwrap() {
        Ok(value) => assert_eq!(value.request.input.amount_base_units, 3_000_000),
        Err(value) => error::<SellbackResult>(Err(value), SellbackError::SnapshotChanged),
    }
    let after = page(&fixture, "alice", &config);
    assert!(matches!(after.summary.reserved_base_units, 0 | 3_000_000));
    assert_eq!(
        history::page(&fixture, "alice", 20, None)
            .unwrap()
            .snapshot_digest,
        before.snapshot_digest
    );
    assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 1);
}

#[test]
fn official_allocation_racing_request_changes_snapshot_instead_of_losing_balance() {
    let (fixture, formal, config) = setup();
    let new = input(&fixture, "alice", "allocation-race", 3_000_000, &config);
    let mut body = super::super::body();
    body.transfer_index = 99;
    let allocation = crate::esk_asset::platform::prepare_input(&formal, body).unwrap();
    let prepared = fixture
        .store
        .prepare_esk_platform_allocation(&formal, &allocation, "admin-1", &token("admin-1"))
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let (store, formal, post_barrier) = (fixture.store.clone(), formal, barrier.clone());
    let post = std::thread::spawn(move || {
        post_barrier.wait();
        store.record_esk_platform_allocation(
            &formal,
            &prepared.allocation_id,
            &prepared.input.request_digest,
            "admin-1",
            &token("admin-1"),
        )
    });
    let (store, submit_config) = (fixture.store.clone(), config.clone());
    let submit = std::thread::spawn(move || {
        barrier.wait();
        store.submit_esk_platform_sellback("alice", &token("alice"), &new, &submit_config)
    });
    post.join().unwrap().unwrap();
    if let Err(value) = submit.join().unwrap() {
        error::<SellbackResult>(Err(value), SellbackError::SnapshotChanged);
    }
    let after = page(&fixture, "alice", &config);
    assert_eq!(after.summary.total_base_units, 20_000_000);
    assert!(matches!(after.summary.reserved_base_units, 0 | 3_000_000));
    assert_eq!(
        after.summary.available_base_units,
        after.summary.total_base_units - after.summary.reserved_base_units
    );
}
