use crate::{
    open_commerce_portability_reauthorization_model::{
        CreatePortabilityReauthorizationRequest, CreatePortabilityRelationshipMappingRequest,
        PORTABILITY_RELATIONSHIP_MAPPING_SCHEMA,
    },
    open_commerce_relationship_model::RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
};

#[path = "open_commerce_portability_reauthorization_test_support.rs"]
mod support;
use support::*;

#[test]
fn mapping_requires_explicit_confirmation_and_is_owner_scoped_and_idempotent() {
    let fixture = fixture();
    let owner = fixture.owner_actor();
    let mut unconfirmed = mapping_request(&fixture);
    unconfirmed.confirmed_by_user = false;
    assert!(super::create_mapping(
        &fixture.state.store,
        &fixture.destination_project_id,
        &owner,
        unconfirmed,
    )
    .unwrap_err()
    .to_string()
    .contains("明确确认"));

    let mapping = super::create_mapping(
        &fixture.state.store,
        &fixture.destination_project_id,
        &owner,
        mapping_request(&fixture),
    )
    .unwrap();
    assert_eq!(mapping.schema, PORTABILITY_RELATIONSHIP_MAPPING_SCHEMA);
    assert_eq!(mapping.status, "active");
    assert_eq!(mapping.identity_match_status, "not_verified");
    assert!(mapping.identity_match_key_id.is_none());

    let retry = super::create_mapping(
        &fixture.state.store,
        &fixture.destination_project_id,
        &owner,
        mapping_request(&fixture),
    )
    .unwrap();
    assert_eq!(retry.id, mapping.id);
    assert_eq!(
        fixture.count("open_commerce_portability_relationship_mappings"),
        1
    );
    assert_eq!(
        fixture
            .audit_metadata("consumer_portability.relationship_mapping_created")
            .len(),
        1
    );

    let conflicting = CreatePortabilityRelationshipMappingRequest {
        target_merchant_id: fixture.alternate_target_merchant_id.clone(),
        ..mapping_request(&fixture)
    };
    assert!(super::create_mapping(
        &fixture.state.store,
        &fixture.destination_project_id,
        &owner,
        conflicting,
    )
    .unwrap_err()
    .to_string()
    .contains("先撤销"));

    assert!(super::list_mappings(
        &fixture.state.store,
        &fixture.destination_project_id,
        &fixture.member_actor(),
        100,
    )
    .unwrap()
    .is_empty());
    let owner_mappings = super::list_mappings(
        &fixture.state.store,
        &fixture.destination_project_id,
        &owner,
        100,
    )
    .unwrap();
    assert_eq!(owner_mappings.len(), 1);
    assert_eq!(owner_mappings[0].id, mapping.id);
}

#[test]
fn reauthorization_only_creates_a_fresh_narrow_pending_request() {
    let fixture = fixture();
    let owner = fixture.owner_actor();
    let mapping = super::create_mapping(
        &fixture.state.store,
        &fixture.destination_project_id,
        &owner,
        mapping_request(&fixture),
    )
    .unwrap();
    let relationship_count = fixture.count("open_commerce_consumer_relationships");

    let mut unconfirmed = reauthorization_request(&fixture);
    unconfirmed.confirmed_by_user = false;
    assert!(super::create_reauthorization(
        &fixture.state.store,
        &fixture.destination_project_id,
        &mapping.id,
        &owner,
        unconfirmed,
    )
    .unwrap_err()
    .to_string()
    .contains("明确确认"));

    let broad = CreatePortabilityReauthorizationRequest {
        scopes: vec!["order.create".to_string()],
        ..reauthorization_request(&fixture)
    };
    assert!(super::create_reauthorization(
        &fixture.state.store,
        &fixture.destination_project_id,
        &mapping.id,
        &owner,
        broad,
    )
    .unwrap_err()
    .to_string()
    .contains("不能超出"));

    let wrong_app = CreatePortabilityReauthorizationRequest {
        requester_app_id: fixture.outsider_app_id.clone(),
        ..reauthorization_request(&fixture)
    };
    assert!(super::create_reauthorization(
        &fixture.state.store,
        &fixture.destination_project_id,
        &mapping.id,
        &owner,
        wrong_app,
    )
    .unwrap_err()
    .to_string()
    .contains("不能代表"));

    fixture.set_target_published(false);
    assert!(super::create_reauthorization(
        &fixture.state.store,
        &fixture.destination_project_id,
        &mapping.id,
        &owner,
        reauthorization_request(&fixture),
    )
    .unwrap_err()
    .to_string()
    .contains("未发布"));
    fixture.set_target_published(true);

    let result = super::create_reauthorization(
        &fixture.state.store,
        &fixture.destination_project_id,
        &mapping.id,
        &owner,
        reauthorization_request(&fixture),
    )
    .unwrap();
    assert!(!result.old_grant_restored);
    assert_eq!(result.authorization_request.status, "pending");
    assert_eq!(
        result.authorization_request.merchant_id,
        fixture.target_merchant_id
    );
    assert_eq!(
        result.authorization_request.requester_app_id,
        fixture.app_id
    );
    assert_eq!(
        result.authorization_request.scopes,
        vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()]
    );
    assert_eq!(fixture.count("open_commerce_authorization_requests"), 1);
    assert_eq!(fixture.count("open_commerce_grants"), 0);
    assert_eq!(
        fixture.count("open_commerce_consumer_relationships"),
        relationship_count
    );

    let retry = super::create_reauthorization(
        &fixture.state.store,
        &fixture.destination_project_id,
        &mapping.id,
        &owner,
        reauthorization_request(&fixture),
    )
    .unwrap();
    assert_eq!(
        retry.authorization_request.id,
        result.authorization_request.id
    );
    assert_eq!(fixture.count("open_commerce_authorization_requests"), 1);
    assert_eq!(fixture.count("open_commerce_grants"), 0);

    let audits = fixture.audit_metadata("consumer_portability.reauthorization_requested");
    assert!(audits
        .iter()
        .all(|metadata| metadata["old_grant_restored"] == false));
    assert!(audits
        .iter()
        .all(|metadata| metadata["target_merchant_id"] == fixture.target_merchant_id));

    let revoked = super::revoke_mapping(
        &fixture.state.store,
        &fixture.destination_project_id,
        &mapping.id,
        &owner,
    )
    .unwrap();
    assert_eq!(revoked.status, "revoked");
    assert!(super::create_reauthorization(
        &fixture.state.store,
        &fixture.destination_project_id,
        &mapping.id,
        &owner,
        reauthorization_request(&fixture),
    )
    .unwrap_err()
    .to_string()
    .contains("已撤销"));
}
