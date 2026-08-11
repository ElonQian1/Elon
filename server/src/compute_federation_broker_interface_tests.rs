use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use crate::{
    compute_federation_broker_service::control_plane_tests::{workload, BrokerFixture},
    open_commerce_developer_production_test_support::test_app_state,
    store::Store,
};

const CREATE_JOB: &str = "compute_create_my_job";
const GET_JOB: &str = "compute_get_my_job";
const RESERVE: &str = "compute_reserve_my_job";
const RELEASE: &str = "compute_release_my_reservation";

#[tokio::test]
async fn broker_http_and_mcp_enforce_identity_project_and_confirmations() {
    let fixture = BrokerFixture::new();
    assert_tool_contracts();

    let outsider = fixture
        .supply
        .store
        .create_user(
            &format!(
                "broker-outsider-{}@example.com",
                uuid::Uuid::new_v4().simple()
            ),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let (consumer_token, _) = fixture
        .supply
        .store
        .create_session(&fixture.consumer_id, Some("broker-interface"), None)
        .unwrap();
    let (outsider_token, _) = fixture
        .supply
        .store
        .create_session(&outsider.id, Some("broker-interface"), None)
        .unwrap();

    let quoted = fixture.create_quoted_job("mcp-interface");
    fixture
        .supply
        .store
        .billing_recharge(
            &fixture.consumer_id,
            100,
            "broker_interface",
            &fixture.supply.admin_id,
            None,
        )
        .unwrap();
    let reserve_request = fixture.reserve_request(&quoted, "mcp-interface", 20, 1);
    let unconfirmed = mcp(
        &fixture,
        &fixture.project_id,
        &fixture.consumer_id,
        RESERVE,
        reserve_arguments(&reserve_request, false),
    );
    assert!(unconfirmed
        .unwrap_err()
        .to_string()
        .contains("明确确认本次算力余额冻结"));
    fixture.assert_capacity(100, 0, 4, 0);

    let reserved = mcp(
        &fixture,
        &fixture.project_id,
        &fixture.consumer_id,
        RESERVE,
        reserve_arguments(&reserve_request, true),
    )
    .unwrap();
    assert_eq!(reserved["status"], "active");
    assert_eq!(reserved["budget_reserved_fen"], 10);

    let finish_arguments = json!({
        "reservation_id":reserved["reservation_id"],
        "idempotency_key":"mcp-interface-release",
        "expected_reservation_revision":reserved["reservation_revision"],
        "expected_reservation_digest":reserved["reservation_digest"],
        "confirm_cancellation":false
    });
    assert!(mcp(
        &fixture,
        &fixture.project_id,
        &fixture.consumer_id,
        RELEASE,
        finish_arguments.clone(),
    )
    .unwrap_err()
    .to_string()
    .contains("明确确认取消"));
    let mut confirmed_finish = finish_arguments;
    confirmed_finish["confirm_cancellation"] = json!(true);
    let released = mcp(
        &fixture,
        &fixture.project_id,
        &fixture.consumer_id,
        RELEASE,
        confirmed_finish.clone(),
    )
    .unwrap();
    assert_eq!(released["status"], "released");
    assert_eq!(released["replayed"], false);
    let replayed = mcp(
        &fixture,
        &fixture.project_id,
        &fixture.consumer_id,
        RELEASE,
        confirmed_finish,
    )
    .unwrap();
    assert_eq!(replayed["replayed"], true);

    let foreign_project = mcp(
        &fixture,
        "other-project",
        &fixture.consumer_id,
        GET_JOB,
        json!({"job_id":quoted.job.job_id}),
    );
    assert!(foreign_project
        .unwrap_err()
        .to_string()
        .contains("不属于当前 MCP 项目"));
    let foreign_consumer = mcp(
        &fixture,
        &fixture.project_id,
        &outsider.id,
        GET_JOB,
        json!({"job_id":quoted.job.job_id}),
    );
    assert!(foreign_consumer
        .unwrap_err()
        .to_string()
        .contains("当前登录用户自己"));

    let state_store = Store::open(&fixture.supply.root.join("state.sqlite")).unwrap();
    let state = Arc::new(test_app_state(state_store, &fixture.supply.root));
    let router = Router::new()
        .merge(crate::compute_federation_broker_api::routes())
        .with_state(state);
    let job_id = format!("http-job-{}", fixture.consumer_id);
    let project_path = format!("/api/projects/{}/compute/jobs", fixture.project_id);

    assert_eq!(
        call_http(
            &router,
            Method::POST,
            &project_path,
            None,
            create_job_body(&job_id),
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call_http(
            &router,
            Method::POST,
            &project_path,
            Some(&outsider_token),
            create_job_body(&job_id),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, created) = call_http(
        &router,
        Method::POST,
        &project_path,
        Some(&consumer_token),
        create_job_body(&job_id),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(created["job"]["status"], "submitted");

    let job_path = format!("/api/me/compute/jobs/{job_id}");
    let (status, own_job) = call_http(
        &router,
        Method::GET,
        &job_path,
        Some(&consumer_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{own_job}");
    let (status, outsider_job) = call_http(
        &router,
        Method::GET,
        &job_path,
        Some(&outsider_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{outsider_job}");
    assert!(outsider_job["error"]
        .as_str()
        .unwrap()
        .contains("当前登录用户自己"));

    assert!(super::call_if_handled(
        &fixture.supply.store,
        &fixture.project_id,
        &fixture.consumer_id,
        "compute_unknown_broker_tool",
        json!({}),
    )
    .unwrap()
    .is_none());
}

fn assert_tool_contracts() {
    let definitions = super::definitions();
    for (name, read_only, destructive) in [
        (CREATE_JOB, false, false),
        (GET_JOB, true, false),
        (RESERVE, false, true),
        (RELEASE, false, true),
    ] {
        let tool = definitions
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("missing MCP tool {name}"));
        assert_eq!(tool["annotations"]["readOnlyHint"], read_only);
        assert_eq!(tool["annotations"]["destructiveHint"], destructive);
    }
    assert_eq!(
        definitions
            .iter()
            .find(|tool| tool["name"] == RESERVE)
            .unwrap()["inputSchema"]["properties"]["confirm_financial_action"]["const"],
        true
    );
}

fn mcp(
    fixture: &BrokerFixture,
    project_id: &str,
    user_id: &str,
    name: &str,
    arguments: Value,
) -> anyhow::Result<Value> {
    super::call_if_handled(&fixture.supply.store, project_id, user_id, name, arguments)?
        .ok_or_else(|| anyhow::anyhow!("MCP tool was not handled: {name}"))
}

fn reserve_arguments(
    request: &crate::compute_federation_broker_service::ReserveMyComputeRequest,
    confirm: bool,
) -> Value {
    json!({
        "reservation_id":request.reservation_id,
        "idempotency_key":request.idempotency_key,
        "job_id":request.job_id,
        "expected_job_revision":request.expected_job_revision,
        "expected_job_digest":request.expected_job_digest,
        "reserved_capacity":request.reserved_capacity,
        "expires_at":request.expires_at,
        "confirm_financial_action":confirm
    })
}

fn create_job_body(job_id: &str) -> Value {
    json!({
        "job_id":job_id,
        "idempotency_key":format!("create-{job_id}"),
        "merchant_id":null,
        "workload":workload(),
        "provider_scope":{
            "allowed_provider_ids":[],
            "allowed_provider_kinds":["user_node"],
            "excluded_provider_ids":[],
            "required_trust_tier":"platform_verified",
            "required_regions":["cn-east"]
        },
        "max_consumer_charge_micros":100_000,
        "currency":"CNY"
    })
}

async fn call_http(
    router: &Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = if body.is_null() {
        Body::empty()
    } else {
        request = request.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    };
    let response = router
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}
