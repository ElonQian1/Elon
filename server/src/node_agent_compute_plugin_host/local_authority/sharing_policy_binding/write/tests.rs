use rusqlite::{params, Connection, TransactionBehavior};

use super::super::{
    revocation::{
        insert_prepared_revocation, prepare_revocation, read_exact_revocation,
        validate_terminalized_work,
    },
    test_support,
    validation::project,
};
use super::{insert_receipt, read_exact_receipt};
use crate::node_agent_compute_plugin_host::{
    local_authority::sharing_policy_binding::types::{
        PolicyBindingAuthorityState, ProjectedSharingPolicyBinding,
    },
    local_authority_schema::ensure_schema,
};

fn projected_binding() -> ProjectedSharingPolicyBinding {
    project(
        test_support::request(4),
        test_support::authority_state(),
        &test_support::bound_at(),
    )
    .unwrap()
}

fn connection(before: &PolicyBindingAuthorityState) -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .unwrap();
    connection
        .pragma_update(None, "trusted_schema", "OFF")
        .unwrap();
    ensure_schema(&mut connection).unwrap();
    connection
        .execute(
            r#"INSERT INTO authority_meta (
                singleton, schema_version, installation_id_digest,
                state_revision, inventory_revision, inventory_digest, inventory_json,
                desired_policy_revision, sharing_enabled,
                sharing_authorization_ref, sharing_authorization_revision,
                sharing_authorization_digest, node_profile_digest,
                manifest_catalog_revision, target_id, host_api_protocol_id,
                host_api_revision, active_bundle_revision,
                publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest,
                authority_epoch, process_owner_epoch,
                trusted_time_high_water_ms, clock_status, updated_at_ms
            ) VALUES (
                1, 3, ?1,
                ?2, ?3, ?4, ?5,
                ?6, ?7,
                ?8, ?9,
                ?10, ?11,
                0, 'windows_x86_64', 'elon_compute_plugin_host',
                1, NULL,
                NULL, NULL,
                NULL, NULL,
                ?12, ?13,
                ?14, 'trusted', ?15
            )"#,
            params![
                &before.installation_id_digest,
                before.state_revision,
                before.inventory_revision,
                &before.inventory_digest,
                &before.inventory_json,
                before.desired_policy_revision,
                i64::from(before.sharing_enabled),
                before.sharing_authorization_ref.as_deref(),
                before.sharing_authorization_revision,
                before.sharing_authorization_digest.as_deref(),
                "f".repeat(64),
                before.authority_epoch,
                before.process_owner_epoch,
                before.trusted_time_high_water_ms,
                before.updated_at_ms,
            ],
        )
        .unwrap();
    connection
}

fn commit_transition(connection: &mut Connection, projected: &ProjectedSharingPolicyBinding) {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let prepared = prepare_revocation(&transaction, projected).unwrap();
    assert_eq!(prepared.hashed_receipt.receipt().work_item_count, 0);
    insert_prepared_revocation(&transaction, &prepared).unwrap();
    insert_receipt(&transaction, projected).unwrap();

    let binding = read_exact_receipt(&transaction, &projected.request)
        .unwrap()
        .unwrap();
    assert_eq!(binding, projected.hashed_receipt);
    let revocation = read_exact_revocation(&transaction, &projected.request, &binding)
        .unwrap()
        .unwrap();
    assert_eq!(revocation, prepared);
    validate_terminalized_work(&transaction, &revocation).unwrap();
    transaction.commit().unwrap();
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
    let mut connection = connection(&projected.before);

    commit_transition(&mut connection, &projected);

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
    let mut connection = connection(&projected.before);
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
    let mut connection = connection(&projected.before);
    commit_transition(&mut connection, &projected);

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
    let mut connection = connection(&projected.before);
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
    let mut connection = connection(&projected.before);
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
