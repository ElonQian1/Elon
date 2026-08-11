use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    compute_federation::{
        capacity::ComputeCapacityPoolStatus,
        market::{ComputeFeeRule, ComputePriceComponent},
        offer::ComputeOfferExecutionLimits,
        provider::{
            ComputeProvider, ComputeProviderCapabilities, ComputeProviderEndpointRef,
            ComputeProviderEvidenceProfile, COMPUTE_PROVIDER_SCHEMA,
        },
        workload::ComputeRuntimeRef,
    },
    compute_federation_capacity_bucket_service::{self, CreateMyComputeCapacityBucketRequest},
    compute_federation_capacity_pool_service::{
        self, CreateMyComputeCapacityMeterPolicyRequest, CreateMyComputeCapacityPoolRequest,
    },
    compute_federation_capacity_supply_service::{
        self, AddMyComputeCapacitySupplyLineRequest, AddMyComputeCapacitySupplyRequest,
    },
    compute_federation_offer_draft_model::{
        ComputeOfferDraftAuthorizationInput, ComputeOfferDraftCapacityInput,
        ComputeOfferDraftPriceTermsInput, ComputeOfferDraftResourceProfileInput,
        ComputeOfferDraftSkuInput, CreateMyComputeOfferDraftRequest,
        ReviseMyComputeOfferDraftRequest,
    },
    store::{Store, TransitionComputeCapacityPoolStatus},
};

pub(crate) struct Fixture {
    pub(crate) store: Store,
    pub(crate) owner_id: String,
    pub(crate) admin_id: String,
    pub(crate) provider_id: String,
    pub(crate) pool_id: String,
    pub(crate) token_bucket_id: String,
    pub(crate) concurrency_bucket_id: String,
    pub(crate) window_id: String,
    pub(crate) starts_at: String,
    pub(crate) ends_at: String,
    pub(crate) valid_until: String,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let suffix = Uuid::new_v4().simple().to_string();
        let root = std::env::temp_dir().join(format!("elon-offer-control-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        Self {
            store: Store::open(&root.join("state.sqlite")).unwrap(),
            owner_id: format!("offer-owner-{suffix}"),
            admin_id: format!("offer-admin-{suffix}"),
            provider_id: format!("provider-{suffix}"),
            pool_id: format!("pool-{suffix}"),
            token_bucket_id: format!("bucket-token-{suffix}"),
            concurrency_bucket_id: format!("bucket-concurrency-{suffix}"),
            window_id: format!("window-{suffix}"),
            starts_at: (Utc::now() + Duration::seconds(3)).to_rfc3339(),
            ends_at: (Utc::now() + Duration::hours(3)).to_rfc3339(),
            valid_until: (Utc::now() + Duration::hours(4)).to_rfc3339(),
        }
    }

    pub(crate) fn seed_active_supply(&self) {
        let now = Utc::now().to_rfc3339();
        let mut provider = ComputeProvider {
            schema: COMPUTE_PROVIDER_SCHEMA.into(),
            provider_id: self.provider_id.clone(),
            provider_kind: "user_node".into(),
            owner_account_id: self.owner_id.clone(),
            settlement_account_id: Some(self.owner_id.clone()),
            display_name: "Offer test provider".into(),
            status: "registering".into(),
            trust_tier: "platform_verified".into(),
            home_region: Some("cn-east".into()),
            policy_revision: 1,
            capabilities: ComputeProviderCapabilities {
                task_kinds: vec!["llm_chat".into()],
                accelerator_kinds: vec!["consumer_gpu".into()],
                regions: vec!["cn-east".into()],
                allowed_data_classes: vec!["public".into()],
                supports_streaming: true,
                supports_checkpointing: false,
            },
            endpoint: Some(ComputeProviderEndpointRef {
                endpoint_id: format!("endpoint-{}", self.provider_id),
                transport: "https".into(),
                address_hint: Some("provider.test.invalid".into()),
                gateway_id: Some("gateway-test".into()),
                credential_ref: Some("vault://offer-test".into()),
            }),
            adapter: None,
            evidence_profile: ComputeProviderEvidenceProfile {
                declared_hardware_digest: Some(digest('a')),
                observed_hardware_digest: Some(digest('b')),
                verified_hardware_digest: Some(digest('c')),
                last_observed_at: Some(now.clone()),
                last_verified_at: Some(now.clone()),
            },
            created_at: now.clone(),
            updated_at: now,
        };
        self.store.register_compute_provider(&provider).unwrap();
        compute_federation_capacity_pool_service::create_for_user(
            &self.store,
            &self.owner_id,
            &self.provider_id,
            CreateMyComputeCapacityPoolRequest {
                pool_id: self.pool_id.clone(),
                resource_scope_key: "desktop-gpu-0".into(),
                region_or_data_zone: "cn-east".into(),
                resource_profile: serde_json::json!({"accelerator":"consumer_gpu","count":1}),
                meter_policies: vec![
                    meter_policy("tokens", "consumable", 10),
                    meter_policy("concurrency", "reusable", 1),
                ],
            },
        )
        .unwrap();
        for (bucket_id, meter) in [
            (&self.token_bucket_id, "tokens"),
            (&self.concurrency_bucket_id, "concurrency"),
        ] {
            compute_federation_capacity_bucket_service::create_for_user(
                &self.store,
                &self.owner_id,
                &self.provider_id,
                &self.pool_id,
                CreateMyComputeCapacityBucketRequest {
                    bucket_id: bucket_id.clone(),
                    window_id: self.window_id.clone(),
                    starts_at_utc: self.starts_at.clone(),
                    ends_at_utc: self.ends_at.clone(),
                    meter: meter.into(),
                },
            )
            .unwrap();
        }
        compute_federation_capacity_supply_service::add_for_user(
            &self.store,
            &self.owner_id,
            &self.provider_id,
            &self.pool_id,
            AddMyComputeCapacitySupplyRequest {
                idempotency_key: "seed-offer-supply".into(),
                lines: vec![
                    AddMyComputeCapacitySupplyLineRequest {
                        bucket_id: self.token_bucket_id.clone(),
                        quantity_units: 100,
                    },
                    AddMyComputeCapacitySupplyLineRequest {
                        bucket_id: self.concurrency_bucket_id.clone(),
                        quantity_units: 4,
                    },
                ],
                confirm_supply: true,
            },
        )
        .unwrap();
        provider.status = "active".into();
        provider.policy_revision = 2;
        provider.updated_at = Utc::now().to_rfc3339();
        self.store.register_compute_provider(&provider).unwrap();
        self.store
            .transition_compute_capacity_pool_status(TransitionComputeCapacityPoolStatus {
                pool_id: self.pool_id.clone(),
                expected_capacity_epoch: 1,
                expected_status: ComputeCapacityPoolStatus::Registering,
                target_status: ComputeCapacityPoolStatus::Active,
                reason_code: "test_prerequisite".into(),
                subject_kind: "offer_control_plane_test".into(),
                subject_id: self.provider_id.clone(),
                idempotency_scope: format!("offer-test-pool:active:{}", self.pool_id),
                idempotency_key: "activate-pool".into(),
                request_digest: digest('d'),
                occurred_at: Utc::now().to_rfc3339(),
            })
            .unwrap();
    }

    pub(crate) fn create_request(
        &self,
        idempotency_key: &str,
        token_units: i64,
        concurrency_units: i64,
    ) -> CreateMyComputeOfferDraftRequest {
        let common = self.offer_parts(token_units, concurrency_units);
        CreateMyComputeOfferDraftRequest {
            idempotency_key: idempotency_key.into(),
            sku: common.sku,
            model: None,
            runtime: common.runtime,
            resource_profile: common.resource_profile,
            capacity: common.capacity,
            execution_limits: common.execution_limits,
            authorization: common.authorization,
            price_terms: common.price_terms,
            valid_from: self.starts_at.clone(),
            valid_until: self.valid_until.clone(),
            confirm_create: true,
        }
    }

    pub(crate) fn revise_request(
        &self,
        digest: &str,
        version: i64,
        token_units: i64,
        concurrency_units: i64,
    ) -> ReviseMyComputeOfferDraftRequest {
        let common = self.offer_parts(token_units, concurrency_units);
        ReviseMyComputeOfferDraftRequest {
            expected_offer_version: version,
            expected_offer_digest: digest.into(),
            sku: common.sku,
            model: None,
            runtime: common.runtime,
            resource_profile: common.resource_profile,
            capacity: common.capacity,
            execution_limits: common.execution_limits,
            authorization: common.authorization,
            price_terms: common.price_terms,
            valid_from: self.starts_at.clone(),
            valid_until: self.valid_until.clone(),
            confirm_revise: true,
        }
    }

    fn offer_parts(&self, token_units: i64, concurrency_units: i64) -> OfferParts {
        OfferParts {
            sku: ComputeOfferDraftSkuInput {
                sku_id: "llm-chat-consumer-gpu-cn-east".into(),
                task_kind: "llm_chat".into(),
                context_or_shape_bucket: "8k".into(),
                verification_tier: "platform_verified".into(),
                sla_tier: "best_effort".into(),
                delivery_window_class: "scheduled".into(),
            },
            runtime: ComputeRuntimeRef {
                runtime_family: "llama_cpp".into(),
                runtime_version: "1".into(),
                precision: "fp16".into(),
                runner_digest: digest('e'),
                plugin_id: None,
                plugin_version: None,
                plugin_digest: None,
            },
            resource_profile: ComputeOfferDraftResourceProfileInput {
                accelerator_kind: "consumer_gpu".into(),
                accelerator_count: 1,
                vram_bytes: 8 * 1024 * 1024 * 1024,
                ram_bytes: 16 * 1024 * 1024 * 1024,
            },
            capacity: vec![
                ComputeOfferDraftCapacityInput {
                    bucket_id: self.token_bucket_id.clone(),
                    total_units: token_units,
                    reservable_units: token_units,
                },
                ComputeOfferDraftCapacityInput {
                    bucket_id: self.concurrency_bucket_id.clone(),
                    total_units: concurrency_units,
                    reservable_units: concurrency_units,
                },
            ],
            execution_limits: ComputeOfferExecutionLimits {
                max_concurrent_attempts: concurrency_units,
                max_attempt_runtime_seconds: 3600,
            },
            authorization: ComputeOfferDraftAuthorizationInput {
                public: true,
                allowed_account_ids: Vec::new(),
                allowed_project_ids: Vec::new(),
                allowed_data_classes: vec!["public".into()],
            },
            price_terms: ComputeOfferDraftPriceTermsInput {
                pricing_mode: "spot".into(),
                currency: "CNY".into(),
                curve_id: None,
                curve_version: None,
                instrument_id: None,
                components: vec![
                    price_component("tokens", 10, 1000, 800, token_units),
                    price_component("concurrency", 1, 5000, 4000, concurrency_units),
                ],
                fee_rules: Vec::<ComputeFeeRule>::new(),
            },
        }
    }
}

struct OfferParts {
    sku: ComputeOfferDraftSkuInput,
    runtime: ComputeRuntimeRef,
    resource_profile: ComputeOfferDraftResourceProfileInput,
    capacity: Vec<ComputeOfferDraftCapacityInput>,
    execution_limits: ComputeOfferExecutionLimits,
    authorization: ComputeOfferDraftAuthorizationInput,
    price_terms: ComputeOfferDraftPriceTermsInput,
}

fn meter_policy(
    meter: &str,
    meter_mode: &str,
    quantum_units: i64,
) -> CreateMyComputeCapacityMeterPolicyRequest {
    CreateMyComputeCapacityMeterPolicyRequest {
        meter: meter.into(),
        meter_mode: meter_mode.into(),
        quantum_units,
    }
}

fn price_component(
    meter: &str,
    unit_size: i64,
    consumer_unit_price_micros: i64,
    provider_unit_price_micros: i64,
    max_units: i64,
) -> ComputePriceComponent {
    ComputePriceComponent {
        meter: meter.into(),
        unit_size,
        consumer_unit_price_micros,
        provider_unit_price_micros,
        max_units,
    }
}

pub(crate) fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}
