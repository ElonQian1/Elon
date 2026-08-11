use chrono::{DateTime, Duration, SecondsFormat, Utc};
use uuid::Uuid;

use crate::{
    compute_federation::{
        capacity_commitment::ComputeCapacityCommitmentQuantity,
        platform_reference_price_curve::{
            ComputePlatformReferencePriceCurveComponent,
            ComputePlatformReferencePriceCurveEntryIntent,
            COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CONFIRMATION,
            COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_METHODOLOGY,
            COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ROUNDING_MODE,
        },
    },
    compute_federation_offer_publication_model::PublishComputeOfferDraftRequest,
    compute_federation_offer_publication_service, compute_federation_offer_service,
    compute_federation_offer_service::test_support::Fixture as OfferFixture,
    store::{
        ApplyComputePlatformReferencePriceCurveBatch,
        ComputePlatformReferencePriceCurveApplicationReceipt,
        ComputePlatformReferencePriceCurveBatchReceipt,
        ComputePlatformReferencePriceCurveReviewReceipt, PublicUser,
        ReviewComputePlatformReferencePriceCurveBatch, Store,
        SubmitComputePlatformReferencePriceCurveBatch,
        PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION,
        PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION,
    },
};

use super::CreateCapacityCommitmentBody;

pub(crate) struct Fixture {
    pub(crate) store: Store,
    pub(crate) root: std::path::PathBuf,
    pub(crate) owner_id: String,
    pub(crate) admin_id: String,
    pub(crate) outsider_id: Option<String>,
    pub(crate) owner_token: Option<String>,
    pub(crate) admin_token: Option<String>,
    pub(crate) outsider_token: Option<String>,
    pub(crate) provider_id: String,
    pub(crate) pool_id: String,
    pub(crate) token_bucket_id: String,
    pub(crate) concurrency_bucket_id: String,
    pub(crate) offer: crate::compute_federation::offer::ComputeOffer,
    pub(crate) provider_policy_revision: i64,
    pub(crate) provider_digest: String,
    pub(crate) binding: crate::store::ComputePlatformReferencePriceCurveSnapshotBindingReceipt,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        Self::build(OfferFixture::new(), false)
    }

    pub(crate) fn new_http() -> Self {
        Self::build(OfferFixture::new(), true)
    }

    fn build(mut source: OfferFixture, with_users: bool) -> Self {
        let (outsider_id, owner_token, admin_token, outsider_token) = if with_users {
            let owner = create_user(&source.store, "capacity-owner", None);
            let admin = create_user(&source.store, "capacity-admin", Some("admin"));
            let outsider = create_user(&source.store, "capacity-outsider", None);
            source.owner_id = owner.id.clone();
            source.admin_id = admin.id.clone();
            let owner_token = session(&source.store, &owner.id);
            let admin_token = session(&source.store, &admin.id);
            let outsider_token = session(&source.store, &outsider.id);
            (
                Some(outsider.id),
                Some(owner_token),
                Some(admin_token),
                Some(outsider_token),
            )
        } else {
            (None, None, None, None)
        };

        let now = Utc::now();
        source.starts_at = canonical(now + Duration::seconds(4));
        source.ends_at = canonical(now + Duration::seconds(6));
        source.valid_until = canonical(now + Duration::minutes(10));
        source.seed_active_supply();
        let provider = source.store.compute_provider(&source.provider_id).unwrap();

        let instrument_id = format!("capacity-future-{}", Uuid::new_v4().simple());
        let mut request = source.create_request("capacity-commitment-offer", 100, 4);
        request.valid_from = canonical(now + Duration::milliseconds(150));
        request.price_terms.pricing_mode = "capacity_future".into();
        request.price_terms.curve_id = Some("platform-reference-cny".into());
        request.price_terms.curve_version = Some(1);
        request.price_terms.instrument_id = Some(instrument_id);
        let draft = compute_federation_offer_service::create_draft_for_user(
            &source.store,
            &source.owner_id,
            &source.provider_id,
            &source.pool_id,
            request,
        )
        .unwrap();
        compute_federation_offer_publication_service::publish_for_review(
            &source.store,
            &source.admin_id,
            &draft.offer.offer_id,
            PublishComputeOfferDraftRequest {
                expected_offer_version: draft.offer.offer_version,
                expected_offer_digest: draft.offer.offer_digest,
                idempotency_key: "publish-capacity-commitment-offer".into(),
                confirm_publish: true,
            },
        )
        .unwrap();
        let offer = compute_federation_offer_service::get_for_user(
            &source.store,
            &source.owner_id,
            &source.provider_id,
            &source.pool_id,
            &draft.offer.offer_id,
        )
        .unwrap()
        .offer;
        wait_until(&offer.valid_from);
        let application = apply_reference_binding(&source, &offer);
        let binding = application.bindings.into_iter().next().unwrap();

        Self {
            store: source.store,
            root: source.root,
            owner_id: source.owner_id,
            admin_id: source.admin_id,
            outsider_id,
            owner_token,
            admin_token,
            outsider_token,
            provider_id: source.provider_id,
            pool_id: source.pool_id,
            token_bucket_id: source.token_bucket_id,
            concurrency_bucket_id: source.concurrency_bucket_id,
            offer,
            provider_policy_revision: provider.provider.policy_revision,
            provider_digest: provider.provider_digest,
            binding,
        }
    }

    pub(crate) fn create_body(
        &self,
        idempotency_key: &str,
        confirm_commitment: bool,
    ) -> CreateCapacityCommitmentBody {
        let window = self
            .offer
            .delivery_windows
            .iter()
            .find(|window| window.binding.window_id == self.binding_window_id())
            .unwrap();
        CreateCapacityCommitmentBody {
            idempotency_key: idempotency_key.into(),
            provider_policy_revision: self.provider_policy_revision,
            provider_digest: self.provider_digest.clone(),
            offer_id: self.offer.offer_id.clone(),
            offer_version: self.offer.offer_version,
            offer_digest: self.offer.offer_digest.clone(),
            capacity_epoch: self.offer.capacity_pool.capacity_epoch,
            pool_revision: self.offer.capacity_pool.pool_revision,
            pool_digest: self.offer.capacity_pool.pool_digest.clone(),
            delivery_window_id: window.binding.window_id.clone(),
            delivery_window_digest: window.binding.window_digest.clone(),
            price_snapshot_id: self.binding.snapshot_id.clone(),
            price_snapshot_digest: self.binding.snapshot_digest.clone(),
            reference_binding_id: self.binding.binding_id.clone(),
            reference_binding_digest: self.binding.binding_digest.clone(),
            instrument_id: self.offer.price_terms.instrument_id.clone().unwrap(),
            quantities: vec![
                ComputeCapacityCommitmentQuantity {
                    meter: "tokens".into(),
                    quantity_units: 20,
                },
                ComputeCapacityCommitmentQuantity {
                    meter: "concurrency".into(),
                    quantity_units: 1,
                },
            ],
            confirm_commitment,
        }
    }

    pub(crate) fn balance(&self, bucket_id: &str) -> (i64, i64) {
        let balance = self
            .store
            .compute_capacity_bucket_balance(bucket_id)
            .unwrap();
        (balance.available_units, balance.held_units)
    }

    pub(crate) fn collection_path(&self) -> String {
        format!(
            "/api/me/compute/providers/{}/capacity-pools/{}/capacity-commitments",
            self.provider_id, self.pool_id
        )
    }

    pub(crate) fn cleanup(self) {
        let root = self.root.clone();
        drop(self);
        let _ = std::fs::remove_dir_all(root);
    }

    fn binding_window_id(&self) -> &str {
        self.offer
            .delivery_windows
            .first()
            .map(|window| window.binding.window_id.as_str())
            .unwrap()
    }
}

fn apply_reference_binding(
    fixture: &OfferFixture,
    offer: &crate::compute_federation::offer::ComputeOffer,
) -> ComputePlatformReferencePriceCurveApplicationReceipt {
    let valid_from = canonical(Utc::now() + Duration::milliseconds(150));
    let batch = fixture
        .store
        .submit_compute_platform_reference_price_curve_batch(reference_submission(
            fixture,
            offer,
            &valid_from,
        ))
        .unwrap();
    let review = fixture
        .store
        .review_compute_platform_reference_price_curve_batch(reference_review(&batch))
        .unwrap();
    wait_until(&valid_from);
    fixture
        .store
        .apply_compute_platform_reference_price_curve_batch(reference_application(&batch, &review))
        .unwrap()
}

fn reference_submission(
    fixture: &OfferFixture,
    offer: &crate::compute_federation::offer::ComputeOffer,
    valid_from: &str,
) -> SubmitComputePlatformReferencePriceCurveBatch {
    let window = offer
        .delivery_windows
        .iter()
        .find(|window| window.binding.window_id == fixture.window_id)
        .unwrap();
    SubmitComputePlatformReferencePriceCurveBatch {
        submitted_by_admin_user_id: fixture.admin_id.clone(),
        curve_id: "platform-reference-cny".into(),
        curve_version: 1,
        methodology_kind: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_METHODOLOGY.into(),
        valid_from: valid_from.into(),
        valid_until: fixture.valid_until.clone(),
        quote_ttl_seconds: 300,
        rounding_mode: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ROUNDING_MODE.into(),
        entries: vec![ComputePlatformReferencePriceCurveEntryIntent {
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
        }],
        idempotency_key: "capacity-reference-submit".into(),
        confirmation: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CONFIRMATION.into(),
        submission_note: "capacity commitment test reference".into(),
        idempotency_scope: "capacity-reference-submit".into(),
    }
}

fn reference_review(
    batch: &ComputePlatformReferencePriceCurveBatchReceipt,
) -> ReviewComputePlatformReferencePriceCurveBatch {
    ReviewComputePlatformReferencePriceCurveBatch {
        batch_id: batch.batch_id.clone(),
        expected_batch_digest: batch.batch_digest.clone(),
        expected_batch_material_digest: batch.batch_material_digest.clone(),
        decision: "approved".into(),
        review_confirmation: PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION.into(),
        review_note: None,
        reviewed_by_admin_user_id: "capacity-reference-reviewer".into(),
        idempotency_scope: "capacity-reference-review".into(),
        idempotency_key: "capacity-reference-review".into(),
    }
}

fn reference_application(
    batch: &ComputePlatformReferencePriceCurveBatchReceipt,
    review: &ComputePlatformReferencePriceCurveReviewReceipt,
) -> ApplyComputePlatformReferencePriceCurveBatch {
    ApplyComputePlatformReferencePriceCurveBatch {
        batch_id: batch.batch_id.clone(),
        expected_batch_digest: batch.batch_digest.clone(),
        expected_batch_material_digest: batch.batch_material_digest.clone(),
        expected_review_id: review.review_id.clone(),
        expected_review_digest: review.review_digest.clone(),
        applied_by_admin_user_id: "capacity-reference-applier".into(),
        apply_confirmation: PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION.into(),
        apply_note: "register capacity future snapshot".into(),
        idempotency_scope: "capacity-reference-apply".into(),
        idempotency_key: "capacity-reference-apply".into(),
    }
}

fn create_user(store: &Store, prefix: &str, role: Option<&str>) -> PublicUser {
    store
        .create_user(
            &format!("{prefix}-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            None,
            role,
        )
        .unwrap()
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("capacity-commitment-test"), None)
        .unwrap()
        .0
}

pub(crate) fn canonical(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

pub(crate) fn wait_until(value: &str) {
    let target = DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc);
    if let Ok(wait) = (target - Utc::now()).to_std() {
        std::thread::sleep(wait + std::time::Duration::from_millis(20));
    }
}
