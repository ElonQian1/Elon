use serde_json::{json, Value};
use uuid::Uuid;

use crate::store::Store;

const OWNER_SUBMIT: &str = "compute_submit_my_external_pool_onboarding";
const ADMIN_ONBOARDING_REVIEW: &str = "compute_admin_review_external_pool_onboarding_request";
const ADMIN_ONBOARDING_APPLY: &str = "compute_admin_apply_external_pool_onboarding_request";
const ADMIN_RELEASE_SUBMIT: &str = "compute_admin_submit_external_pool_adapter_release";
const ADMIN_RELEASE_REVIEW: &str = "compute_admin_review_external_pool_adapter_release";
const ADMIN_RELEASE_STAGE: &str = "compute_admin_stage_external_pool_adapter_release";
const ADMIN_CURVE_LIST: &str = "compute_admin_list_platform_reference_price_curves";

#[test]
fn management_definitions_are_partitioned_by_platform_role() {
    let user_tools = super::definitions_for_platform_role("user");
    assert!(has_tool(&user_tools, OWNER_SUBMIT));
    assert!(!user_tools.iter().any(|tool| tool["name"]
        .as_str()
        .is_some_and(|name| name.starts_with("compute_admin_"))));

    let admin_tools = super::definitions_for_platform_role("admin");
    for name in [
        OWNER_SUBMIT,
        ADMIN_ONBOARDING_REVIEW,
        ADMIN_RELEASE_SUBMIT,
        ADMIN_CURVE_LIST,
    ] {
        assert!(has_tool(&admin_tools, name), "missing MCP tool {name}");
    }
    assert_eq!(admin_tools.len(), user_tools.len() + 27);
    assert_eq!(
        tool(&admin_tools, ADMIN_RELEASE_STAGE)["annotations"]["readOnlyHint"],
        false
    );
    assert_eq!(
        tool(
            &user_tools,
            "compute_cancel_my_external_pool_onboarding_request"
        )["annotations"]["destructiveHint"],
        true
    );
}

#[test]
fn direct_admin_calls_reject_non_admin_platform_roles_before_decoding() {
    let (store, path) = temporary_store("role-guard");
    for name in [
        "compute_admin_list_external_pool_onboarding_requests",
        "compute_admin_list_external_pool_adapter_releases",
        ADMIN_CURVE_LIST,
    ] {
        let error = super::call_admin_if_handled(&store, "ordinary-user", "user", name, json!({}))
            .unwrap_err();
        assert!(error.to_string().contains("只有平台管理员"), "{error:#}");
    }
    cleanup(store, path);
}

#[test]
fn onboarding_and_adapter_release_governance_run_through_mcp_dispatch() {
    let (store, path) = temporary_store("governance");

    let submitted = regular_call(
        &store,
        "merchant-owner",
        OWNER_SUBMIT,
        onboarding_submit_body(),
    );
    assert_eq!(submitted["status"], "submitted");
    let request_id = text(&submitted, "request_id");
    let request_digest = text(&submitted, "request_digest");

    let reviewed = admin_call(
        &store,
        "onboarding-reviewer",
        ADMIN_ONBOARDING_REVIEW,
        json!({
            "request_id":request_id,
            "request":{
                "idempotency_key":"review-onboarding-mcp",
                "expected_request_digest":request_digest,
                "decision":"approved",
                "review_reason":null,
                "confirm_review":true
            }
        }),
    );
    assert_eq!(reviewed["decision"], "approved");
    let applied = admin_call(
        &store,
        "onboarding-applier",
        ADMIN_ONBOARDING_APPLY,
        json!({
            "request_id":request_id,
            "request":{
                "idempotency_key":"apply-onboarding-mcp",
                "expected_request_digest":request_digest,
                "expected_review_digest":text(&reviewed,"review_digest"),
                "confirm_application":true
            }
        }),
    );
    assert_eq!(applied["onboarding_effect"], "provider_registered_only");
    assert_eq!(
        store
            .compute_provider_if_exists("external-pool-provider-mcp")
            .unwrap()
            .unwrap()
            .provider
            .status,
        "registering"
    );

    let release = admin_call(
        &store,
        "release-submitter",
        ADMIN_RELEASE_SUBMIT,
        adapter_release_submit_body(),
    );
    assert_eq!(release["status"], "submitted");
    let release_id = text(&release, "request_id");
    let release_review = admin_call(
        &store,
        "release-reviewer",
        ADMIN_RELEASE_REVIEW,
        json!({
            "request_id":release_id,
            "request":{
                "idempotency_key":"review-release-mcp",
                "expected_request_digest":text(&release,"request_digest"),
                "expected_request_material_digest":text(&release,"request_material_digest"),
                "decision":"approved",
                "review_note":null,
                "confirm_review":true
            }
        }),
    );
    let staged = admin_call(
        &store,
        "release-applier",
        ADMIN_RELEASE_STAGE,
        json!({
            "request_id":release_id,
            "request":{
                "idempotency_key":"stage-release-mcp",
                "expected_request_digest":text(&release,"request_digest"),
                "expected_request_material_digest":text(&release,"request_material_digest"),
                "expected_review_digest":text(&release_review,"review_digest"),
                "apply_note":"metadata only",
                "confirm_stage":true
            }
        }),
    );
    assert_eq!(staged["status"], "staged");
    assert_eq!(staged["release_effect"], "staged_admission_only");

    cleanup(store, path);
}

#[tokio::test]
async fn open_commerce_mcp_routes_platform_admin_tools_and_preserves_user_guard() {
    let (store, path) = temporary_store("open-commerce-route");
    let params = json!({"name":ADMIN_CURVE_LIST,"arguments":{"limit":5}});
    let response = crate::open_commerce_mcp::call_tool_for_platform_role(
        &store,
        "project-unused",
        "admin-user",
        "viewer",
        "admin",
        "mcp-test",
        params.clone(),
    )
    .await
    .unwrap();
    assert_eq!(
        response["structuredContent"]["reference_curve_batches"],
        json!([])
    );

    let denied = crate::open_commerce_mcp::call_tool_for_platform_role(
        &store,
        "project-unused",
        "ordinary-user",
        "owner",
        "user",
        "mcp-test",
        params,
    )
    .await
    .unwrap_err();
    assert!(denied.to_string().contains("只有平台管理员"), "{denied:#}");
    cleanup(store, path);
}

fn regular_call(store: &Store, user_id: &str, name: &str, arguments: Value) -> Value {
    super::call_if_handled(store, "project", user_id, name, arguments)
        .unwrap()
        .unwrap_or_else(|| panic!("regular MCP tool not handled: {name}"))
}

fn admin_call(store: &Store, user_id: &str, name: &str, arguments: Value) -> Value {
    super::call_admin_if_handled(store, user_id, "admin", name, arguments)
        .unwrap()
        .unwrap_or_else(|| panic!("admin MCP tool not handled: {name}"))
}

fn onboarding_submit_body() -> Value {
    json!({
        "request_id":"external-pool-request-mcp",
        "idempotency_key":"submit-onboarding-mcp",
        "submitted_at":"2026-08-11T00:00:00.000000000Z",
        "provider_id":"external-pool-provider-mcp",
        "display_name":"External pool MCP",
        "home_region":"cn-east",
        "task_kinds":["llm_inference"],
        "accelerator_kinds":["consumer_gpu"],
        "regions":["cn-east"],
        "allowed_data_classes":["public"],
        "supports_streaming":true,
        "supports_checkpointing":false,
        "declared_hardware_digest":"4".repeat(64),
        "adapter_intent":{
            "expected_adapter_id":"community-external-pool",
            "expected_release_version":"1.0.0",
            "expected_config_revision":1,
            "expected_config_digest":"community-config-v1"
        },
        "credential_intent":{
            "non_bearer_credential_ref":"vault-ref:external-pool-mcp",
            "credential_hint":"server-held credential"
        },
        "external_evidence_ref":"evidence-ref:external-pool-mcp",
        "external_evidence_sha256":"5".repeat(64),
        "owner_note":"metadata only",
        "confirm_submission":true
    })
}

fn adapter_release_submit_body() -> Value {
    let capabilities = [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ]
    .into_iter()
    .map(|capability_id| json!({"capability_id":capability_id,"capability_revision":1}))
    .collect::<Vec<_>>();
    json!({
        "idempotency_key":"submit-release-mcp",
        "adapter_id":"community-external-pool",
        "release_version":"1.0.0",
        "candidate_artifact_ref":"artifact-ref:community-pool-1.0.0",
        "declared_implementation_sha256":"1".repeat(64),
        "supported_capabilities":capabilities,
        "expected_credential_verifier":{
            "verification_kind":"signed_challenge",
            "verifier_id":"community-pool-verifier",
            "verifier_revision":1,
            "verifier_digest":"2".repeat(64)
        },
        "submission_note":"metadata only",
        "confirm_submission":true
    })
}

fn temporary_store(case: &str) -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon_compute_management_mcp_{case}_{}.db",
        Uuid::new_v4().simple()
    ));
    (Store::open(&path).unwrap(), path)
}

fn cleanup(store: Store, path: std::path::PathBuf) {
    drop(store);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

fn has_tool(tools: &[Value], name: &str) -> bool {
    tools.iter().any(|tool| tool["name"] == name)
}

fn tool<'a>(tools: &'a [Value], name: &str) -> &'a Value {
    tools
        .iter()
        .find(|tool| tool["name"] == name)
        .unwrap_or_else(|| panic!("missing MCP tool {name}"))
}

fn text(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("missing string {key}: {value}"))
        .to_string()
}
