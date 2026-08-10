use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use crate::{open_commerce_developer_production_test_support::test_app_state, types::AppState};

use super::{
    api,
    sui_preflight_job_test_support::{fixture, RuntimeFlagGuard},
};

struct ApiFixture {
    _state: Arc<AppState>,
    router: Router,
    project_id: String,
    projection_id: String,
    owner_token: String,
    outsider_token: String,
    adapter_token: String,
}

#[tokio::test]
async fn project_http_requires_membership_confirmation_and_keeps_jobs_isolated() {
    let fixture = api_fixture();
    let jobs_path = format!(
        "/api/projects/{}/economy/sui-preflight-jobs",
        fixture.project_id
    );

    assert_eq!(
        call(&fixture.router, Method::GET, &jobs_path, None, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &jobs_path,
            Some(&fixture.outsider_token),
            Value::Null,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &jobs_path,
            Some(&fixture.owner_token),
            queue_body(&fixture.projection_id, false),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let (queue_status, queued) = call(
        &fixture.router,
        Method::POST,
        &jobs_path,
        Some(&fixture.owner_token),
        queue_body(&fixture.projection_id, true),
    )
    .await;
    assert_eq!(queue_status, StatusCode::OK, "{queued}");
    assert_eq!(queued["status"], "pending");
    let job_id = queued["id"].as_str().unwrap();

    let (list_status, listed) = call(
        &fixture.router,
        Method::GET,
        &jobs_path,
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(list_status, StatusCode::OK, "{listed}");
    assert_eq!(listed["jobs"].as_array().unwrap().len(), 1);
    assert!(!listed.to_string().contains(&fixture.adapter_token));

    let cancel_path = format!("{jobs_path}/{job_id}/cancel");
    let (cancel_status, canceled) = call(
        &fixture.router,
        Method::POST,
        &cancel_path,
        Some(&fixture.owner_token),
        json!({"reason":"owner canceled local preflight","confirmed_by_user":true}),
    )
    .await;
    assert_eq!(cancel_status, StatusCode::OK, "{canceled}");
    assert_eq!(canceled["status"], "canceled");
}

#[tokio::test]
async fn machine_http_authenticates_short_lease_and_completes_idempotently() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = api_fixture();
    let jobs_path = format!(
        "/api/projects/{}/economy/sui-preflight-jobs",
        fixture.project_id
    );
    let (_, queued) = call(
        &fixture.router,
        Method::POST,
        &jobs_path,
        Some(&fixture.owner_token),
        queue_body(&fixture.projection_id, true),
    )
    .await;
    let job_id = queued["id"].as_str().unwrap();
    let claim_path = "/api/economy/sui-preflight/jobs/claim";

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            claim_path,
            None,
            json!({"lease_seconds":300}),
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            claim_path,
            Some("invalid-machine-token"),
            json!({"lease_seconds":300}),
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );

    let (claim_status, claimed) = call(
        &fixture.router,
        Method::POST,
        claim_path,
        Some(&fixture.adapter_token),
        json!({"lease_seconds":60}),
    )
    .await;
    assert_eq!(claim_status, StatusCode::OK, "{claimed}");
    assert_eq!(claimed["claimed"], true);
    assert_eq!(claimed["issue"]["job"]["attempt_no"], 1);
    assert_eq!(claimed["issue"]["lease_token_visible_once"], true);
    assert_eq!(
        claimed["issue"]["handoff"]["constraints"]["signature_present"],
        false
    );
    assert_eq!(
        claimed["issue"]["handoff"]["constraints"]["transaction_broadcast"],
        false
    );
    assert_eq!(
        claimed["issue"]["handoff"]["constraints"]["funds_moved"],
        false
    );
    let first_lease = claimed["issue"]["lease_token"].as_str().unwrap();
    assert!(first_lease.starts_with("sui_preflight_lease_"));
    assert!(!claimed["issue"]["job"].to_string().contains(first_lease));

    let (empty_status, empty) = call(
        &fixture.router,
        Method::POST,
        claim_path,
        Some(&fixture.adapter_token),
        json!({"lease_seconds":300}),
    )
    .await;
    assert_eq!(empty_status, StatusCode::OK, "{empty}");
    assert_eq!(empty["claimed"], false);

    let renew_path = format!("/api/economy/sui-preflight/jobs/{job_id}/renew");
    let (renew_status, renewed) = call(
        &fixture.router,
        Method::POST,
        &renew_path,
        Some(&fixture.adapter_token),
        json!({"lease_token":first_lease,"extend_seconds":300}),
    )
    .await;
    assert_eq!(renew_status, StatusCode::OK, "{renewed}");
    assert_eq!(renewed["renewed"], true);

    let release_path = format!("/api/economy/sui-preflight/jobs/{job_id}/release");
    let (release_status, released) = call(
        &fixture.router,
        Method::POST,
        &release_path,
        Some(&fixture.adapter_token),
        json!({"lease_token":first_lease,"reason":"worker requested retry"}),
    )
    .await;
    assert_eq!(release_status, StatusCode::OK, "{released}");
    assert_eq!(released["job"]["status"], "pending");

    let (_, reclaimed) = call(
        &fixture.router,
        Method::POST,
        claim_path,
        Some(&fixture.adapter_token),
        json!({"lease_seconds":300}),
    )
    .await;
    assert_eq!(reclaimed["issue"]["job"]["attempt_no"], 2);
    let second_lease = reclaimed["issue"]["lease_token"].as_str().unwrap();
    let complete_path = format!("/api/economy/sui-preflight/jobs/{job_id}/complete");
    let completion = json!({
        "lease_token":second_lease,
        "outcome":"passed",
        "summary":"offline package verified",
        "tool_version":"http-test-v1",
        "idempotency_key":"sui-preflight-http-complete-1"
    });
    let (complete_status, completed) = call(
        &fixture.router,
        Method::POST,
        &complete_path,
        Some(&fixture.adapter_token),
        completion.clone(),
    )
    .await;
    assert_eq!(complete_status, StatusCode::OK, "{completed}");
    assert_eq!(completed["job"]["status"], "completed");
    assert_eq!(completed["report"]["outcome"], "passed");
    let report_id = completed["report"]["id"].clone();

    let (repeat_status, repeated) = call(
        &fixture.router,
        Method::POST,
        &complete_path,
        Some(&fixture.adapter_token),
        completion,
    )
    .await;
    assert_eq!(repeat_status, StatusCode::OK, "{repeated}");
    assert_eq!(repeated["report"]["id"], report_id);

    let (_, listed) = call(
        &fixture.router,
        Method::GET,
        &jobs_path,
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(listed["jobs"][0]["status"], "completed");
    assert!(!listed.to_string().contains(second_lease));
}

fn api_fixture() -> ApiFixture {
    let fixture = fixture();
    let project_id = fixture.project_id;
    let projection_id = fixture.projection_id;
    let owner_token = fixture.owner_token;
    let outsider_token = fixture.outsider_token;
    let adapter_token = fixture.adapter_token;
    let state = Arc::new(test_app_state(fixture.store, &std::env::temp_dir()));
    let router = api::routes().with_state(Arc::clone(&state));
    ApiFixture {
        _state: state,
        router,
        project_id,
        projection_id,
        owner_token,
        outsider_token,
        adapter_token,
    }
}

fn queue_body(projection_id: &str, confirmed_by_user: bool) -> Value {
    json!({
        "package_kind":"standard",
        "projection_package_id":projection_id,
        "confirmed_by_user":confirmed_by_user
    })
}

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = if body.is_null() {
        Body::empty()
    } else {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    };
    let response = router
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}
