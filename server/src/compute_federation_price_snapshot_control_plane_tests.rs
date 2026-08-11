use chrono::{DateTime, Utc};

use crate::{
    compute_federation::market::PRICE_SOURCE_FALLBACK_CURVE,
    compute_federation_offer_lifecycle_model::DrainComputeOfferRequest,
    compute_federation_offer_lifecycle_service,
    compute_federation_offer_publication_model::PublishComputeOfferDraftRequest,
    compute_federation_offer_publication_service, compute_federation_offer_service,
    compute_federation_offer_service::test_support::{digest, Fixture},
    compute_federation_price_snapshot_model::PublishMyComputePriceSnapshotRequest,
    compute_federation_price_snapshot_service,
};

#[test]
fn active_offer_publishes_replays_and_lists_audited_snapshot() {
    let fixture = Fixture::new();
    let active = active_offer(&fixture);
    let request = snapshot_request(&fixture, &active.offer, "snapshot-primary", 100_000, 80_000);

    let published = compute_federation_price_snapshot_service::publish_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &active.offer.offer_id,
        request.clone(),
    )
    .unwrap();
    assert!(!published.replayed);
    assert_eq!(
        published.snapshot.price_source.source_kind,
        PRICE_SOURCE_FALLBACK_CURVE
    );
    assert_eq!(
        published.snapshot.price_source.source_version,
        active.offer.offer_version
    );
    assert_eq!(published.snapshot.price_source.sample_count, 0);
    assert_eq!(published.market_effect, "quote_candidate_enabled");
    assert_eq!(published.reservation_effect, "none");
    assert_eq!(published.capacity_effect, "none");
    assert_eq!(published.funds_effect, "none");
    assert!(
        parse_utc(&published.snapshot.expires_at) <= parse_utc(&active.offer.valid_until),
        "snapshot expiry must be capped by the Offer contract"
    );

    let replayed = compute_federation_price_snapshot_service::publish_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &active.offer.offer_id,
        request,
    )
    .unwrap();
    assert!(replayed.replayed);
    assert_eq!(
        replayed.snapshot.snapshot_id,
        published.snapshot.snapshot_id
    );

    let loaded = compute_federation_price_snapshot_service::get_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &active.offer.offer_id,
        &published.snapshot.snapshot_id,
    )
    .unwrap();
    assert_eq!(
        loaded.snapshot.snapshot_digest,
        published.snapshot.snapshot_digest
    );
    let listed = compute_federation_price_snapshot_service::list_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &active.offer.offer_id,
        20,
    )
    .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].snapshot.snapshot_id,
        published.snapshot.snapshot_id
    );
}

#[test]
fn snapshot_fails_closed_for_stale_replay_or_draining_offer() {
    let fixture = Fixture::new();
    let active = active_offer(&fixture);
    let mut request = snapshot_request(&fixture, &active.offer, "snapshot-guards", 90_000, 70_000);

    request.expected_offer_digest = digest('f');
    let stale = compute_federation_price_snapshot_service::publish_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &active.offer.offer_id,
        request.clone(),
    );
    assert!(stale
        .unwrap_err()
        .to_string()
        .contains("当前版本和摘要精确匹配的 active Offer"));

    request.expected_offer_digest = active.offer.offer_digest.clone();
    compute_federation_price_snapshot_service::publish_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &active.offer.offer_id,
        request.clone(),
    )
    .unwrap();
    request.provider_max_amount_micros -= 1;
    let changed_replay = compute_federation_price_snapshot_service::publish_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &active.offer.offer_id,
        request,
    );
    assert!(changed_replay
        .unwrap_err()
        .to_string()
        .contains("幂等键已绑定不同报价合同"));

    let drained = compute_federation_offer_lifecycle_service::drain_for_review(
        &fixture.store,
        &fixture.admin_id,
        &active.offer.offer_id,
        DrainComputeOfferRequest {
            expected_offer_version: active.offer.offer_version,
            expected_offer_digest: active.offer.offer_digest.clone(),
            reason: "snapshot gate verification".into(),
            idempotency_key: "drain-before-new-snapshot".into(),
            confirm_drain: true,
        },
    )
    .unwrap();
    let after_drain = compute_federation_price_snapshot_service::publish_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &active.offer.offer_id,
        PublishMyComputePriceSnapshotRequest {
            expected_offer_version: drained.target_offer_version,
            expected_offer_digest: drained.target_offer_digest,
            delivery_window_id: fixture.window_id.clone(),
            consumer_max_amount_micros: 80_000,
            provider_max_amount_micros: 60_000,
            ttl_seconds: 300,
            rounding_mode: "half_even".into(),
            idempotency_key: "snapshot-after-drain".into(),
            confirm_publish: true,
        },
    );
    assert!(after_drain
        .unwrap_err()
        .to_string()
        .contains("当前版本和摘要精确匹配的 active Offer"));
}

fn active_offer(fixture: &Fixture) -> crate::compute_federation_offer_service::MyComputeOfferView {
    fixture.seed_active_supply();
    let draft = compute_federation_offer_service::create_draft_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_request("snapshot-offer", 100, 4),
    )
    .unwrap();
    compute_federation_offer_publication_service::publish_for_review(
        &fixture.store,
        &fixture.admin_id,
        &draft.offer.offer_id,
        PublishComputeOfferDraftRequest {
            expected_offer_version: draft.offer.offer_version,
            expected_offer_digest: draft.offer.offer_digest.clone(),
            idempotency_key: "publish-snapshot-offer".into(),
            confirm_publish: true,
        },
    )
    .unwrap();
    let active = compute_federation_offer_service::get_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &draft.offer.offer_id,
    )
    .unwrap();
    let valid_from = parse_utc(&active.offer.valid_from);
    if let Ok(wait) = (valid_from - Utc::now()).to_std() {
        std::thread::sleep(wait + std::time::Duration::from_millis(20));
    }
    active
}

fn snapshot_request(
    fixture: &Fixture,
    offer: &crate::compute_federation::offer::ComputeOffer,
    idempotency_key: &str,
    consumer_max_amount_micros: i64,
    provider_max_amount_micros: i64,
) -> PublishMyComputePriceSnapshotRequest {
    PublishMyComputePriceSnapshotRequest {
        expected_offer_version: offer.offer_version,
        expected_offer_digest: offer.offer_digest.clone(),
        delivery_window_id: fixture.window_id.clone(),
        consumer_max_amount_micros,
        provider_max_amount_micros,
        ttl_seconds: 300,
        rounding_mode: "half_even".into(),
        idempotency_key: idempotency_key.into(),
        confirm_publish: true,
    }
}

fn parse_utc(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
