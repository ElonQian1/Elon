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
