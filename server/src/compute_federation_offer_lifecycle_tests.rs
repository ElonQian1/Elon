use crate::{
    compute_federation_offer_lifecycle_model::{
        DrainComputeOfferRequest, TerminateComputeOfferRequest,
    },
    compute_federation_offer_lifecycle_service,
    compute_federation_offer_publication_model::PublishComputeOfferDraftRequest,
    compute_federation_offer_publication_service, compute_federation_offer_service,
};

use super::control_plane_tests::Fixture;

#[test]
fn revised_offer_publishes_drains_and_revokes_with_auditable_receipts() {
    let fixture = Fixture::new();
    fixture.seed_active_supply();

    let created = compute_federation_offer_service::create_draft_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_request("offer-primary", 100, 4),
    )
    .unwrap();
    assert_eq!(created.offer.status, "draft");
    assert_eq!(created.offer.offer_version, 1);
    assert_eq!(created.market_effect, "none");

    let replayed = compute_federation_offer_service::create_draft_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_request("offer-primary", 100, 4),
    )
    .unwrap();
    assert!(replayed.replayed);
    assert_eq!(replayed.offer.offer_id, created.offer.offer_id);

    let revised = compute_federation_offer_service::revise_draft_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &created.offer.offer_id,
        fixture.revise_request(&created.offer.offer_digest, 1, 80, 3),
    )
    .unwrap();
    assert_eq!(revised.offer.status, "draft");
    assert_eq!(revised.offer.offer_version, 2);
    assert_eq!(revised.offer.authorization.policy_revision, 2);

    let published = compute_federation_offer_publication_service::publish_for_review(
        &fixture.store,
        &fixture.admin_id,
        &revised.offer.offer_id,
        PublishComputeOfferDraftRequest {
            expected_offer_version: revised.offer.offer_version,
            expected_offer_digest: revised.offer.offer_digest.clone(),
            idempotency_key: "publish-primary".into(),
            confirm_publish: true,
        },
    )
    .unwrap();
    assert_eq!(published.active_offer_version, 3);
    assert_eq!(published.offer_effect, "active");
    assert_eq!(published.price_snapshot_effect, "none");
    assert_eq!(published.capacity_effect, "none");
    assert_eq!(published.funds_effect, "none");

    let active = compute_federation_offer_service::get_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &revised.offer.offer_id,
    )
    .unwrap();
    assert_eq!(active.offer.status, "active");
    assert_eq!(active.offer.offer_version, 3);

    let drained = compute_federation_offer_lifecycle_service::drain_for_review(
        &fixture.store,
        &fixture.admin_id,
        &active.offer.offer_id,
        DrainComputeOfferRequest {
            expected_offer_version: active.offer.offer_version,
            expected_offer_digest: active.offer.offer_digest.clone(),
            reason: "planned capacity rotation".into(),
            idempotency_key: "drain-primary".into(),
            confirm_drain: true,
        },
    )
    .unwrap();
    assert_eq!(drained.previous_status, "active");
    assert_eq!(drained.target_status, "draining");
    assert_eq!(drained.target_offer_version, 4);
    assert_eq!(drained.quote_candidate_effect, "excluded_from_new_quotes");
    assert_eq!(drained.reservation_effect, "preserved");
    assert_eq!(drained.funds_effect, "none");

    let early_expiration = compute_federation_offer_lifecycle_service::expire_for_review(
        &fixture.store,
        &fixture.admin_id,
        &active.offer.offer_id,
        TerminateComputeOfferRequest {
            expected_offer_version: drained.target_offer_version,
            expected_offer_digest: drained.target_offer_digest.clone(),
            reason: "attempt early expiration".into(),
            idempotency_key: "expire-primary-early".into(),
            confirm_terminal: true,
        },
    );
    assert!(early_expiration
        .unwrap_err()
        .to_string()
        .contains("提前退出必须使用 revoked"));

    let revoked = compute_federation_offer_lifecycle_service::revoke_for_review(
        &fixture.store,
        &fixture.admin_id,
        &active.offer.offer_id,
        TerminateComputeOfferRequest {
            expected_offer_version: drained.target_offer_version,
            expected_offer_digest: drained.target_offer_digest.clone(),
            reason: "capacity withdrawn before expiry".into(),
            idempotency_key: "revoke-primary".into(),
            confirm_terminal: true,
        },
    )
    .unwrap();
    assert_eq!(revoked.previous_status, "draining");
    assert_eq!(revoked.target_status, "revoked");
    assert_eq!(revoked.target_offer_version, 5);
    assert_eq!(revoked.reservation_effect, "preserved");
    assert_eq!(revoked.attempt_effect, "none_direct");
    assert_eq!(revoked.funds_effect, "none");

    let persisted =
        compute_federation_offer_service::get_for_review(&fixture.store, &active.offer.offer_id)
            .unwrap();
    assert_eq!(persisted.offer.status, "revoked");
    assert_eq!(persisted.offer.offer_version, 5);
    assert!(compute_federation_offer_publication_service::get_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &active.offer.offer_id,
    )
    .is_ok());
    assert!(
        compute_federation_offer_lifecycle_service::get_drain_for_user(
            &fixture.store,
            &fixture.owner_id,
            &fixture.provider_id,
            &fixture.pool_id,
            &active.offer.offer_id,
        )
        .is_ok()
    );
    assert!(
        compute_federation_offer_lifecycle_service::get_terminal_for_user(
            &fixture.store,
            &fixture.owner_id,
            &fixture.provider_id,
            &fixture.pool_id,
            &active.offer.offer_id,
            "revoked",
        )
        .is_ok()
    );
}
