use chrono::{DateTime, Duration, SecondsFormat, Utc};

use crate::{
    compute_federation::platform_reference_price_curve::{
        ComputePlatformReferencePriceCurveComponent, ComputePlatformReferencePriceCurveEntryIntent,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CONFIRMATION,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_METHODOLOGY,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ROUNDING_MODE,
    },
    compute_federation_offer_publication_model::PublishComputeOfferDraftRequest,
    compute_federation_offer_publication_service, compute_federation_offer_service,
    compute_federation_offer_service::test_support::Fixture,
    store::Store,
};

use super::types::{
    ApplyComputePlatformReferencePriceCurveBatch, ReviewComputePlatformReferencePriceCurveBatch,
    SubmitComputePlatformReferencePriceCurveBatch,
    PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION,
    PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION,
};

const SUBMITTER: &str = "reference-curve-submitter";
const REVIEWER: &str = "reference-curve-reviewer";
const APPLIER: &str = "reference-curve-applier";

#[test]
fn approved_batch_atomically_registers_snapshot_and_survives_reopen() {
    let (fixture, offer) = active_offer();
    let valid_from = canonical(Utc::now() + Duration::milliseconds(250));
    let valid_until = canonical(Utc::now() + Duration::minutes(30));
    let submit = || {
        submit_input(
            &fixture,
            &offer.offer,
            &valid_from,
            &valid_until,
            "curve-v1",
        )
    };

    let batch = fixture
        .store
        .submit_compute_platform_reference_price_curve_batch(submit())
        .expect("batch submits with a server-assigned timestamp");
    assert_eq!(batch.status, "submitted");
    assert_eq!(batch.entries.len(), 1);
    assert!(!batch.replayed);
    let replay = fixture
        .store
        .submit_compute_platform_reference_price_curve_batch(submit())
        .expect("exact submission replays");
    assert_eq!(replay.batch_id, batch.batch_id);
    assert!(replay.replayed);

    let mut self_review = review_input(&batch, "approved", SUBMITTER);
    self_review.idempotency_key = "curve-v1-self-review".into();
    assert!(fixture
        .store
        .review_compute_platform_reference_price_curve_batch(self_review)
        .err()
        .expect("submitter cannot self-review")
        .to_string()
        .contains("cannot review"));
    let review = fixture
        .store
        .review_compute_platform_reference_price_curve_batch(review_input(
            &batch, "approved", REVIEWER,
        ))
        .expect("independent administrator approves");
    assert_eq!(review.decision, "approved");

    let mut wrong = apply_input(&batch, &review);
    wrong.expected_review_digest = "f".repeat(64);
    let wrong_review_error = fixture
        .store
        .apply_compute_platform_reference_price_curve_batch(wrong)
        .err()
        .expect("wrong review digest fails")
        .to_string();
    assert!(
        wrong_review_error.contains("exact approved"),
        "{wrong_review_error}"
    );
    wait_until(&valid_from);
    let application = fixture
        .store
        .apply_compute_platform_reference_price_curve_batch(apply_input(&batch, &review))
        .expect("approved batch applies atomically");
    assert_eq!(application.status, "applied");
    assert_eq!(application.market_effect, "quote_candidate_enabled");
    assert_eq!(application.bindings.len(), 1);
    let binding = &application.bindings[0];
    let snapshot = fixture
        .store
        .compute_price_snapshot(&binding.snapshot_id)
        .expect("bound v171 Snapshot reads");
    assert_eq!(snapshot.snapshot.price_source.source_kind, "fallback_curve");
    assert_eq!(snapshot.snapshot.price_source.sample_count, 0);
    assert!(snapshot.snapshot.trade_id.is_none());
    assert_eq!(
        snapshot.snapshot.price_source.source_digest,
        binding.entry_digest
    );

    let replayed = fixture
        .store
        .apply_compute_platform_reference_price_curve_batch(apply_input(&batch, &review))
        .expect("exact application replays");
    assert_eq!(replayed.application_id, application.application_id);
    assert!(replayed.replayed);
    let detail = fixture
        .store
        .platform_reference_price_curve_batch(&batch.batch_id)
        .expect("applied detail reads");
    assert_eq!(detail.batch.status, "applied");
    assert_eq!(detail.review.unwrap().review_id, review.review_id);
    assert_eq!(
        detail.application.unwrap().application_id,
        application.application_id
    );

    let database_path = fixture.root.join("state.sqlite");
    let root = fixture.root.clone();
    drop(fixture);
    let reopened = Store::open(&database_path).expect("Store reopens after v223 application");
    let listed = reopened
        .list_platform_reference_price_curve_batches_for_admin(Some("applied"), 1_000)
        .expect("applied batch survives reopen");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].batch.batch_id, batch.batch_id);
    assert_eq!(
        listed[0]
            .application
            .as_ref()
            .unwrap()
            .bindings
            .first()
            .unwrap()
            .snapshot_id,
        binding.snapshot_id
    );
    drop(reopened);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn non_approval_closes_application_without_partial_snapshot_rows() {
    let (fixture, offer) = active_offer();
    let valid_from = canonical(Utc::now() + Duration::milliseconds(250));
    let valid_until = canonical(Utc::now() + Duration::minutes(30));
    let batch = fixture
        .store
        .submit_compute_platform_reference_price_curve_batch(submit_input(
            &fixture,
            &offer.offer,
            &valid_from,
            &valid_until,
            "curve-v2",
        ))
        .expect("batch submits");
    let review = fixture
        .store
        .review_compute_platform_reference_price_curve_batch(review_input(
            &batch,
            "changes_requested",
            REVIEWER,
        ))
        .expect("changes are requested");
    assert!(fixture
        .store
        .apply_compute_platform_reference_price_curve_batch(apply_input(&batch, &review))
        .err()
        .expect("non-approved batch cannot apply")
        .to_string()
        .contains("exact approved"));
    let detail = fixture
        .store
        .platform_reference_price_curve_batch(&batch.batch_id)
        .expect("closed detail reads");
    assert_eq!(detail.batch.status, "changes_requested");
    assert!(detail.application.is_none());
    let connection = fixture.store.conn().unwrap();
    for table in [
        "compute_platform_reference_price_curve_applications",
        "compute_platform_reference_price_curve_snapshot_bindings",
        "compute_price_snapshots",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "unexpected partial rows in {table}");
    }
    assert!(fixture
        .store
        .list_platform_reference_price_curve_batches_for_admin(Some("canceled"), 20)
        .err()
        .expect("unknown status fails closed")
        .to_string()
        .contains("status filter is unsupported"));
    drop(connection);
    let root = fixture.root.clone();
    drop(fixture);
    let _ = std::fs::remove_dir_all(root);
}

fn active_offer() -> (
    Fixture,
    crate::compute_federation_offer_service::MyComputeOfferView,
) {
    let mut fixture = Fixture::new();
    let now = Utc::now();
    fixture.starts_at = (now + Duration::milliseconds(300)).to_rfc3339();
    fixture.ends_at = (now + Duration::hours(2)).to_rfc3339();
    fixture.valid_until = (now + Duration::hours(3)).to_rfc3339();
    fixture.seed_active_supply();
    let draft = compute_federation_offer_service::create_draft_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_request("reference-curve-offer", 100, 4),
    )
    .unwrap();
    compute_federation_offer_publication_service::publish_for_review(
        &fixture.store,
        &fixture.admin_id,
        &draft.offer.offer_id,
        PublishComputeOfferDraftRequest {
            expected_offer_version: draft.offer.offer_version,
            expected_offer_digest: draft.offer.offer_digest,
            idempotency_key: "publish-reference-curve-offer".into(),
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
    wait_until(&canonical(
        DateTime::parse_from_rfc3339(&active.offer.valid_from)
            .unwrap()
            .with_timezone(&Utc),
    ));
    (fixture, active)
}

fn submit_input(
    fixture: &Fixture,
    offer: &crate::compute_federation::offer::ComputeOffer,
    valid_from: &str,
    valid_until: &str,
    idempotency_key: &str,
) -> SubmitComputePlatformReferencePriceCurveBatch {
    SubmitComputePlatformReferencePriceCurveBatch {
        submitted_by_admin_user_id: SUBMITTER.into(),
        curve_id: "platform-reference-cny".into(),
        curve_version: if idempotency_key == "curve-v1" { 1 } else { 2 },
        methodology_kind: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_METHODOLOGY.into(),
        valid_from: valid_from.into(),
        valid_until: valid_until.into(),
        quote_ttl_seconds: 300,
        rounding_mode: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ROUNDING_MODE.into(),
        entries: vec![entry(fixture, offer)],
        idempotency_key: idempotency_key.into(),
        confirmation: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CONFIRMATION.into(),
        submission_note: "governed fallback only; no market samples".into(),
        idempotency_scope: format!("platform-reference-curve:{idempotency_key}"),
    }
}

fn entry(
    fixture: &Fixture,
    offer: &crate::compute_federation::offer::ComputeOffer,
) -> ComputePlatformReferencePriceCurveEntryIntent {
    let window = offer
        .delivery_windows
        .iter()
        .find(|window| window.binding.window_id == fixture.window_id)
        .unwrap();
    ComputePlatformReferencePriceCurveEntryIntent {
        entry_key: format!("{}:{}", offer.offer_id, fixture.window_id),
        provider_id: offer.provider_id.clone(),
        offer_id: offer.offer_id.clone(),
        offer_version: offer.offer_version,
        offer_digest: offer.offer_digest.clone(),
        sku_id: offer.sku.sku_id.clone(),
        sku_digest: offer.sku.sku_digest.clone(),
        delivery_window_id: window.binding.window_id.clone(),
        delivery_window_digest: window.binding.window_digest.clone(),
        pricing_mode: offer.price_terms.pricing_mode.clone(),
        currency: offer.price_terms.currency.clone(),
        offer_curve_id: offer.price_terms.curve_id.clone(),
        offer_curve_version: offer.price_terms.curve_version,
        instrument_id: offer.price_terms.instrument_id.clone(),
        components: offer
            .price_terms
            .components
            .iter()
            .map(|component| ComputePlatformReferencePriceCurveComponent {
                meter: component.meter.clone(),
                unit_size: component.unit_size,
                consumer_unit_price_micros: component.consumer_unit_price_micros,
                provider_unit_price_micros: component.provider_unit_price_micros,
                max_units: component.max_units,
            })
            .collect(),
        fee_rules: Vec::new(),
        consumer_max_amount_micros: 100_000,
        provider_max_amount_micros: 80_000,
    }
}

fn review_input(
    batch: &super::types::ComputePlatformReferencePriceCurveBatchReceipt,
    decision: &str,
    reviewer: &str,
) -> ReviewComputePlatformReferencePriceCurveBatch {
    ReviewComputePlatformReferencePriceCurveBatch {
        batch_id: batch.batch_id.clone(),
        expected_batch_digest: batch.batch_digest.clone(),
        expected_batch_material_digest: batch.batch_material_digest.clone(),
        decision: decision.into(),
        review_confirmation: PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION.into(),
        review_note: (decision != "approved").then(|| "revise reference batch".into()),
        reviewed_by_admin_user_id: reviewer.into(),
        idempotency_scope: format!("platform-reference-review:{reviewer}"),
        idempotency_key: format!("review-{}-{decision}", batch.batch_id),
    }
}

fn apply_input(
    batch: &super::types::ComputePlatformReferencePriceCurveBatchReceipt,
    review: &super::types::ComputePlatformReferencePriceCurveReviewReceipt,
) -> ApplyComputePlatformReferencePriceCurveBatch {
    ApplyComputePlatformReferencePriceCurveBatch {
        batch_id: batch.batch_id.clone(),
        expected_batch_digest: batch.batch_digest.clone(),
        expected_batch_material_digest: batch.batch_material_digest.clone(),
        expected_review_id: review.review_id.clone(),
        expected_review_digest: review.review_digest.clone(),
        applied_by_admin_user_id: APPLIER.into(),
        apply_confirmation: PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION.into(),
        apply_note: "register v171 fallback snapshots only".into(),
        idempotency_scope: "platform-reference-apply".into(),
        idempotency_key: format!("apply-{}", batch.batch_id),
    }
}

fn canonical(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn wait_until(value: &str) {
    let target = DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc);
    if let Ok(wait) = (target - Utc::now()).to_std() {
        std::thread::sleep(wait + std::time::Duration::from_millis(20));
    }
}
