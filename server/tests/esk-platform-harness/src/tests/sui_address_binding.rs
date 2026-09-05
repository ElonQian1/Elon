mod append_only;
mod helpers;

use chrono::{TimeZone, Utc};

use super::{token, Fixture};
use crate::esk_asset::platform::sui_address_binding::*;
use helpers::*;

#[test]
fn challenge_reuse_binding_replay_and_private_receipt_preserve_balances() {
    let fixture = Fixture::new();
    let first = fixture
        .store
        .create_esk_sui_address_binding_challenge("alice", &token("alice"), &material(address(), 1))
        .unwrap();
    let reused = fixture
        .store
        .create_esk_sui_address_binding_challenge("alice", &token("alice"), &material(address(), 2))
        .unwrap();
    assert_eq!(reused, first);
    assert_eq!(
        fixture.count("esk_platform_sui_address_binding_challenges"),
        1
    );

    let proof = verified(&first);
    let bound = fixture
        .store
        .complete_esk_sui_address_binding("alice", &token("alice"), &first.challenge_id, &proof)
        .unwrap();
    assert!(!bound.replayed);
    assert_eq!(bound.address, address());
    assert!(bound.binding_id.starts_with("eskpsb_"));
    assert!(bound.binding_receipt_sha256.starts_with("sha256:"));

    let replay = fixture
        .store
        .complete_esk_sui_address_binding("alice", &token("alice"), &first.challenge_id, &proof)
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.binding_id, bound.binding_id);
    assert_eq!(replay.binding_receipt_sha256, bound.binding_receipt_sha256);
    assert_eq!(fixture.count("esk_platform_sui_address_bindings"), 1);
    fixture.assert_empty_posting();
    assert_eq!(
        fixture
            .store
            .conn()
            .unwrap()
            .execute("DELETE FROM sessions WHERE id='alice'", [])
            .unwrap(),
        1
    );
    assert_eq!(fixture.count("esk_platform_sui_address_bindings"), 1);
}

#[test]
fn authentication_uniqueness_tamper_and_append_only_guards_fail_closed() {
    let fixture = Fixture::new();
    let challenge = fixture
        .store
        .create_esk_sui_address_binding_challenge("alice", &token("alice"), &material(address(), 3))
        .unwrap();
    let cross_user = fixture
        .store
        .load_esk_sui_address_binding_challenge("bob", &token("bob"), &challenge.challenge_id)
        .unwrap_err();
    assert_eq!(
        cross_user.downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::NotFound)
    );
    let cross_user = fixture
        .store
        .complete_esk_sui_address_binding(
            "bob",
            &token("bob"),
            &challenge.challenge_id,
            &verified(&challenge),
        )
        .unwrap_err();
    assert_eq!(
        cross_user.downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::NotFound)
    );

    let mut tampered = verified(&challenge);
    tampered.response_digest = format!("sha256:{}", "1".repeat(64));
    let error = fixture
        .store
        .complete_esk_sui_address_binding(
            "alice",
            &token("alice"),
            &challenge.challenge_id,
            &tampered,
        )
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::InvalidResponse)
    );

    let bound = fixture
        .store
        .complete_esk_sui_address_binding(
            "alice",
            &token("alice"),
            &challenge.challenge_id,
            &verified(&challenge),
        )
        .unwrap();
    let conflict = fixture.store.create_esk_sui_address_binding_challenge(
        "bob",
        &token("bob"),
        &material(bound.address.clone(), 4),
    );
    assert_eq!(
        conflict.unwrap_err().downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::Conflict)
    );
    let conflict = fixture.store.create_esk_sui_address_binding_challenge(
        "alice",
        &token("alice"),
        &material(format!("0x{}", "8".repeat(64)), 5),
    );
    assert_eq!(
        conflict.unwrap_err().downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::Conflict)
    );

    let conn = fixture.store.conn().unwrap();
    for statement in [
        "UPDATE esk_platform_sui_subjects SET user_id=user_id",
        "DELETE FROM esk_platform_sui_subjects",
        "UPDATE esk_platform_sui_address_binding_challenges SET user_id=user_id",
        "DELETE FROM esk_platform_sui_address_binding_challenges",
        "UPDATE esk_platform_sui_address_bindings SET user_id=user_id",
        "DELETE FROM esk_platform_sui_address_bindings",
    ] {
        assert!(
            conn.execute(statement, []).is_err(),
            "must reject {statement}"
        );
    }
    drop(conn);
    fixture.assert_empty_posting();
}

#[test]
fn live_and_rolling_limits_are_enforced_inside_the_ledger() {
    let fixture = Fixture::new();
    for index in 1..=3 {
        fixture
            .store
            .create_esk_sui_address_binding_challenge(
                "alice",
                &token("alice"),
                &material(format!("0x{:064x}", index), index as u8),
            )
            .unwrap();
    }
    let limited = fixture.store.create_esk_sui_address_binding_challenge(
        "alice",
        &token("alice"),
        &material(format!("0x{:064x}", 4), 4),
    );
    assert_eq!(
        limited.unwrap_err().downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::RateLimited)
    );

    let rolling = Fixture::new();
    insert_twenty_recent_expired_challenges(&rolling);
    let limited = rolling.store.create_esk_sui_address_binding_challenge(
        "alice",
        &token("alice"),
        &material(format!("0x{:064x}", 99), 99),
    );
    assert_eq!(
        limited.unwrap_err().downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::RateLimited)
    );
    rolling.assert_empty_posting();
}

#[test]
fn time_window_revocation_and_concurrent_consumption_fail_closed() {
    let future = Fixture::new();
    let error = future
        .store
        .create_esk_sui_address_binding_challenge(
            "alice",
            &token("alice"),
            &material_at(
                address(),
                7,
                "2026-09-04T10:01:00.000Z",
                "2026-09-04T10:11:00.000Z",
            ),
        )
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::NotYetValid)
    );
    let error = future
        .store
        .create_esk_sui_address_binding_challenge(
            "alice",
            &token("alice"),
            &material_at(
                address(),
                8,
                "2026-09-04T09:49:00.000Z",
                "2026-09-04T09:59:00.000Z",
            ),
        )
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::Expired)
    );
    assert_eq!(
        future.count("esk_platform_sui_address_binding_challenges"),
        0
    );

    let revoked = Fixture::new();
    let challenge = revoked
        .store
        .create_esk_sui_address_binding_challenge("alice", &token("alice"), &material(address(), 9))
        .unwrap();
    revoked
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE sessions SET revoked_at='2026-09-04T10:00:00.000Z' WHERE id='alice'",
            [],
        )
        .unwrap();
    let error = revoked
        .store
        .complete_esk_sui_address_binding(
            "alice",
            &token("alice"),
            &challenge.challenge_id,
            &verified(&challenge),
        )
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::Unauthorized)
    );
    assert_eq!(revoked.count("esk_platform_sui_address_bindings"), 0);

    let concurrent = Fixture::new();
    let challenge = concurrent
        .store
        .create_esk_sui_address_binding_challenge(
            "alice",
            &token("alice"),
            &material(address(), 10),
        )
        .unwrap();
    let proof = verified(&challenge);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let store = concurrent.store.clone();
        let barrier = barrier.clone();
        let challenge_id = challenge.challenge_id.clone();
        let proof = proof.clone();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            store.complete_esk_sui_address_binding("alice", &token("alice"), &challenge_id, &proof)
        }));
    }
    let outcomes = workers
        .into_iter()
        .map(|worker| worker.join().unwrap().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|value| !value.replayed).count(), 1);
    assert_eq!(outcomes.iter().filter(|value| value.replayed).count(), 1);
    assert_eq!(concurrent.count("esk_platform_sui_address_bindings"), 1);
    concurrent.assert_empty_posting();
}

#[test]
fn completion_rechecks_expired_and_future_challenges_inside_the_transaction() {
    for (issued_at, expires_at, recorded_at, verified_at, expected) in [
        (
            "2026-09-04T09:45:00.000Z",
            "2026-09-04T09:55:00.000Z",
            "2026-09-04T09:50:00.000Z",
            "2026-09-04T09:50:00.000Z",
            AddressBindingError::Expired,
        ),
        (
            "2026-09-04T10:05:00.000Z",
            "2026-09-04T10:15:00.000Z",
            "2026-09-04T10:06:00.000Z",
            "2026-09-04T10:06:00.000Z",
            AddressBindingError::NotYetValid,
        ),
    ] {
        let fixture = Fixture::new();
        let subject = subject_commitment(&[9_u8; 32]);
        let challenge =
            assemble_challenge(&subject, &material_at(address(), 12, issued_at, expires_at))
                .unwrap();
        insert_challenge(&fixture, &challenge, recorded_at);
        let proof = verify_wallet_response(
            &challenge,
            &wallet_response(&challenge),
            chrono::DateTime::parse_from_rfc3339(verified_at)
                .unwrap()
                .with_timezone(&Utc),
        )
        .unwrap();

        let error = fixture
            .store
            .complete_esk_sui_address_binding(
                "alice",
                &token("alice"),
                &challenge.challenge_id,
                &proof,
            )
            .unwrap_err();
        assert_eq!(error.downcast_ref::<AddressBindingError>(), Some(&expected));
        assert_eq!(fixture.count("esk_platform_sui_address_bindings"), 0);
        fixture.assert_empty_posting();
    }
}

#[test]
fn exact_replay_still_rechecks_expiry_inside_the_transaction() {
    let fixture = Fixture::new();
    let subject = subject_commitment(&[9_u8; 32]);
    let challenge = assemble_challenge(
        &subject,
        &material_at(
            address(),
            13,
            "2026-09-04T09:45:00.000Z",
            "2026-09-04T09:55:00.000Z",
        ),
    )
    .unwrap();
    insert_challenge(&fixture, &challenge, "2026-09-04T09:46:00.000Z");
    let proof = verify_wallet_response(
        &challenge,
        &wallet_response(&challenge),
        Utc.with_ymd_and_hms(2026, 9, 4, 9, 50, 0).single().unwrap(),
    )
    .unwrap();
    insert_binding(&fixture, &challenge, &proof, "2026-09-04T09:51:00.000Z");

    let error = fixture
        .store
        .complete_esk_sui_address_binding("alice", &token("alice"), &challenge.challenge_id, &proof)
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<AddressBindingError>(),
        Some(&AddressBindingError::Expired)
    );
    assert_eq!(fixture.count("esk_platform_sui_address_bindings"), 1);
    fixture.assert_empty_posting();
}

#[test]
fn migration_is_repeatable_without_replacing_existing_evidence() {
    let fixture = Fixture::new();
    let challenge = fixture
        .store
        .create_esk_sui_address_binding_challenge(
            "alice",
            &token("alice"),
            &material(address(), 11),
        )
        .unwrap();
    let binding = fixture
        .store
        .complete_esk_sui_address_binding(
            "alice",
            &token("alice"),
            &challenge.challenge_id,
            &verified(&challenge),
        )
        .unwrap();

    crate::sui_address_binding_migration::migration_v290(&fixture.store.conn().unwrap()).unwrap();

    assert_eq!(
        fixture.count("esk_platform_sui_address_binding_challenges"),
        1
    );
    assert_eq!(fixture.count("esk_platform_sui_address_bindings"), 1);
    assert_eq!(
        fixture
            .store
            .get_esk_sui_address_binding("alice", &token("alice"))
            .unwrap()
            .unwrap()
            .binding_id,
        binding.binding_id
    );
    fixture.assert_empty_posting();
}
