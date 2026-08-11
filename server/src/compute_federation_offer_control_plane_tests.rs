use crate::{
    compute_federation_offer_draft_model::RevokeMyComputeOfferDraftRequest,
    compute_federation_offer_service,
};

use super::test_support::{digest, Fixture};

#[test]
fn owner_can_revoke_only_an_exact_current_draft() {
    let fixture = Fixture::new();
    fixture.seed_active_supply();
    let created = compute_federation_offer_service::create_draft_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_request("owner-revocation", 100, 4),
    )
    .unwrap();

    let stale = compute_federation_offer_service::revoke_draft_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &created.offer.offer_id,
        RevokeMyComputeOfferDraftRequest {
            expected_offer_version: 1,
            expected_offer_digest: digest('f'),
            confirm_revoke: true,
        },
    );
    assert!(stale.unwrap_err().to_string().contains("版本或摘要已变化"));

    let revoked = compute_federation_offer_service::revoke_draft_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &created.offer.offer_id,
        RevokeMyComputeOfferDraftRequest {
            expected_offer_version: created.offer.offer_version,
            expected_offer_digest: created.offer.offer_digest.clone(),
            confirm_revoke: true,
        },
    )
    .unwrap();
    assert_eq!(revoked.offer.status, "revoked");
    assert_eq!(revoked.offer.offer_version, 2);
    assert_eq!(revoked.market_effect, "none");
}
