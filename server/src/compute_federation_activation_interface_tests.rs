use axum::http::{Method, StatusCode};
use chrono::Utc;
use serde_json::{json, Value};

use crate::{compute_federation::capacity::ComputeCapacityPoolStatus, store::Store};

use super::activation_interface_test_support::{
    call_http, digest, endpoint_json, InterfaceFixture,
};

const LIST: &str = "compute_admin_list_activation_evidence_requests";
const REVIEW: &str = "compute_admin_review_activation_evidence_request";
const PREPARE_PLAN: &str = "compute_admin_prepare_activation_plan";
const PREFLIGHT_PLAN: &str = "compute_admin_preflight_activation_plan";
const APPLY_PLAN: &str = "compute_admin_apply_activation_plan";
const PREPARE_RECOVERY: &str = "compute_admin_prepare_activation_recovery_plan";
const REVIEW_RECOVERY: &str = "compute_admin_review_activation_recovery_plan";
const PREFLIGHT_RECOVERY: &str = "compute_admin_preflight_activation_recovery_plan";
const SUPERSEDE_RECOVERY: &str = "compute_admin_supersede_activation_recovery_plan";
const GET_RECOVERY_APPLICATION: &str = "compute_admin_get_activation_recovery_application";

const ADMIN_TOOLS: [&str; 22] = [
    LIST,
    "compute_admin_preflight_activation_evidence_request",
    REVIEW,
    "compute_admin_supersede_activation_evidence_request",
    "compute_admin_get_activation_plan",
    PREPARE_PLAN,
    PREFLIGHT_PLAN,
    "compute_admin_get_activation_plan_review",
    "compute_admin_review_activation_plan",
    "compute_admin_get_activation_application",
    APPLY_PLAN,
    "compute_admin_get_activation_quarantine",
    "compute_admin_quarantine_activation_application",
    "compute_admin_get_activation_recovery_plan",
    PREPARE_RECOVERY,
    PREFLIGHT_RECOVERY,
    "compute_admin_get_activation_recovery_supersession",
    SUPERSEDE_RECOVERY,
    "compute_admin_get_activation_recovery_review",
    REVIEW_RECOVERY,
    GET_RECOVERY_APPLICATION,
    "compute_admin_apply_activation_recovery_plan",
];

#[tokio::test]
async fn activation_http_and_mcp_share_governed_lifecycle_and_reopen() {
    let fixture = InterfaceFixture::new();
    assert_activation_tool_partition();
    let denied = super::call_admin_if_handled(
        &fixture.state.store,
        &fixture.outsider_id,
        "user",
        LIST,
        json!({"unexpected":true}),
    )
    .unwrap_err();
    assert!(denied.to_string().contains("只有平台管理员"), "{denied:#}");

    assert_eq!(
        call_http(
            &fixture.router,
            Method::GET,
            "/api/admin/compute/activation-evidence-requests",
            None,
            Value::Null,
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call_http(
            &fixture.router,
            Method::GET,
            "/api/admin/compute/activation-evidence-requests",
            Some(&fixture.outsider_token),
            Value::Null,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let submitted = owner_call(
        &fixture,
        "compute_submit_my_activation_evidence_request",
        json!({
            "provider_id":fixture.provider_id,
            "pool_id":fixture.pool_id,
            "idempotency_key":"activation-interface-evidence",
            "node_binding_ref":format!("node-binding://{}", fixture.provider_id),
            "ready_capability_digest":digest('a'),
            "route_proof_digest":digest('b'),
            "hardware_observation_digest":digest('c'),
            "confirm_evidence_submission":true
        }),
    );
    let request_id = text_at(&submitted, "/request/request_id");
    let request_digest = text_at(&submitted, "/request/request_digest");

    let (status, queue) = call_http(
        &fixture.router,
        Method::GET,
        "/api/admin/compute/activation-evidence-requests?status=submitted&limit=20",
        Some(&fixture.admin_one_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{queue}");
    assert_eq!(
        queue["activation_evidence_requests"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let (status, approved) = call_http(
        &fixture.router,
        Method::POST,
        &admin_path(&request_id, "review"),
        Some(&fixture.admin_one_token),
        json!({
            "expected_request_digest":request_digest,
            "decision":"approved",
            "review_note":"interface evidence accepted",
            "confirm_review":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    let approved_digest = text_at(&approved, "/request/request_digest");

    let prepare_arguments = json!({
        "request_id":request_id,
        "request":{
            "idempotency_key":"activation-interface-plan",
            "expected_request_digest":approved_digest,
            "endpoint":endpoint_json(&fixture.provider_id),
            "verified_hardware_digest":digest('d'),
            "trust_tier":"platform_verified",
            "verified_at":Utc::now().to_rfc3339(),
            "confirm_prepare":true
        }
    });
    let plan = admin_call(
        &fixture,
        &fixture.admin_one_id,
        PREPARE_PLAN,
        prepare_arguments.clone(),
    );
    let plan_digest = text_at(&plan, "/plan/plan_digest");
    let replayed = admin_call(
        &fixture,
        &fixture.admin_one_id,
        PREPARE_PLAN,
        prepare_arguments,
    );
    assert_eq!(replayed["replayed"], true);

    let (status, review) = call_http(
        &fixture.router,
        Method::POST,
        &admin_path(&request_id, "activation-plan/review"),
        Some(&fixture.admin_two_token),
        json!({
            "idempotency_key":"activation-interface-plan-review",
            "expected_plan_digest":plan_digest,
            "review_note":"independent interface review",
            "confirm_review":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{review}");

    let preflight = admin_call(
        &fixture,
        &fixture.admin_one_id,
        PREFLIGHT_PLAN,
        json!({"request_id":request_id}),
    );
    assert_eq!(preflight["ready_for_apply"], true, "{preflight}");
    let application = admin_call(
        &fixture,
        &fixture.admin_one_id,
        APPLY_PLAN,
        json!({
            "request_id":request_id,
            "request":{
                "idempotency_key":"activation-interface-apply",
                "expected_plan_digest":plan_digest,
                "confirm_apply":true
            }
        }),
    );
    assert_eq!(application["activation_effect"], "provider_and_pool_active");
    let application_digest = text_at(&application, "/application_digest");

    let (status, quarantine) = call_http(
        &fixture.router,
        Method::POST,
        &admin_path(&request_id, "activation-plan/application/quarantine"),
        Some(&fixture.admin_one_token),
        json!({
            "idempotency_key":"activation-interface-quarantine",
            "expected_application_digest":application_digest,
            "reason":"interface route integrity incident",
            "confirm_quarantine":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{quarantine}");
    let quarantine_digest = text_at(&quarantine, "/quarantine_digest");

    let first_recovery = recovery_prepare(
        &fixture,
        &request_id,
        &quarantine_digest,
        "activation-interface-recovery-one",
    );
    let first_plan_digest = text_at(&first_recovery, "/plan/plan_digest");
    let (status, first_review) = call_http(
        &fixture.router,
        Method::POST,
        &admin_path(&request_id, "activation-recovery-plan/review"),
        Some(&fixture.admin_two_token),
        recovery_review_body(
            "activation-interface-recovery-review-one",
            &first_plan_digest,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{first_review}");
    let superseded = admin_call(
        &fixture,
        &fixture.admin_one_id,
        SUPERSEDE_RECOVERY,
        json!({
            "request_id":request_id,
            "request":{
                "idempotency_key":"activation-interface-recovery-supersede",
                "expected_plan_digest":first_plan_digest,
                "reason":"replace interface recovery evidence",
                "confirm_supersede":true
            }
        }),
    );
    assert_eq!(superseded["recovery_effect"], "plan_superseded");

    let (status, second_recovery) = call_http(
        &fixture.router,
        Method::POST,
        &admin_path(&request_id, "activation-recovery-plan"),
        Some(&fixture.admin_one_token),
        recovery_prepare_body(&quarantine_digest, "activation-interface-recovery-two"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{second_recovery}");
    let second_plan_digest = text_at(&second_recovery, "/plan/plan_digest");
    admin_call(
        &fixture,
        &fixture.admin_two_id,
        REVIEW_RECOVERY,
        json!({
            "request_id":request_id,
            "request":recovery_review_body(
                "activation-interface-recovery-review-two",
                &second_plan_digest
            )
        }),
    );
    let recovery_preflight = admin_call(
        &fixture,
        &fixture.admin_one_id,
        PREFLIGHT_RECOVERY,
        json!({"request_id":request_id}),
    );
    assert_eq!(
        recovery_preflight["ready_for_apply"], true,
        "{recovery_preflight}"
    );

    let (status, recovered) = call_http(
        &fixture.router,
        Method::POST,
        &admin_path(&request_id, "activation-recovery-plan/application"),
        Some(&fixture.admin_one_token),
        json!({
            "idempotency_key":"activation-interface-recovery-apply",
            "expected_plan_digest":second_plan_digest,
            "confirm_apply":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{recovered}");
    assert_eq!(recovered["provider_effect"], "active");
    let recovery_application = admin_call(
        &fixture,
        &fixture.admin_one_id,
        GET_RECOVERY_APPLICATION,
        json!({"request_id":request_id}),
    );
    let recovery_application_digest = text_at(&recovery_application, "/application_digest");

    let provider_id = fixture.provider_id.clone();
    let pool_id = fixture.pool_id.clone();
    let root = fixture.close();
    let reopened = Store::open(&root.join("state.sqlite")).unwrap();
    assert_eq!(
        reopened
            .compute_provider(&provider_id)
            .unwrap()
            .provider
            .status,
        "active"
    );
    assert_eq!(
        reopened.compute_capacity_pool(&pool_id).unwrap().status,
        ComputeCapacityPoolStatus::Active
    );
    assert_eq!(
        reopened
            .compute_activation_recovery_application_for_request(&request_id)
            .unwrap()
            .unwrap()
            .application_digest,
        recovery_application_digest
    );
    assert!(reopened
        .compute_activation_recovery_supersession_for_request(&request_id)
        .unwrap()
        .is_some());
    drop(reopened);
    let _ = std::fs::remove_dir_all(root);
}

fn assert_activation_tool_partition() {
    let user_tools = super::definitions_for_platform_role("user");
    let admin_tools = super::definitions_for_platform_role("admin");
    assert!(has_tool(
        &user_tools,
        "compute_submit_my_activation_evidence_request"
    ));
    for name in ADMIN_TOOLS {
        assert!(
            !has_tool(&user_tools, name),
            "user unexpectedly sees {name}"
        );
        assert!(has_tool(&admin_tools, name), "admin missing {name}");
    }
}

fn owner_call(fixture: &InterfaceFixture, name: &str, arguments: Value) -> Value {
    super::call_if_handled(
        &fixture.state.store,
        "project-unused",
        &fixture.owner_id,
        name,
        arguments,
    )
    .unwrap()
    .unwrap_or_else(|| panic!("owner MCP tool not handled: {name}"))
}

fn admin_call(fixture: &InterfaceFixture, user_id: &str, name: &str, arguments: Value) -> Value {
    super::call_admin_if_handled(&fixture.state.store, user_id, "admin", name, arguments)
        .unwrap()
        .unwrap_or_else(|| panic!("admin MCP tool not handled: {name}"))
}

fn recovery_prepare(
    fixture: &InterfaceFixture,
    request_id: &str,
    quarantine_digest: &str,
    idempotency_key: &str,
) -> Value {
    admin_call(
        fixture,
        &fixture.admin_one_id,
        PREPARE_RECOVERY,
        json!({
            "request_id":request_id,
            "request":recovery_prepare_body(quarantine_digest, idempotency_key)
        }),
    )
}

fn recovery_prepare_body(quarantine_digest: &str, idempotency_key: &str) -> Value {
    json!({
        "idempotency_key":idempotency_key,
        "expected_quarantine_digest":quarantine_digest,
        "verified_hardware_digest":digest('e'),
        "trust_tier":"platform_verified",
        "verified_at":Utc::now().to_rfc3339(),
        "remediation_summary":"route evidence refreshed",
        "evidence_refs":[format!("evidence://{idempotency_key}")],
        "confirm_prepare":true
    })
}

fn recovery_review_body(idempotency_key: &str, plan_digest: &str) -> Value {
    json!({
        "idempotency_key":idempotency_key,
        "expected_plan_digest":plan_digest,
        "review_note":"independent recovery review",
        "confirm_review":true
    })
}

fn admin_path(request_id: &str, suffix: &str) -> String {
    format!("/api/admin/compute/activation-evidence-requests/{request_id}/{suffix}")
}

fn text_at(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing string at {pointer}: {value}"))
        .to_string()
}

fn has_tool(tools: &[Value], name: &str) -> bool {
    tools.iter().any(|tool| tool["name"] == name)
}
