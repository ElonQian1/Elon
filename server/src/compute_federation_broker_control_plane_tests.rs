use chrono::{DateTime, Duration, Utc};

use crate::{
    compute_federation::{
        execution::{ComputeProviderScope, ComputeReservedCapacity},
        workload::{
            ComputeCheckpointPolicy, ComputeOutputContract, ComputeResourceRequirements,
            ComputeRetryPolicy, ComputeUsageLimit, ComputeVerificationPolicy, ComputeWorkloadSpec,
            COMPUTE_WORKLOAD_SCHEMA,
        },
    },
    compute_federation_broker_service::{
        self, CreateMyComputeJobRequest, FinishMyComputeRequest, QuoteMyComputeJobRequest,
        ReserveMyComputeRequest,
    },
    compute_federation_offer_publication_model::PublishComputeOfferDraftRequest,
    compute_federation_offer_publication_service, compute_federation_offer_service,
    compute_federation_offer_service::test_support::Fixture,
    compute_federation_price_snapshot_model::PublishMyComputePriceSnapshotRequest,
    compute_federation_price_snapshot_service,
    store::ComputeBrokerFinishAction,
};

#[test]
fn quote_reserve_and_release_are_atomic_and_replayable() {
    let fixture = BrokerFixture::new();
    let quoted = fixture.create_quoted_job("release");
    fixture
        .supply
        .store
        .billing_recharge(
            &fixture.consumer_id,
            100,
            "broker_test",
            &fixture.supply.admin_id,
            None,
        )
        .unwrap();

    let reserve_request = fixture.reserve_request(&quoted, "release", 20, 1);
    let reserved = compute_federation_broker_service::reserve_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        reserve_request.clone(),
    )
    .unwrap();
    assert!(!reserved.replayed);
    assert_eq!(reserved.status, "active");
    assert_eq!(reserved.budget_adapter, "platform_balance_cny");
    assert_eq!(reserved.budget_reserved_fen, 10);
    assert_eq!(reserved.reserved_job.job_revision, quoted.revision + 1);
    assert_eq!(reserved.reservation_revision, 2);
    assert_eq!(
        fixture
            .supply
            .store
            .billing_get_balance(&fixture.consumer_id)
            .unwrap(),
        Some(90)
    );
    fixture.assert_capacity(80, 20, 3, 1);

    let replayed = compute_federation_broker_service::reserve_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        reserve_request,
    )
    .unwrap();
    assert!(replayed.replayed);
    assert_eq!(replayed.reservation_id, reserved.reservation_id);
    assert_eq!(replayed.reservation_digest, reserved.reservation_digest);

    let finish_request = FinishMyComputeRequest {
        idempotency_key: "finish-release".into(),
        expected_reservation_revision: reserved.reservation_revision,
        expected_reservation_digest: reserved.reservation_digest.clone(),
    };
    let released = compute_federation_broker_service::finish_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        reserved.reservation_id.clone(),
        ComputeBrokerFinishAction::Release,
        finish_request.clone(),
    )
    .unwrap();
    assert!(!released.replayed);
    assert_eq!(released.action, ComputeBrokerFinishAction::Release);
    assert_eq!(released.status, "released");
    assert_eq!(released.budget_refunded_fen, 10);
    assert_eq!(released.reservation_revision, 3);
    assert_eq!(
        fixture
            .supply
            .store
            .billing_get_balance(&fixture.consumer_id)
            .unwrap(),
        Some(100)
    );
    fixture.assert_capacity(100, 0, 4, 0);

    let current_job = compute_federation_broker_service::get_job_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        &quoted.job.job_id,
    )
    .unwrap();
    assert_eq!(current_job.job.status, "canceled");
    let current_reservation = compute_federation_broker_service::get_reservation_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        &released.reservation_id,
    )
    .unwrap();
    assert_eq!(current_reservation.reservation.status, "released");

    let finish_replay = compute_federation_broker_service::finish_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        released.reservation_id.clone(),
        ComputeBrokerFinishAction::Release,
        finish_request,
    )
    .unwrap();
    assert!(finish_replay.replayed);
    assert_eq!(
        finish_replay.reservation_digest,
        released.reservation_digest
    );
}

#[test]
fn insufficient_balance_rolls_back_the_entire_reservation() {
    let fixture = BrokerFixture::new();
    let quoted = fixture.create_quoted_job("insufficient");
    fixture
        .supply
        .store
        .billing_recharge(
            &fixture.consumer_id,
            5,
            "broker_test",
            &fixture.supply.admin_id,
            None,
        )
        .unwrap();

    let request = fixture.reserve_request(&quoted, "insufficient", 20, 1);
    let rejected = compute_federation_broker_service::reserve_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        request.clone(),
    );
    assert!(format!("{:#}", rejected.unwrap_err()).contains("余额不足"));
    assert_eq!(
        fixture
            .supply
            .store
            .billing_get_balance(&fixture.consumer_id)
            .unwrap(),
        Some(5)
    );
    fixture.assert_capacity(100, 0, 4, 0);

    let current_job = compute_federation_broker_service::get_job_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        &quoted.job.job_id,
    )
    .unwrap();
    assert_eq!(current_job.job.status, "quoted");
    assert!(compute_federation_broker_service::get_reservation_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        &request.reservation_id,
    )
    .is_err());
}

struct BrokerFixture {
    supply: Fixture,
    consumer_id: String,
    project_id: String,
    snapshot: crate::compute_federation_price_snapshot_service::MyComputePriceSnapshotView,
}

impl BrokerFixture {
    fn new() -> Self {
        let supply = Fixture::new();
        supply.seed_active_supply();
        let draft = compute_federation_offer_service::create_draft_for_user(
            &supply.store,
            &supply.owner_id,
            &supply.provider_id,
            &supply.pool_id,
            supply.create_request("broker-offer", 100, 4),
        )
        .unwrap();
        compute_federation_offer_publication_service::publish_for_review(
            &supply.store,
            &supply.admin_id,
            &draft.offer.offer_id,
            PublishComputeOfferDraftRequest {
                expected_offer_version: draft.offer.offer_version,
                expected_offer_digest: draft.offer.offer_digest.clone(),
                idempotency_key: "publish-broker-offer".into(),
                confirm_publish: true,
            },
        )
        .unwrap();
        let active = compute_federation_offer_service::get_for_user(
            &supply.store,
            &supply.owner_id,
            &supply.provider_id,
            &supply.pool_id,
            &draft.offer.offer_id,
        )
        .unwrap();
        wait_until(&active.offer.valid_from);
        let snapshot = compute_federation_price_snapshot_service::publish_for_user(
            &supply.store,
            &supply.owner_id,
            &supply.provider_id,
            &supply.pool_id,
            &active.offer.offer_id,
            PublishMyComputePriceSnapshotRequest {
                expected_offer_version: active.offer.offer_version,
                expected_offer_digest: active.offer.offer_digest,
                delivery_window_id: supply.window_id.clone(),
                consumer_max_amount_micros: 100_000,
                provider_max_amount_micros: 80_000,
                ttl_seconds: 600,
                rounding_mode: "half_even".into(),
                idempotency_key: "broker-snapshot".into(),
                confirm_publish: true,
            },
        )
        .unwrap();
        let consumer = supply
            .store
            .create_user(
                &format!("broker-{}@example.com", uuid::Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let project_id = format!("project-{}", consumer.id);
        Self {
            supply,
            consumer_id: consumer.id,
            project_id,
            snapshot,
        }
    }

    fn create_quoted_job(&self, suffix: &str) -> crate::store::ComputeJobRegistrationReceipt {
        let created = compute_federation_broker_service::create_job_for_project(
            &self.supply.store,
            &self.consumer_id,
            &self.project_id,
            CreateMyComputeJobRequest {
                job_id: format!("job-{suffix}-{}", self.consumer_id),
                idempotency_key: format!("job-create-{suffix}"),
                merchant_id: None,
                workload: workload(),
                provider_scope: ComputeProviderScope {
                    allowed_provider_ids: vec![self.supply.provider_id.clone()],
                    allowed_provider_kinds: vec!["user_node".into()],
                    excluded_provider_ids: Vec::new(),
                    required_trust_tier: "platform_verified".into(),
                    required_regions: vec!["cn-east".into()],
                },
                max_consumer_charge_micros: 100_000,
                currency: "CNY".into(),
            },
        )
        .unwrap();
        assert_eq!(created.job.status, "submitted");

        let candidates = compute_federation_broker_service::list_quote_candidates_for_project(
            &self.supply.store,
            &self.consumer_id,
            &self.project_id,
            &created.job.job_id,
            20,
        )
        .unwrap();
        assert_eq!(candidates.candidates.len(), 1);
        let candidate = &candidates.candidates[0];
        assert_eq!(
            candidate.price_snapshot.snapshot_id,
            self.snapshot.snapshot.snapshot_id
        );
        let quoted = compute_federation_broker_service::quote_job_for_project(
            &self.supply.store,
            &self.consumer_id,
            &self.project_id,
            &created.job.job_id,
            QuoteMyComputeJobRequest {
                offer_id: candidate.offer.offer_id.clone(),
                price_snapshot_id: candidate.price_snapshot.snapshot_id.clone(),
                expected_job_revision: created.revision,
                expected_job_digest: created.job_digest,
            },
        )
        .unwrap();
        assert_eq!(quoted.job.status, "quoted");
        quoted
    }

    fn reserve_request(
        &self,
        quoted: &crate::store::ComputeJobRegistrationReceipt,
        suffix: &str,
        tokens: i64,
        concurrency: i64,
    ) -> ReserveMyComputeRequest {
        let snapshot_expiry = parse_utc(&self.snapshot.snapshot.expires_at);
        ReserveMyComputeRequest {
            reservation_id: format!("reservation-{suffix}-{}", self.consumer_id),
            idempotency_key: format!("reserve-{suffix}"),
            job_id: quoted.job.job_id.clone(),
            expected_job_revision: quoted.revision,
            expected_job_digest: quoted.job_digest.clone(),
            reserved_capacity: vec![
                ComputeReservedCapacity {
                    meter: "tokens".into(),
                    quantity: tokens,
                },
                ComputeReservedCapacity {
                    meter: "concurrency".into(),
                    quantity: concurrency,
                },
            ],
            expires_at: (snapshot_expiry - Duration::seconds(30)).to_rfc3339(),
        }
    }

    fn assert_capacity(
        &self,
        token_available: i64,
        token_held: i64,
        concurrency_available: i64,
        concurrency_held: i64,
    ) {
        let tokens = self
            .supply
            .store
            .compute_capacity_bucket_balance(&self.supply.token_bucket_id)
            .unwrap();
        let concurrency = self
            .supply
            .store
            .compute_capacity_bucket_balance(&self.supply.concurrency_bucket_id)
            .unwrap();
        assert_eq!(tokens.available_units, token_available);
        assert_eq!(tokens.held_units, token_held);
        assert_eq!(concurrency.available_units, concurrency_available);
        assert_eq!(concurrency.held_units, concurrency_held);
    }
}

fn workload() -> ComputeWorkloadSpec {
    ComputeWorkloadSpec {
        schema: COMPUTE_WORKLOAD_SCHEMA.into(),
        task_kind: "llm_chat".into(),
        input_artifacts: Vec::new(),
        model: None,
        runtime: None,
        resources: ComputeResourceRequirements {
            accelerator_kinds: vec!["consumer_gpu".into()],
            min_accelerator_count: 1,
            min_vram_bytes: 1024 * 1024 * 1024,
            min_ram_bytes: 2 * 1024 * 1024 * 1024,
            min_disk_bytes: 0,
            max_runtime_seconds: 1800,
            allow_network_egress: false,
        },
        output: ComputeOutputContract {
            media_type: "application/json".into(),
            max_output_bytes: 1024 * 1024,
            streaming: true,
            result_artifact_required: false,
            deterministic_digest_expected: false,
        },
        usage_limits: vec![
            ComputeUsageLimit {
                meter: "tokens".into(),
                max_quantity: 20,
            },
            ComputeUsageLimit {
                meter: "concurrency".into(),
                max_quantity: 1,
            },
        ],
        data_class: "public".into(),
        shard: None,
        retry_policy: ComputeRetryPolicy {
            max_attempts: 1,
            initial_backoff_ms: 0,
            max_backoff_ms: 0,
            retryable_error_codes: Vec::new(),
        },
        checkpoint_policy: ComputeCheckpointPolicy {
            mode: "disabled".into(),
            interval_seconds: None,
            max_checkpoints: 0,
            checkpoint_media_type: None,
        },
        verification_policy: ComputeVerificationPolicy {
            verification_tier: "platform_verified".into(),
            minimum_independent_receipts: 1,
            duplicate_sample_rate_basis_points: 0,
            challenge_profile_id: None,
            require_server_metering: true,
        },
        deadline_at: (Utc::now() + Duration::hours(1)).to_rfc3339(),
    }
}

fn wait_until(value: &str) {
    let boundary = parse_utc(value);
    if let Ok(wait) = (boundary - Utc::now()).to_std() {
        std::thread::sleep(wait + std::time::Duration::from_millis(20));
    }
}

fn parse_utc(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
