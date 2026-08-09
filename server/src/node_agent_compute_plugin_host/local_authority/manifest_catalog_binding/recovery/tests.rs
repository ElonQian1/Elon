use std::time::Instant;

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::TransactionBehavior;

use super::{
    exact_prestate, validate_committed_successor, validate_not_created_successor,
    validate_receipt_absence, validate_recovery_state_floor,
};
use crate::node_agent_compute_plugin_host::local_authority::{
    manifest_catalog_binding::{
        test_support,
        types::{ComputePluginManifestCatalogBindingRecoveryKey, ProjectedManifestCatalogBinding},
    },
    ComputePluginLocalAuthority,
};

fn at(timestamp_ms: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(timestamp_ms).single().unwrap()
}

fn recovery_key(
    projected: &ProjectedManifestCatalogBinding,
) -> ComputePluginManifestCatalogBindingRecoveryKey {
    let authority = ComputePluginLocalAuthority::new("unused");
    ComputePluginManifestCatalogBindingRecoveryKey {
        authority_instance_binding: authority.instance_binding().clone(),
        root_identity_digest: "4".repeat(64),
        clock_epoch_digest: "5".repeat(64),
        prepared_at: Instant::now(),
        request: projected.request.clone(),
        before: projected.before.clone(),
        hashed_receipt: projected.hashed_receipt.clone(),
    }
}

#[test]
fn exact_prestate_requires_every_catalog_authority_field() {
    let before = test_support::authority_state();
    assert!(exact_prestate(&before, &before));

    let mut changed = before.clone();
    changed.node_profile_digest = "6".repeat(64);

    assert!(!exact_prestate(&changed, &before));
}

#[test]
fn receipt_absence_rejects_a_committed_identity_collision() {
    let projected = test_support::projected();
    let key = recovery_key(&projected);
    let mut connection = test_support::connection(&projected.before);
    {
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .unwrap();
        validate_receipt_absence(&transaction, &key).unwrap();
    }
    test_support::commit_transition(&mut connection, &projected);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    let error = validate_receipt_absence(&transaction, &key).unwrap_err();

    assert!(format!("{error:#}").contains("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_COLLISION"));
}

#[test]
fn committed_binding_remains_valid_after_a_legal_successor() {
    let projected_4 = test_support::projected();
    let key_4 = recovery_key(&projected_4);
    let mut connection = test_support::connection(&projected_4.before);
    test_support::commit_transition(&mut connection, &projected_4);
    let current_4 = test_support::read_current_state(
        &mut connection,
        &at(projected_4.hashed_receipt.receipt.bound_at_ms),
    );
    {
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .unwrap();
        validate_recovery_state_floor(&current_4, &key_4).unwrap();
        validate_committed_successor(&transaction, &current_4, &key_4.hashed_receipt).unwrap();
    }

    let bound_5_ms = projected_4.hashed_receipt.receipt.bound_at_ms + 60_000;
    let projected_5 = test_support::projected_revision(5, current_4, bound_5_ms);
    test_support::commit_transition(&mut connection, &projected_5);
    let current_5 = test_support::read_current_state(&mut connection, &at(bound_5_ms));
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    validate_recovery_state_floor(&current_5, &key_4).unwrap();
    validate_committed_successor(&transaction, &current_5, &key_4.hashed_receipt).unwrap();
}

#[test]
fn absent_binding_is_superseded_by_a_legal_catalog_head() {
    let projected_4 = test_support::projected();
    let key_4 = recovery_key(&projected_4);
    let bound_5_ms = projected_4.hashed_receipt.receipt.bound_at_ms + 60_000;
    let projected_5 =
        test_support::projected_revision(5, test_support::authority_state(), bound_5_ms);
    let mut connection = test_support::connection(&projected_5.before);
    test_support::commit_transition(&mut connection, &projected_5);
    let current = test_support::read_current_state(&mut connection, &at(bound_5_ms));
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    validate_not_created_successor(&transaction, &current, &key_4).unwrap();
}

#[test]
fn unchanged_prestate_cannot_prove_a_successor() {
    let projected = test_support::projected();
    let key = recovery_key(&projected);
    let mut connection = test_support::connection(&projected.before);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    let error = validate_not_created_successor(&transaction, &projected.before, &key).unwrap_err();

    assert!(format!("{error:#}")
        .contains("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_SUCCESSOR_UNPROVEN"));
}

#[test]
fn recovery_state_floor_rejects_precommit_authority() {
    let projected = test_support::projected();
    let key = recovery_key(&projected);

    let error = validate_recovery_state_floor(&projected.before, &key).unwrap_err();

    assert!(
        format!("{error:#}").contains("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_STATE_ROLLBACK")
    );
}

#[test]
fn advanced_catalog_head_requires_its_exact_receipt() {
    let projected = test_support::projected();
    let key = recovery_key(&projected);
    let mut current = projected.before.clone();
    current.manifest_catalog_revision = projected.request.catalog_revision + 1;
    current.state_revision = projected.hashed_receipt.receipt.state_revision_after + 1;
    current.authority_epoch = projected.hashed_receipt.receipt.authority_epoch_after + 1;
    current.trusted_time_high_water_ms = projected.hashed_receipt.receipt.bound_at_ms + 1;
    current.updated_at_ms = current.trusted_time_high_water_ms;
    let mut connection = test_support::connection(&projected.before);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();

    let error =
        validate_committed_successor(&transaction, &current, &key.hashed_receipt).unwrap_err();

    assert!(format!("{error:#}").contains("COMPUTE_PLUGIN_MANIFEST_CATALOG_HEAD_RECEIPT_MISSING"));
}
