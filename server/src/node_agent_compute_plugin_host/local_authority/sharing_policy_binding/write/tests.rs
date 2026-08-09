use rusqlite::{Connection, TransactionBehavior};

use super::super::{
    revocation::{
        insert_prepared_revocation, prepare_revocation, read_exact_revocation,
        validate_terminalized_work,
    },
    test_support,
};
use super::{insert_receipt, read_exact_receipt};
use crate::node_agent_compute_plugin_host::local_authority::sharing_policy_binding::types::ProjectedSharingPolicyBinding;

fn projected_binding() -> ProjectedSharingPolicyBinding {
    test_support::projected_binding(
        test_support::request(4),
        test_support::authority_state(),
        &test_support::bound_at(),
    )
}

fn prepared_work_states(connection: &Connection) -> (String, String) {
    connection
        .query_row(
            r#"SELECT
                (SELECT state FROM fetch_claims WHERE claim_id = 'claim_prepared'),
                (SELECT state FROM candidate_verification_runs
                 WHERE verification_id = 'verification_prepared')"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
}

#[test]
fn revocation_and_binding_commit_atomically_and_advance_authority() {
    let projected = projected_binding();
    let mut connection = test_support::connection(&projected.before);

    let prepared = test_support::commit_transition(&mut connection, &projected);
    assert_eq!(prepared.hashed_receipt.receipt().work_item_count, 0);

    let receipt = &projected.hashed_receipt.receipt;
    let authority: (i64, i64, String, i64, i64, i64) = connection
        .query_row(
            r#"SELECT state_revision, inventory_revision, inventory_digest,
                desired_policy_revision, authority_epoch, trusted_time_high_water_ms
               FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        authority,
        (
            receipt.state_revision_after,
            receipt.inventory_revision_after,
            receipt.inventory_digest_after.clone(),
            receipt.policy_revision,
            receipt.authority_epoch_after,
            receipt.bound_at_ms,
        )
    );
    let counts: (i64, i64) = connection
        .query_row(
            r#"SELECT
                (SELECT COUNT(*) FROM sharing_policy_binding_revocation_receipts),
                (SELECT COUNT(*) FROM sharing_policy_binding_receipts)"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(counts, (1, 1));
}

#[test]
fn revocation_without_binding_cannot_commit_or_leave_an_orphan() {
    let projected = projected_binding();
    let mut connection = test_support::connection(&projected.before);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let prepared = prepare_revocation(&transaction, &projected).unwrap();
    insert_prepared_revocation(&transaction, &prepared).unwrap();
    assert!(
        read_exact_revocation(&transaction, &projected.request, &projected.hashed_receipt)
            .unwrap()
            .is_some()
    );

    let error = transaction.commit().unwrap_err();

    assert!(format!("{error:#}").contains("FOREIGN KEY"));
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sharing_policy_binding_revocation_receipts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn committed_revocation_receipt_is_immutable_and_append_only() {
    let projected = projected_binding();
    let mut connection = test_support::connection(&projected.before);
    let prepared = test_support::commit_transition(&mut connection, &projected);
    assert_eq!(prepared.hashed_receipt.receipt().work_item_count, 0);

    let update_error = connection
        .execute(
            r#"UPDATE sharing_policy_binding_revocation_receipts
               SET work_set_json = work_set_json WHERE policy_revision = 4"#,
            [],
        )
        .unwrap_err();
    assert!(format!("{update_error:#}").contains("policy prepared-work receipts are immutable"));

    let delete_error = connection
        .execute(
            "DELETE FROM sharing_policy_binding_revocation_receipts WHERE policy_revision = 4",
            [],
        )
        .unwrap_err();
    assert!(format!("{delete_error:#}").contains("policy prepared-work receipts are append-only"));
}

#[test]
fn prepared_fetch_and_verification_terminalize_with_binding() {
    let projected = projected_binding();
    let mut connection = test_support::connection(&projected.before);
    test_support::seed_prepared_work(&mut connection, &projected.before);
    assert_eq!(
        prepared_work_states(&connection),
        ("prepared".to_string(), "prepared".to_string())
    );
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let prepared = prepare_revocation(&transaction, &projected).unwrap();
    assert_eq!(prepared.hashed_receipt.receipt().fetch_claim_count, 1);
    assert_eq!(prepared.hashed_receipt.receipt().verification_count, 1);
    assert_eq!(prepared.hashed_receipt.receipt().work_item_count, 2);
    insert_prepared_revocation(&transaction, &prepared).unwrap();
    insert_receipt(&transaction, &projected).unwrap();

    let binding = read_exact_receipt(&transaction, &projected.request)
        .unwrap()
        .unwrap();
    let revocation = read_exact_revocation(&transaction, &projected.request, &binding)
        .unwrap()
        .unwrap();
    assert_eq!(revocation, prepared);
    validate_terminalized_work(&transaction, &revocation).unwrap();
    transaction.commit().unwrap();

    assert_eq!(
        prepared_work_states(&connection),
        ("aborted".to_string(), "aborted".to_string())
    );
    let terminal: (i64, String, i64, String) = connection
        .query_row(
            r#"SELECT
                (SELECT resolved_at_ms FROM fetch_claims WHERE claim_id = 'claim_prepared'),
                (SELECT resolution_reason FROM fetch_claims WHERE claim_id = 'claim_prepared'),
                (SELECT resolved_at_ms FROM candidate_verification_runs
                 WHERE verification_id = 'verification_prepared'),
                (SELECT resolution_reason FROM candidate_verification_runs
                 WHERE verification_id = 'verification_prepared')"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        terminal,
        (
            projected.hashed_receipt.receipt.bound_at_ms,
            "sharing_policy_transition_aborted".to_string(),
            projected.hashed_receipt.receipt.bound_at_ms,
            "verification_aborted".to_string(),
        )
    );
}

#[test]
fn orphan_revocation_rollback_preserves_prepared_work() {
    let projected = projected_binding();
    let mut connection = test_support::connection(&projected.before);
    test_support::seed_prepared_work(&mut connection, &projected.before);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let prepared = prepare_revocation(&transaction, &projected).unwrap();
    assert_eq!(prepared.hashed_receipt.receipt().work_item_count, 2);
    insert_prepared_revocation(&transaction, &prepared).unwrap();

    let error = transaction.commit().unwrap_err();

    assert!(format!("{error:#}").contains("FOREIGN KEY"));
    assert_eq!(
        prepared_work_states(&connection),
        ("prepared".to_string(), "prepared".to_string())
    );
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sharing_policy_binding_revocation_receipts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}
