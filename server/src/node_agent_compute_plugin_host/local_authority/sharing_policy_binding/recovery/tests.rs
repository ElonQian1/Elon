use std::time::Instant;

use chrono::Duration;
use rusqlite::{Connection, TransactionBehavior};

use super::{
    not_created_prestate_matches, validate_committed_history, validate_not_created_successor,
};
use crate::node_agent_compute_plugin_host::local_authority::{
    sharing_policy_binding::{
        revocation::prepare_revocation,
        test_support,
        types::{ComputePluginSharingPolicyBindingRecoveryKey, ProjectedSharingPolicyBinding},
    },
    ComputePluginLocalAuthority,
};

fn projected(policy_revision: i64) -> ProjectedSharingPolicyBinding {
    test_support::projected_binding(
        test_support::request(policy_revision),
        test_support::authority_state(),
        &test_support::bound_at(),
    )
}

fn recovery_key(
    connection: &mut Connection,
    projected: &ProjectedSharingPolicyBinding,
) -> ComputePluginSharingPolicyBindingRecoveryKey {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();
    let prepared_revocation = prepare_revocation(&transaction, projected).unwrap();
    transaction.rollback().unwrap();
    let authority = ComputePluginLocalAuthority::new("unused");
    ComputePluginSharingPolicyBindingRecoveryKey {
        authority_instance_binding: authority.instance_binding().clone(),
        root_identity_digest: "c".repeat(64),
        clock_epoch_digest: "d".repeat(64),
        prepared_at: Instant::now(),
        request: projected.request.clone(),
        before: projected.before.clone(),
        inventory_after_json: projected.inventory_after_json.clone(),
        hashed_receipt: projected.hashed_receipt.clone(),
        prepared_revocation,
    }
}

#[test]
fn exact_prestate_without_receipts_is_classified_not_created() {
    let projected = projected(4);
    let mut connection = test_support::connection(&projected.before);
    let key = recovery_key(&mut connection, &projected);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    assert!(not_created_prestate_matches(&transaction, &key).unwrap());
}

#[test]
fn committed_history_revalidates_terminalized_prepared_work() {
    let projected = projected(4);
    let mut connection = test_support::connection(&projected.before);
    test_support::seed_prepared_work(&mut connection, &projected.before);
    let key = recovery_key(&mut connection, &projected);
    assert_eq!(
        key.prepared_revocation
            .hashed_receipt
            .receipt()
            .work_item_count,
        2
    );
    let committed = test_support::commit_transition(&mut connection, &projected);
    assert_eq!(committed, key.prepared_revocation);
    let trusted_now = test_support::bound_at() + Duration::minutes(1);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    validate_committed_history(
        &transaction,
        &key,
        &trusted_now,
        projected.before.process_owner_epoch,
    )
    .unwrap();
}

#[test]
fn committed_binding_remains_valid_history_after_a_legal_successor() {
    let projected_4 = projected(4);
    let mut connection = test_support::connection(&projected_4.before);
    let key_4 = recovery_key(&mut connection, &projected_4);
    test_support::commit_transition(&mut connection, &projected_4);

    let bound_5 = test_support::bound_at() + Duration::minutes(2);
    let current = test_support::read_current_state(
        &mut connection,
        &(test_support::bound_at() + Duration::minutes(1)),
    );
    let projected_5 = test_support::projected_binding(test_support::request(5), current, &bound_5);
    test_support::commit_transition(&mut connection, &projected_5);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    validate_committed_history(
        &transaction,
        &key_4,
        &(bound_5 + Duration::minutes(1)),
        projected_4.before.process_owner_epoch,
    )
    .unwrap();
}

#[test]
fn absent_binding_is_classified_superseded_by_a_legal_successor() {
    let projected_4 = projected(4);
    let mut connection = test_support::connection(&projected_4.before);
    let key_4 = recovery_key(&mut connection, &projected_4);
    let bound_5 = test_support::bound_at() + Duration::minutes(2);
    let projected_5 = test_support::projected_binding(
        test_support::request(5),
        test_support::authority_state(),
        &bound_5,
    );
    test_support::commit_transition(&mut connection, &projected_5);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    validate_not_created_successor(
        &transaction,
        &key_4,
        &(bound_5 + Duration::minutes(1)),
        projected_4.before.process_owner_epoch,
    )
    .unwrap();
}

#[test]
fn unchanged_prestate_cannot_prove_a_missing_successor() {
    let projected = projected(4);
    let mut connection = test_support::connection(&projected.before);
    let key = recovery_key(&mut connection, &projected);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    let error = validate_not_created_successor(
        &transaction,
        &key,
        &(test_support::bound_at() + Duration::minutes(1)),
        projected.before.process_owner_epoch,
    )
    .unwrap_err();

    assert!(format!("{error:#}")
        .contains("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_SUCCESSOR_HEAD_CHANGED"));
}

#[test]
fn committed_history_rejects_the_wrong_process_owner_epoch() {
    let projected = projected(4);
    let mut connection = test_support::connection(&projected.before);
    let key = recovery_key(&mut connection, &projected);
    test_support::commit_transition(&mut connection, &projected);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    let error = validate_committed_history(
        &transaction,
        &key,
        &(test_support::bound_at() + Duration::minutes(1)),
        projected.before.process_owner_epoch + 1,
    )
    .unwrap_err();

    assert!(format!("{error:#}")
        .contains("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_HEAD_CHANGED"));
}
