use rusqlite::TransactionBehavior;

use super::insert_receipt;
use crate::node_agent_compute_plugin_host::local_authority::manifest_catalog_binding::{
    test_support as fixture, validation::project,
};

#[test]
fn catalog_binding_commits_atomically_and_advances_authority() {
    let projected = fixture::projected();
    let mut connection = fixture::connection(&projected.before);
    fixture::commit_transition(&mut connection, &projected);
    let receipt = &projected.hashed_receipt.receipt;

    let authority: (i64, i64, i64, i64) = connection
        .query_row(
            r#"SELECT state_revision, manifest_catalog_revision,
                authority_epoch, trusted_time_high_water_ms
               FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        authority,
        (
            receipt.state_revision_after,
            receipt.catalog_revision,
            receipt.authority_epoch_after,
            receipt.bound_at_ms,
        )
    );
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM manifest_catalog_binding_receipts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn stale_authority_fence_rejects_binding_without_residue() {
    let current = fixture::projected();
    let mut stale_before = current.before.clone();
    stale_before.state_revision -= 1;
    let stale = project(
        current.request.clone(),
        stale_before,
        fixture::bound_at_ms(),
    )
    .unwrap();
    let mut connection = fixture::connection(&current.before);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();

    let error = insert_receipt(&transaction, &stale).unwrap_err();
    assert!(
        format!("{error:#}").contains("manifest catalog binding lost its exact authority fence")
    );
    transaction.rollback().unwrap();

    let state: (i64, i64, i64) = connection
        .query_row(
            r#"SELECT state_revision, manifest_catalog_revision, authority_epoch
               FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        state,
        (
            current.before.state_revision,
            current.before.manifest_catalog_revision,
            current.before.authority_epoch,
        )
    );
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM manifest_catalog_binding_receipts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn committed_catalog_receipt_is_immutable_and_append_only() {
    let projected = fixture::projected();
    let mut connection = fixture::connection(&projected.before);
    fixture::commit_transition(&mut connection, &projected);

    let update_error = connection
        .execute(
            r#"UPDATE manifest_catalog_binding_receipts
               SET receipt_json = receipt_json WHERE catalog_revision = 4"#,
            [],
        )
        .unwrap_err();
    assert!(format!("{update_error:#}").contains("manifest catalog binding receipts are immutable"));

    let delete_error = connection
        .execute(
            "DELETE FROM manifest_catalog_binding_receipts WHERE catalog_revision = 4",
            [],
        )
        .unwrap_err();
    assert!(
        format!("{delete_error:#}").contains("manifest catalog binding receipts are append-only")
    );
}
