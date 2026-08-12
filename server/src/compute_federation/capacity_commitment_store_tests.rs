use crate::{
    compute_federation::capacity_commitment_service::{
        self, CancelCapacityCommitmentBody, ExpireDueCapacityCommitmentsBody,
    },
    store::Store,
};

use super::test_support::{wait_until, Fixture};

#[test]
fn commitment_create_cancel_is_atomic_idempotent_and_reopenable() {
    let fixture = Fixture::new();
    assert_eq!(fixture.balance(&fixture.token_bucket_id), (100, 0));
    assert_eq!(fixture.balance(&fixture.concurrency_bucket_id), (4, 0));

    let denied = capacity_commitment_service::create_for_owner(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_body("commitment-primary", false),
    )
    .unwrap_err();
    assert!(denied.to_string().contains("显式确认"));
    assert_table_count(&fixture.store, "compute_capacity_commitments", 0);
    assert_eq!(fixture.balance(&fixture.token_bucket_id), (100, 0));

    let created = capacity_commitment_service::create_for_owner(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_body("commitment-primary", true),
    )
    .unwrap();
    assert!(!created.replayed);
    assert_eq!(created.commitment.commitment_status, "committed");
    assert_eq!(fixture.balance(&fixture.token_bucket_id), (80, 20));
    assert_eq!(fixture.balance(&fixture.concurrency_bucket_id), (3, 1));

    let replayed = capacity_commitment_service::create_for_owner(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_body("commitment-primary", true),
    )
    .unwrap();
    assert!(replayed.replayed);
    assert_eq!(
        replayed.commitment.commitment_id,
        created.commitment.commitment_id
    );
    assert_eq!(fixture.balance(&fixture.token_bucket_id), (80, 20));

    let outsider = capacity_commitment_service::get_for_owner(
        &fixture.store,
        "not-the-owner",
        &fixture.provider_id,
        &fixture.pool_id,
        &created.commitment.commitment_id,
    )
    .unwrap_err();
    assert!(outsider.to_string().contains("不存在"));

    let canceled = capacity_commitment_service::cancel_for_owner(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &created.commitment.commitment_id,
        CancelCapacityCommitmentBody {
            idempotency_key: "commitment-primary-cancel".into(),
            expected_commitment_revision: 1,
            expected_commitment_digest: created.commitment.commitment_digest.clone(),
            reason: "provider withdrew before delivery".into(),
            confirm_cancel: true,
        },
    )
    .unwrap();
    assert_eq!(canceled.terminal_receipt.terminal_status, "canceled");
    assert_eq!(canceled.terminal_receipt.result_claim_state, "released");
    assert_eq!(fixture.balance(&fixture.token_bucket_id), (100, 0));
    assert_eq!(fixture.balance(&fixture.concurrency_bucket_id), (4, 0));

    let root = fixture.root.clone();
    let owner_id = fixture.owner_id.clone();
    let provider_id = fixture.provider_id.clone();
    let pool_id = fixture.pool_id.clone();
    let commitment_id = created.commitment.commitment_id;
    drop(fixture);
    let reopened = Store::open(&root.join("state.sqlite")).unwrap();
    let detail = capacity_commitment_service::get_for_owner(
        &reopened,
        &owner_id,
        &provider_id,
        &pool_id,
        &commitment_id,
    )
    .unwrap();
    assert_eq!(detail.current_status, "canceled");
    assert_eq!(detail.terminal_receipt.unwrap().terminal_revision, 2);
    drop(reopened);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn due_commitment_expires_once_and_restores_capacity() {
    let fixture = Fixture::new();
    let created = capacity_commitment_service::create_for_owner(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_body("commitment-expiry", true),
    )
    .unwrap();
    wait_until(&created.commitment.expires_at);

    let denied = capacity_commitment_service::expire_due_for_admin(
        &fixture.store,
        &fixture.admin_id,
        ExpireDueCapacityCommitmentsBody {
            limit: 20,
            confirm_expire_due: false,
        },
    )
    .unwrap_err();
    assert!(denied.to_string().contains("显式确认"));

    let report = capacity_commitment_service::expire_due_for_admin(
        &fixture.store,
        &fixture.admin_id,
        ExpireDueCapacityCommitmentsBody {
            limit: 20,
            confirm_expire_due: true,
        },
    )
    .unwrap();
    assert_eq!(report.selected_count, 1);
    assert_eq!(report.expired_count, 1);
    assert_eq!(report.failed_count, 0);
    assert_eq!(report.items[0].status, "expired");
    assert_eq!(fixture.balance(&fixture.token_bucket_id), (100, 0));
    assert_eq!(fixture.balance(&fixture.concurrency_bucket_id), (4, 0));

    let second = capacity_commitment_service::expire_due_for_admin(
        &fixture.store,
        &fixture.admin_id,
        ExpireDueCapacityCommitmentsBody {
            limit: 20,
            confirm_expire_due: true,
        },
    )
    .unwrap();
    assert_eq!(second.selected_count, 0);
    let detail = capacity_commitment_service::get_for_owner(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &created.commitment.commitment_id,
    )
    .unwrap();
    assert_eq!(detail.current_status, "expired");
    fixture.cleanup();
}

#[test]
fn capacity_instrument_replays_reject_drift_and_retirement_fences_only_fresh_work() {
    let fixture = Fixture::new();

    let registration = fixture
        .store
        .register_compute_capacity_instrument(fixture.instrument_registration_input())
        .unwrap();
    assert!(registration.replayed);
    assert_eq!(registration.instrument, fixture.capacity_instrument);
    let activation = fixture
        .store
        .activate_compute_capacity_instrument(fixture.instrument_activation_input())
        .unwrap();
    assert!(activation.replayed);
    let adoption = fixture
        .store
        .adopt_compute_capacity_instrument_offer(fixture.instrument_adoption_input())
        .unwrap();
    assert!(adoption.replayed);
    assert_eq!(adoption.adoption.offer_digest, fixture.offer.offer_digest);
    assert_eq!(
        adoption.adoption.publication_digest,
        fixture.publication.publication_digest
    );

    let mut drifted = fixture.instrument_registration_input();
    drifted.contract_units[0].quantity_units += drifted.contract_units[0].unit_size;
    let drift_error = fixture
        .store
        .register_compute_capacity_instrument(drifted)
        .unwrap_err();
    assert!(drift_error
        .to_string()
        .contains("idempotency key binds different input"));
    let mut drifted_adoption = fixture.instrument_adoption_input();
    drifted_adoption.expected_offer_digest = "0".repeat(64);
    let adoption_drift_error = fixture
        .store
        .adopt_compute_capacity_instrument_offer(drifted_adoption)
        .unwrap_err();
    assert!(adoption_drift_error
        .to_string()
        .contains("idempotency key binds different input"));

    let fresh_snapshot = fixture.fresh_price_snapshot("before-retirement");
    let snapshot_receipt = fixture
        .store
        .register_compute_price_snapshot(&fresh_snapshot)
        .unwrap();
    assert!(!snapshot_receipt.replayed);

    let mut fractional = fixture.create_body("fractional-standard-contract", true);
    fractional.quantities[0].quantity_units = 10;
    let fractional_error = capacity_commitment_service::create_for_owner(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fractional,
    )
    .unwrap_err();
    assert!(format!("{fractional_error:#}").contains("整数倍"));
    assert_eq!(fixture.balance(&fixture.token_bucket_id), (100, 0));

    let committed = capacity_commitment_service::create_for_owner(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_body("retirement-preserved-terminal", true),
    )
    .unwrap();
    fixture.retire_instrument("fence-fresh-work");
    let retired_adoption_replay = fixture
        .store
        .adopt_compute_capacity_instrument_offer(fixture.instrument_adoption_input())
        .unwrap();
    assert!(retired_adoption_replay.replayed);
    assert_eq!(
        retired_adoption_replay.adoption.adoption_receipt_id,
        adoption.adoption.adoption_receipt_id
    );
    let currentness = fixture
        .store
        .compute_capacity_instrument_currentness(&fixture.capacity_instrument.instrument_id)
        .unwrap()
        .unwrap();
    assert_eq!(currentness.current_status, "retired");

    let blocked_snapshot = fixture.fresh_price_snapshot("after-retirement");
    let snapshot_error = fixture
        .store
        .register_compute_price_snapshot(&blocked_snapshot)
        .unwrap_err();
    assert!(snapshot_error.to_string().contains("not current active"));
    let commitment_error = capacity_commitment_service::create_for_owner(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_body("blocked-after-retirement", true),
    )
    .unwrap_err();
    assert!(format!("{commitment_error:#}").contains("not current active"));

    let canceled = capacity_commitment_service::cancel_for_owner(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &committed.commitment.commitment_id,
        CancelCapacityCommitmentBody {
            idempotency_key: "terminal-after-instrument-retirement".into(),
            expected_commitment_revision: committed.commitment.commitment_revision,
            expected_commitment_digest: committed.commitment.commitment_digest,
            reason: "instrument retirement must not lock historical cancellation".into(),
            confirm_cancel: true,
        },
    )
    .unwrap();
    assert_eq!(canceled.terminal_receipt.terminal_status, "canceled");
    assert_eq!(fixture.balance(&fixture.token_bucket_id), (100, 0));
    assert_eq!(fixture.balance(&fixture.concurrency_bucket_id), (4, 0));
    fixture.cleanup();
}

fn assert_table_count(store: &Store, table: &str, expected: i64) {
    let count: i64 = store
        .conn()
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, expected, "unexpected row count in {table}");
}
