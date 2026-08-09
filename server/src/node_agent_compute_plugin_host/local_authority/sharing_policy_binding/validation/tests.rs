use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;

use super::{project, validate_hashed_receipt};

mod fixtures;

#[test]
fn projection_advances_all_fences_and_revokes_existing_plugin_admission() {
    let before = fixtures::authority_state();
    let original = before.inventory.plugins[0].clone();
    let bound_at = fixtures::bound_at();

    let projected = project(fixtures::request(4), before, &bound_at).unwrap();
    validate_hashed_receipt(&projected.hashed_receipt).unwrap();

    let receipt = &projected.hashed_receipt.receipt;
    assert_eq!(
        receipt.state_revision_after,
        receipt.state_revision_before + 1
    );
    assert_eq!(
        receipt.inventory_revision_after,
        receipt.inventory_revision_before + 1
    );
    assert_eq!(
        receipt.authority_epoch_after,
        receipt.authority_epoch_before + 1
    );
    assert_eq!(receipt.policy_revision, 4);
    assert!(receipt.sharing_enabled);
    assert_eq!(receipt.sharing_authorization_revision, Some(4));
    assert_eq!(
        receipt.sharing_authorization_digest.as_deref(),
        Some(receipt.policy_digest.as_str())
    );

    let inventory = fixtures::decode_inventory(&projected.inventory_after_json);
    assert_eq!(
        inventory.inventory_revision,
        receipt.inventory_revision_after
    );
    assert_eq!(inventory.desired_policy_revision, 4);
    assert!(inventory.sharing_enabled);
    assert_eq!(
        jcs_sha256_hex(&inventory).unwrap(),
        receipt.inventory_digest_after
    );

    let record = &inventory.plugins[0];
    assert_eq!(record.desired_activation, "disabled");
    assert_eq!(record.admission, "revoked");
    assert_eq!(record.health, None);
    assert_eq!(record.state_changed_at, "2026-08-07T00:02:00.000Z");
    assert_eq!(record.plugin_id, original.plugin_id);
    assert_eq!(record.last_plan_id, original.last_plan_id);
    assert_eq!(record.install_generation, original.install_generation);
    assert_eq!(record.activation_generation, original.activation_generation);
    assert_eq!(record.active_slot_ref, original.active_slot_ref);
    assert_eq!(record.candidate_slot_ref, original.candidate_slot_ref);
    assert_eq!(record.slots, original.slots);
    assert_eq!(record.desired_presence, original.desired_presence);
    assert_eq!(record.runtime, original.runtime);
    assert_eq!(
        record.permission_grant_digest,
        original.permission_grant_digest
    );
    assert_eq!(record.active_attempts, original.active_attempts);
    assert_eq!(record.last_error, original.last_error);
}

#[test]
fn projection_rejects_stale_revision_and_non_advancing_trusted_time() {
    let stale_revision = project(
        fixtures::request(3),
        fixtures::authority_state(),
        &fixtures::bound_at(),
    )
    .unwrap_err();
    assert!(
        format!("{stale_revision:#}").contains("COMPUTE_PLUGIN_POLICY_BINDING_AUTHORITY_CHANGED")
    );

    let stale_time = project(
        fixtures::request(4),
        fixtures::authority_state(),
        &fixtures::trusted_high_water(),
    )
    .unwrap_err();
    assert!(format!("{stale_time:#}").contains("COMPUTE_PLUGIN_POLICY_BINDING_AUTHORITY_CHANGED"));
}

#[test]
fn disabled_projection_requires_no_authorization_or_preparation() {
    let projected = project(
        fixtures::disabled_request(4),
        fixtures::authority_state(),
        &fixtures::bound_at(),
    )
    .unwrap();
    validate_hashed_receipt(&projected.hashed_receipt).unwrap();

    let receipt = &projected.hashed_receipt.receipt;
    assert!(!receipt.sharing_enabled);
    assert_eq!(receipt.sharing_authorization_ref, None);
    assert_eq!(receipt.sharing_authorization_revision, None);
    assert_eq!(receipt.sharing_authorization_digest, None);
    assert_eq!(receipt.source_preparation_id, None);

    let inventory = fixtures::decode_inventory(&projected.inventory_after_json);
    assert!(!inventory.sharing_enabled);
    assert_eq!(inventory.plugins[0].desired_activation, "disabled");
    assert_eq!(inventory.plugins[0].admission, "revoked");
}

#[test]
fn receipt_validation_rejects_rehashed_fence_and_authorization_tampering() {
    let projected = project(
        fixtures::request(4),
        fixtures::authority_state(),
        &fixtures::bound_at(),
    )
    .unwrap();

    let mut fence_tamper = projected.hashed_receipt.clone();
    fence_tamper.receipt.state_revision_after += 1;
    fence_tamper.receipt_digest = jcs_sha256_hex(&fence_tamper.receipt).unwrap();
    let fence_error = validate_hashed_receipt(&fence_tamper).unwrap_err();
    assert!(format!("{fence_error:#}").contains("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_INVALID"));

    let mut authorization_tamper = projected.hashed_receipt;
    authorization_tamper.receipt.sharing_authorization_ref = None;
    authorization_tamper.receipt_digest = jcs_sha256_hex(&authorization_tamper.receipt).unwrap();
    let authorization_error = validate_hashed_receipt(&authorization_tamper).unwrap_err();
    assert!(format!("{authorization_error:#}")
        .contains("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_INVALID"));
}
