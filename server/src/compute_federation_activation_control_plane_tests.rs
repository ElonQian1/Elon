use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    compute_federation::{
        capacity::ComputeCapacityPoolStatus,
        provider::{ComputeProviderEndpointRef, PROVIDER_STATUS_ACTIVE},
    },
    compute_federation_activation_application_service::{self, ApplyComputeActivationPlanBody},
    compute_federation_activation_plan_review_service::{self, ReviewComputeActivationPlanBody},
    compute_federation_activation_plan_service::{self, PrepareComputeActivationPlanBody},
    compute_federation_activation_quarantine_service::{
        self, QuarantineComputeActivationApplicationBody,
    },
    compute_federation_activation_recovery_service::{
        self, ApplyActivationRecoveryPlanBody, PrepareActivationRecoveryPlanBody,
        ReviewActivationRecoveryPlanBody, SupersedeActivationRecoveryPlanBody,
    },
    compute_federation_activation_service::{
        self, ReviewComputeActivationEvidenceRequestBody, SubmitMyComputeActivationEvidenceRequest,
    },
    compute_federation_capacity_bucket_service::{self, CreateMyComputeCapacityBucketRequest},
    compute_federation_capacity_pool_service::{
        self, CreateMyComputeCapacityMeterPolicyRequest, CreateMyComputeCapacityPoolRequest,
    },
    compute_federation_provider_service::{self, CreateMyComputeProviderRequest},
    store::Store,
};

#[test]
fn activation_quarantine_and_recovery_are_auditable_end_to_end() {
    let fixture = Fixture::new();
    fixture.register_supply_contract();

    let submitted = compute_federation_activation_service::submit_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        SubmitMyComputeActivationEvidenceRequest {
            idempotency_key: "activation-evidence-v1".into(),
            node_binding_ref: format!("node-binding://{}", fixture.provider_id),
            ready_capability_digest: digest('a'),
            route_proof_digest: digest('b'),
            hardware_observation_digest: digest('c'),
            confirm_evidence_submission: true,
        },
    )
    .unwrap();
    assert_eq!(submitted.request.status, "submitted");
    assert_eq!(submitted.activation_effect, "none");

    let approved = compute_federation_activation_service::review(
        &fixture.store,
        &fixture.admin_one,
        &submitted.request.request_id,
        ReviewComputeActivationEvidenceRequestBody {
            expected_request_digest: submitted.request.request_digest.clone(),
            decision: "approved".into(),
            review_note: Some("evidence accepted".into()),
            confirm_review: true,
        },
    )
    .unwrap();
    assert_eq!(approved.request.status, "approved");
    assert_eq!(approved.activation_effect, "none");

    let plan = compute_federation_activation_plan_service::prepare_for_review(
        &fixture.store,
        &fixture.admin_one,
        &submitted.request.request_id,
        PrepareComputeActivationPlanBody {
            idempotency_key: "activation-plan-v1".into(),
            expected_request_digest: approved.request.request_digest.clone(),
            endpoint: endpoint(&fixture.provider_id),
            verified_hardware_digest: digest('d'),
            trust_tier: "platform_verified".into(),
            verified_at: Utc::now().to_rfc3339(),
            confirm_prepare: true,
        },
    )
    .unwrap();
    assert_eq!(plan.plan.status, "prepared");
    assert_eq!(plan.activation_effect, "none");

    let self_review = compute_federation_activation_plan_review_service::review_for_admin(
        &fixture.store,
        &fixture.admin_one,
        &submitted.request.request_id,
        ReviewComputeActivationPlanBody {
            idempotency_key: "activation-self-review".into(),
            expected_plan_digest: plan.plan.plan_digest.clone(),
            review_note: None,
            confirm_review: true,
        },
    );
    assert!(self_review
        .unwrap_err()
        .to_string()
        .contains("准备人不能复核自己准备的计划"));

    let review = compute_federation_activation_plan_review_service::review_for_admin(
        &fixture.store,
        &fixture.admin_two,
        &submitted.request.request_id,
        ReviewComputeActivationPlanBody {
            idempotency_key: "activation-review-v1".into(),
            expected_plan_digest: plan.plan.plan_digest.clone(),
            review_note: Some("independent review".into()),
            confirm_review: true,
        },
    )
    .unwrap();
    assert_ne!(review.prepared_by_user_id, review.reviewed_by_user_id);

    let preflight = compute_federation_activation_plan_service::preflight_for_review(
        &fixture.store,
        &submitted.request.request_id,
    )
    .unwrap();
    assert!(preflight.ready_for_apply, "{:?}", preflight.blockers);

    let application = compute_federation_activation_application_service::apply_for_review(
        &fixture.store,
        &fixture.admin_one,
        &submitted.request.request_id,
        ApplyComputeActivationPlanBody {
            idempotency_key: "activation-apply-v1".into(),
            expected_plan_digest: plan.plan.plan_digest.clone(),
            confirm_apply: true,
        },
    )
    .unwrap();
    assert_eq!(application.activation_effect, "provider_and_pool_active");
    assert_eq!(application.offer_effect, "none");
    fixture.assert_current_state(PROVIDER_STATUS_ACTIVE, ComputeCapacityPoolStatus::Active, 2);

    let quarantine = compute_federation_activation_quarantine_service::quarantine_for_review(
        &fixture.store,
        &fixture.admin_one,
        &submitted.request.request_id,
        QuarantineComputeActivationApplicationBody {
            idempotency_key: "activation-quarantine-v1".into(),
            expected_application_digest: application.application_digest.clone(),
            reason: "route integrity incident".into(),
            confirm_quarantine: true,
        },
    )
    .unwrap();
    assert_eq!(quarantine.provider_effect, "quarantined");
    assert_eq!(quarantine.pool_effect, "quarantined");
    fixture.assert_current_state("quarantined", ComputeCapacityPoolStatus::Quarantined, 3);

    let first_recovery = fixture.prepare_recovery(
        &submitted.request.request_id,
        &quarantine.quarantine_digest,
        "recovery-plan-v1",
    );
    let first_review = fixture.review_recovery(
        &submitted.request.request_id,
        &first_recovery.plan.plan_digest,
        "recovery-review-v1",
    );
    assert_ne!(
        first_review.prepared_by_user_id,
        first_review.reviewed_by_user_id
    );
    let superseded = compute_federation_activation_recovery_service::supersede(
        &fixture.store,
        &fixture.admin_one,
        &submitted.request.request_id,
        SupersedeActivationRecoveryPlanBody {
            idempotency_key: "recovery-supersede-v1".into(),
            expected_plan_digest: first_recovery.plan.plan_digest,
            reason: "replace remediation evidence".into(),
            confirm_supersede: true,
        },
    )
    .unwrap();
    assert_eq!(superseded.recovery_effect, "plan_superseded");
    assert_eq!(superseded.money_effect, "none");
    fixture.assert_current_state("quarantined", ComputeCapacityPoolStatus::Quarantined, 3);

    let second_recovery = fixture.prepare_recovery(
        &submitted.request.request_id,
        &quarantine.quarantine_digest,
        "recovery-plan-v2",
    );
    let missing_review = compute_federation_activation_recovery_service::preflight(
        &fixture.store,
        &submitted.request.request_id,
    )
    .unwrap();
    assert!(!missing_review.ready_for_apply);
    assert!(missing_review
        .blockers
        .iter()
        .any(|blocker| blocker == "plan_review_missing"));

    fixture.review_recovery(
        &submitted.request.request_id,
        &second_recovery.plan.plan_digest,
        "recovery-review-v2",
    );
    let recovery_preflight = compute_federation_activation_recovery_service::preflight(
        &fixture.store,
        &submitted.request.request_id,
    )
    .unwrap();
    assert!(
        recovery_preflight.ready_for_apply,
        "{:?}",
        recovery_preflight.blockers
    );
    assert_eq!(recovery_preflight.active_offer_count, 0);

    let recovered = compute_federation_activation_recovery_service::apply(
        &fixture.store,
        &fixture.admin_one,
        &submitted.request.request_id,
        ApplyActivationRecoveryPlanBody {
            idempotency_key: "recovery-apply-v2".into(),
            expected_plan_digest: second_recovery.plan.plan_digest,
            confirm_apply: true,
        },
    )
    .unwrap();
    assert_eq!(recovered.provider_effect, "active");
    assert_eq!(recovered.pool_effect, "active");
    assert_eq!(recovered.offer_effect, "none_active_offers_required");
    assert_eq!(recovered.node_effect, "none");
    assert_eq!(recovered.money_effect, "none");
    fixture.assert_current_state(PROVIDER_STATUS_ACTIVE, ComputeCapacityPoolStatus::Active, 4);

    let persisted = fixture
        .store
        .compute_activation_evidence_request(&submitted.request.request_id)
        .unwrap();
    assert_eq!(persisted.status, "activated");
    assert!(
        compute_federation_activation_recovery_service::get_supersession(
            &fixture.store,
            &submitted.request.request_id,
        )
        .unwrap()
        .is_some()
    );
}

struct Fixture {
    store: Store,
    owner_id: String,
    admin_one: String,
    admin_two: String,
    provider_id: String,
    pool_id: String,
    window_id: String,
    starts_at_utc: String,
    ends_at_utc: String,
}

impl Fixture {
    fn new() -> Self {
        let suffix = Uuid::new_v4().simple().to_string();
        let root = std::env::temp_dir().join(format!("elon-activation-control-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        Self {
            store: Store::open(&root.join("state.sqlite")).unwrap(),
            owner_id: format!("merchant-{suffix}"),
            admin_one: format!("activation-admin-one-{suffix}"),
            admin_two: format!("activation-admin-two-{suffix}"),
            provider_id: format!("provider-{suffix}"),
            pool_id: format!("pool-{suffix}"),
            window_id: format!("window-{suffix}"),
            starts_at_utc: (Utc::now() + Duration::hours(1)).to_rfc3339(),
            ends_at_utc: (Utc::now() + Duration::hours(3)).to_rfc3339(),
        }
    }

    fn register_supply_contract(&self) {
        compute_federation_provider_service::create_for_user(
            &self.store,
            &self.owner_id,
            CreateMyComputeProviderRequest {
                provider_id: self.provider_id.clone(),
                provider_kind: "user_node".into(),
                display_name: "Activation test provider".into(),
                home_region: Some("cn-east".into()),
                task_kinds: vec!["llm_inference".into()],
                accelerator_kinds: vec!["consumer_gpu".into()],
                regions: vec!["cn-east".into()],
                allowed_data_classes: vec!["public".into()],
                supports_streaming: true,
                supports_checkpointing: false,
                declared_hardware_digest: Some(digest('0')),
            },
        )
        .unwrap();
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
        for (suffix, meter) in [("tokens", "tokens"), ("concurrency", "concurrency")] {
            compute_federation_capacity_bucket_service::create_for_user(
                &self.store,
                &self.owner_id,
                &self.provider_id,
                &self.pool_id,
                CreateMyComputeCapacityBucketRequest {
                    bucket_id: format!("bucket-{suffix}-{}", self.provider_id),
                    window_id: self.window_id.clone(),
                    starts_at_utc: self.starts_at_utc.clone(),
                    ends_at_utc: self.ends_at_utc.clone(),
                    meter: meter.into(),
                },
            )
            .unwrap();
        }
    }

    fn prepare_recovery(
        &self,
        request_id: &str,
        quarantine_digest: &str,
        idempotency_key: &str,
    ) -> crate::compute_federation_activation_recovery_model::ComputeActivationRecoveryPlanReceipt
    {
        compute_federation_activation_recovery_service::prepare(
            &self.store,
            &self.admin_one,
            request_id,
            PrepareActivationRecoveryPlanBody {
                idempotency_key: idempotency_key.into(),
                expected_quarantine_digest: quarantine_digest.into(),
                endpoint: None,
                adapter: None,
                verified_hardware_digest: digest('e'),
                trust_tier: "platform_verified".into(),
                verified_at: Utc::now().to_rfc3339(),
                remediation_summary: "route evidence refreshed".into(),
                evidence_refs: vec![format!("evidence://{idempotency_key}")],
                confirm_prepare: true,
            },
        )
        .unwrap()
    }

    fn review_recovery(
        &self,
        request_id: &str,
        plan_digest: &str,
        idempotency_key: &str,
    ) -> crate::compute_federation_activation_recovery_model::ComputeActivationRecoveryReviewReceipt
    {
        compute_federation_activation_recovery_service::review(
            &self.store,
            &self.admin_two,
            request_id,
            ReviewActivationRecoveryPlanBody {
                idempotency_key: idempotency_key.into(),
                expected_plan_digest: plan_digest.into(),
                review_note: Some("independent recovery review".into()),
                confirm_review: true,
            },
        )
        .unwrap()
    }

    fn assert_current_state(
        &self,
        provider_status: &str,
        pool_status: ComputeCapacityPoolStatus,
        provider_revision: i64,
    ) {
        let provider = self.store.compute_provider(&self.provider_id).unwrap();
        let pool = self.store.compute_capacity_pool(&self.pool_id).unwrap();
        assert_eq!(provider.provider.status, provider_status);
        assert_eq!(provider.provider.policy_revision, provider_revision);
        assert_eq!(pool.status, pool_status);
    }
}

fn endpoint(provider_id: &str) -> ComputeProviderEndpointRef {
    ComputeProviderEndpointRef {
        endpoint_id: format!("endpoint-{provider_id}"),
        transport: "https".into(),
        address_hint: Some("provider.test.invalid".into()),
        gateway_id: Some("gateway-test".into()),
        credential_ref: Some("vault://activation-test".into()),
    }
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

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}
