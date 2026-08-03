use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_adapter_claim_model::{
        CompleteAdapterHandoffClaimRequest, ReleaseAdapterHandoffClaimRequest,
        RenewAdapterHandoffClaimRequest, ResumeAdapterHandoffClaimRequest,
    },
    open_commerce_adapter_claim_service, open_commerce_adapter_service,
    open_commerce_integration_model::CreateIntegrationRequest,
    open_commerce_merchant_evidence_model::BUSINESS_RECEIPT_SCHEMA,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_MERCHANT_RUNTIME,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::{OpenCommerceInvocationStart, Store},
};

struct Fixture {
    store: Store,
    user_id: String,
    project_id: String,
    integration_id: String,
    invocation_id: String,
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_adapter_claim_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user(
            "adapter-claim@example.com",
            "secret1",
            Some("Adapter Claim"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Adapter Claim Project", None, None)
        .unwrap()
        .project;
    let actor = owner_actor(&owner.id);
    let merchant = open_commerce_service::create_merchant(
        &store,
        &project.id,
        &actor,
        CreateMerchantRequest {
            display_name: "租约咖啡店".to_string(),
            slug: Some("adapter-claim-cafe".to_string()),
            description: String::new(),
            node_mode: "self_hosted".to_string(),
            public_profile: json!({"category":"coffee"}),
        },
    )
    .unwrap();
    let integration = open_commerce_service::create_integration(
        &store,
        &project.id,
        &actor,
        CreateIntegrationRequest {
            merchant_id: merchant.id.clone(),
            integration_key: "merchant.erp.claim".to_string(),
            provider_key: "merchant_erp".to_string(),
            display_name: "商户 ERP 租约适配器".to_string(),
            connection_mode: "local_adapter".to_string(),
            scopes: vec!["orders.write".to_string()],
            data_domains: vec!["orders".to_string()],
        },
    )
    .unwrap();
    let capability = store
        .create_open_commerce_capability(
            &project.id,
            &merchant.id,
            CreateCapabilityRequest {
                capability_key: "order.commit".to_string(),
                display_name: "提交订单".to_string(),
                description: String::new(),
                kind: "action".to_string(),
                access_level: ACCESS_PUBLIC.to_string(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
                handler_type: HANDLER_MERCHANT_RUNTIME.to_string(),
                handler_config: None,
                unit_price_micros: 1_000,
                currency: "CNY".to_string(),
                freshness_seconds: 0,
            },
        )
        .unwrap();
    let invocation_id = store
        .start_open_commerce_invocation(OpenCommerceInvocationStart {
            project_id: &project.id,
            merchant_id: &merchant.id,
            capability_id: &capability.id,
            capability_key: &capability.capability_key,
            requester_user_id: &owner.id,
            requester_app_id: "consumer.ai",
            grant_id: None,
            idempotency_key: "adapter-claim-order-1",
            request_hash: "adapter-claim-request-hash",
            request_shape: &json!({"keys":[]}),
            unit_price_micros: capability.unit_price_micros,
            currency: &capability.currency,
        })
        .unwrap()
        .invocation
        .id;
    store
        .finish_open_commerce_invocation_success(
            &invocation_id,
            &json!({
                "order":{"id":"merchant-order-claim-1","items":[{"sku":"coffee-1","quantity":2}]},
                "_yilong_business_receipt":{
                    "schema":BUSINESS_RECEIPT_SCHEMA,
                    "entity_type":"order",
                    "reference_id":"merchant-order-claim-1",
                    "state":"accepted",
                    "occurred_at":"2026-08-03T06:00:00Z",
                    "amount_minor":3600,
                    "currency":"CNY"
                }
            }),
        )
        .unwrap();
    Fixture {
        store,
        user_id: owner.id,
        project_id: project.id,
        integration_id: integration.id,
        invocation_id,
    }
}

#[test]
fn task_claim_scope_is_explicit_and_claims_only_one_result() {
    let fixture = fixture();
    let actor = owner_actor(&fixture.user_id);
    let write_only = open_commerce_adapter_service::rotate_credential(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        90,
        false,
        &actor,
    )
    .unwrap();
    assert_eq!(write_only.credential.scopes, vec!["business_handoff.write"]);
    assert!(open_commerce_adapter_claim_service::claim_next(
        &fixture.store,
        &write_only.credential,
        300,
    )
    .unwrap_err()
    .to_string()
    .contains("business_handoff.claim"));

    let enabled = open_commerce_adapter_service::rotate_credential(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        90,
        true,
        &actor,
    )
    .unwrap();
    assert_eq!(
        enabled.credential.scopes,
        vec!["business_handoff.write", "business_handoff.claim"]
    );
    let poll =
        open_commerce_adapter_claim_service::claim_next(&fixture.store, &enabled.credential, 300)
            .unwrap();
    assert!(poll.claimed);
    let issue = poll.issue.unwrap();
    assert_eq!(issue.claim.invocation_id, fixture.invocation_id);
    assert_eq!(issue.claim.attempt_no, 1);
    assert!(issue.lease_token.starts_with("oc_claim_"));
    assert_eq!(issue.task.result["order"]["id"], "merchant-order-claim-1");
    assert!(!serde_json::to_string(&issue.claim)
        .unwrap()
        .contains(&issue.lease_token));

    let second =
        open_commerce_adapter_claim_service::claim_next(&fixture.store, &enabled.credential, 300)
            .unwrap();
    assert!(!second.claimed);
    assert!(second.issue.is_none());
}

#[test]
fn claim_completion_is_atomic_idempotent_and_bound_to_lease_secret() {
    let fixture = fixture();
    let credential = claim_enabled_credential(&fixture);
    let issue = open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300)
        .unwrap()
        .issue
        .unwrap();
    let request = completion("applied", None, Some("erp-order-claim-1"));
    assert!(open_commerce_adapter_claim_service::complete_claim(
        &fixture.store,
        &credential,
        &issue.claim.id,
        CompleteAdapterHandoffClaimRequest {
            lease_token: "oc_claim_invalid_invalid_invalid_invalid_invalid".to_string(),
            ..request.clone()
        },
    )
    .unwrap_err()
    .to_string()
    .contains("租约无效"));

    let receipt = open_commerce_adapter_claim_service::complete_claim(
        &fixture.store,
        &credential,
        &issue.claim.id,
        CompleteAdapterHandoffClaimRequest {
            lease_token: issue.lease_token.clone(),
            ..request.clone()
        },
    )
    .unwrap();
    assert_eq!(
        receipt.adapter_claim_id.as_deref(),
        Some(issue.claim.id.as_str())
    );
    assert_eq!(receipt.status, "applied");
    assert!(!receipt.funds_moved);
    let replay = open_commerce_adapter_claim_service::complete_claim(
        &fixture.store,
        &credential,
        &issue.claim.id,
        CompleteAdapterHandoffClaimRequest {
            lease_token: issue.lease_token,
            ..request
        },
    )
    .unwrap();
    assert_eq!(replay.id, receipt.id);
    assert_eq!(
        fixture
            .store
            .open_commerce_adapter_handoff_claim(&issue.claim.id)
            .unwrap()
            .status,
        "completed"
    );
    assert!(
        !open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300,)
            .unwrap()
            .claimed
    );
}

#[test]
fn expired_and_rejected_claims_are_safely_retryable_but_stale_claims_cannot_finish() {
    let fixture = fixture();
    let credential = claim_enabled_credential(&fixture);
    let first = open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 60)
        .unwrap()
        .issue
        .unwrap();
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_business_handoff_claims
                SET lease_expires_at='2000-01-01T00:00:00Z'
              WHERE id=?1",
            rusqlite::params![first.claim.id],
        )
        .unwrap();
    let second = open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300)
        .unwrap()
        .issue
        .unwrap();
    assert_eq!(second.claim.attempt_no, 2);
    assert!(open_commerce_adapter_claim_service::complete_claim(
        &fixture.store,
        &credential,
        &first.claim.id,
        CompleteAdapterHandoffClaimRequest {
            lease_token: first.lease_token,
            ..completion("applied", None, Some("stale-erp-record"))
        },
    )
    .unwrap_err()
    .to_string()
    .contains("租约无效"));

    open_commerce_adapter_claim_service::complete_claim(
        &fixture.store,
        &credential,
        &second.claim.id,
        CompleteAdapterHandoffClaimRequest {
            lease_token: second.lease_token,
            ..completion("rejected", Some("adapter_failed"), None)
        },
    )
    .unwrap();
    let cooling_down =
        open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300).unwrap();
    assert!(!cooling_down.claimed);
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_business_handoff_claims
                SET retry_not_before='2000-01-01T00:00:00Z'
              WHERE id=?1",
            rusqlite::params![second.claim.id],
        )
        .unwrap();
    let third = open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300)
        .unwrap()
        .issue
        .unwrap();
    assert_eq!(third.claim.attempt_no, 3);

    let actor = owner_actor(&fixture.user_id);
    open_commerce_adapter_service::rotate_credential(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        90,
        true,
        &actor,
    )
    .unwrap();
    assert!(open_commerce_adapter_claim_service::complete_claim(
        &fixture.store,
        &credential,
        &third.claim.id,
        CompleteAdapterHandoffClaimRequest {
            lease_token: third.lease_token,
            ..completion("applied", None, Some("rotated-erp-record"))
        },
    )
    .unwrap_err()
    .to_string()
    .contains("机器凭据已失效"));
}

#[test]
fn active_claim_can_be_released_without_creating_a_receipt_and_then_reclaimed() {
    let fixture = fixture();
    let credential = claim_enabled_credential(&fixture);
    let first = open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300)
        .unwrap()
        .issue
        .unwrap();
    let released = open_commerce_adapter_claim_service::release_claim(
        &fixture.store,
        &credential,
        &first.claim.id,
        ReleaseAdapterHandoffClaimRequest {
            lease_token: first.lease_token.clone(),
            reason_code: "capacity_pressure".to_string(),
        },
    )
    .unwrap();
    assert_eq!(released.claim.status, "released");
    assert_eq!(
        released.claim.release_reason_code.as_deref(),
        Some("capacity_pressure")
    );
    assert!(released.retryable);
    assert!(released.claim.completed_receipt_id.is_none());
    assert!(open_commerce_adapter_claim_service::release_claim(
        &fixture.store,
        &credential,
        &first.claim.id,
        ReleaseAdapterHandoffClaimRequest {
            lease_token: first.lease_token,
            reason_code: "manual_release".to_string(),
        },
    )
    .unwrap_err()
    .to_string()
    .contains("已被释放"));

    let second = open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300)
        .unwrap()
        .issue
        .unwrap();
    assert_eq!(second.claim.invocation_id, first.claim.invocation_id);
    assert_eq!(second.claim.attempt_no, 2);
}

#[test]
fn active_claim_can_be_renewed_but_never_beyond_its_hard_deadline() {
    let fixture = fixture();
    let credential = claim_enabled_credential(&fixture);
    let issue = open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 60)
        .unwrap()
        .issue
        .unwrap();
    let renewed = open_commerce_adapter_claim_service::renew_claim(
        &fixture.store,
        &credential,
        &issue.claim.id,
        RenewAdapterHandoffClaimRequest {
            lease_token: issue.lease_token,
            extend_seconds: 900,
        },
    )
    .unwrap();
    assert!(renewed.renewed);
    assert!(
        chrono::DateTime::parse_from_rfc3339(&renewed.claim.lease_expires_at).unwrap()
            <= chrono::DateTime::parse_from_rfc3339(&renewed.claim.lease_deadline_at).unwrap()
    );
    assert!(
        chrono::DateTime::parse_from_rfc3339(&renewed.claim.lease_expires_at).unwrap()
            > chrono::DateTime::parse_from_rfc3339(&issue.claim.lease_expires_at).unwrap()
    );
}

#[test]
fn repeated_rejections_pause_retry_until_an_editor_explicitly_resumes_it() {
    let fixture = fixture();
    let credential = claim_enabled_credential(&fixture);
    let issue = open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300)
        .unwrap()
        .issue
        .unwrap();
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_business_handoff_claims SET attempt_no=6 WHERE id=?1",
            rusqlite::params![issue.claim.id],
        )
        .unwrap();
    open_commerce_adapter_claim_service::complete_claim(
        &fixture.store,
        &credential,
        &issue.claim.id,
        CompleteAdapterHandoffClaimRequest {
            lease_token: issue.lease_token,
            ..completion("rejected", Some("persistent_external_failure"), None)
        },
    )
    .unwrap();
    let suspended = fixture
        .store
        .open_commerce_adapter_handoff_claim(&issue.claim.id)
        .unwrap();
    assert_eq!(
        suspended.retry_suspension_reason.as_deref(),
        Some("max_rejected_attempts")
    );
    assert!(suspended.retry_suspended_at.is_some());
    assert!(
        !open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300)
            .unwrap()
            .claimed
    );

    let resumed = open_commerce_adapter_claim_service::resume_retry(
        &fixture.store,
        &fixture.project_id,
        &issue.claim.id,
        ResumeAdapterHandoffClaimRequest {
            confirmed_by_user: true,
        },
        &owner_actor(&fixture.user_id),
    )
    .unwrap();
    assert!(resumed.resumed);
    assert!(!resumed.funds_moved);
    assert!(resumed.claim.retry_resumed_at.is_some());
    let next = open_commerce_adapter_claim_service::claim_next(&fixture.store, &credential, 300)
        .unwrap()
        .issue
        .unwrap();
    assert_eq!(next.claim.attempt_no, 7);
}

fn claim_enabled_credential(
    fixture: &Fixture,
) -> crate::open_commerce_adapter_model::OpenCommerceAdapterCredential {
    open_commerce_adapter_service::rotate_credential(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        90,
        true,
        &owner_actor(&fixture.user_id),
    )
    .unwrap()
    .credential
}

fn completion(
    status: &str,
    error_code: Option<&str>,
    target_reference: Option<&str>,
) -> CompleteAdapterHandoffClaimRequest {
    CompleteAdapterHandoffClaimRequest {
        lease_token: String::new(),
        receipt_key: format!("claim-receipt-{status}"),
        status: status.to_string(),
        target_domain: "erp".to_string(),
        target_reference: target_reference.map(str::to_string),
        error_code: error_code.map(str::to_string),
        completed_at: "2026-08-03T06:01:00Z".to_string(),
    }
}

fn owner_actor(user_id: &str) -> OpenCommerceActor<'_> {
    OpenCommerceActor {
        user_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    }
}
