use std::{path::PathBuf, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    open_commerce_developer_production_test_support::test_app_state,
    store::{PublicUser, Store},
};

use super::routes;

#[path = "external_pool_onboarding_api_tests/management.rs"]
mod management;

struct Fixture {
    state: Arc<crate::types::AppState>,
    router: Router,
    owner: PublicUser,
    reviewer: PublicUser,
    applier: PublicUser,
    owner_token: String,
    reviewer_token: String,
    applier_token: String,
    member_token: String,
    database_path: PathBuf,
}

#[tokio::test]
async fn onboarding_api_derives_owner_and_registers_provider_after_four_eyes() {
    let fixture = fixture();
    let body = submit_body("success", true);

    assert_eq!(
        call(&fixture.router, Method::POST, owner_path(), None, &body)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let mut actor_injection = body.clone();
    actor_injection["requested_by_owner_user_id"] = json!(fixture.reviewer.id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            owner_path(),
            Some(&fixture.owner_token),
            &actor_injection,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let (status, submitted) = call(
        &fixture.router,
        Method::POST,
        owner_path(),
        Some(&fixture.owner_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{submitted}");
    assert_eq!(submitted["provider_owner_account_id"], fixture.owner.id);
    assert_eq!(submitted["status"], "submitted");
    assert_eq!(submitted["credential_ref_present"], true);
    assert!(submitted.get("non_bearer_credential_ref").is_none());
    assert_eq!(submitted["replayed"], false);

    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        owner_path(),
        Some(&fixture.owner_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["request_id"], submitted["request_id"]);
    assert_eq!(replayed["replayed"], true);

    let request_id = submitted["request_id"].as_str().unwrap();
    let review_path = format!("{}/{request_id}/review", admin_path());
    let review = review_body(&submitted, "review-success", "approved", true);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &review_path,
            Some(&fixture.member_token),
            &review,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, self_review) = call(
        &fixture.router,
        Method::POST,
        &review_path,
        Some(&fixture.owner_token),
        &review,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{self_review}");
    assert!(error(&self_review).contains("cannot review"));

    let (status, approved) = call(
        &fixture.router,
        Method::POST,
        &review_path,
        Some(&fixture.reviewer_token),
        &review,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    assert_eq!(approved["reviewed_by_user_id"], fixture.reviewer.id);
    assert_eq!(approved["decision"], "approved");
    assert_eq!(approved["replayed"], false);

    let (_, approved_replay) = call(
        &fixture.router,
        Method::POST,
        &review_path,
        Some(&fixture.reviewer_token),
        &review,
    )
    .await;
    assert_eq!(approved_replay["review_id"], approved["review_id"]);
    assert_eq!(approved_replay["replayed"], true);

    let application_path = format!("{}/{request_id}/application", admin_path());
    let mut application = application_body(&submitted, &approved, "apply-success", true);
    application["expected_review_digest"] = json!("f".repeat(64));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &application_path,
            Some(&fixture.applier_token),
            &application,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let application = application_body(&submitted, &approved, "apply-success", true);
    let (status, applied) = call(
        &fixture.router,
        Method::POST,
        &application_path,
        Some(&fixture.applier_token),
        &application,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["approved_by_user_id"], fixture.owner.id);
    assert_eq!(applied["reviewed_by_user_id"], fixture.reviewer.id);
    assert_eq!(applied["applied_by_user_id"], fixture.applier.id);
    assert_eq!(applied["onboarding_effect"], "provider_registered_only");
    assert_eq!(applied["replayed"], false);

    let (_, applied_replay) = call(
        &fixture.router,
        Method::POST,
        &application_path,
        Some(&fixture.applier_token),
        &application,
    )
    .await;
    assert_eq!(applied_replay["application_id"], applied["application_id"]);
    assert_eq!(applied_replay["replayed"], true);

    let provider = fixture
        .state
        .store
        .compute_provider(submitted["provider_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(provider.provider.owner_account_id, fixture.owner.id);
    assert_eq!(provider.provider.provider_kind, "external_pool");
    assert_eq!(provider.provider.status, "registering");
    assert_eq!(provider.provider.trust_tier, "self_declared");
    assert_ledger_counts(&fixture, 1, 1, 1);
    fixture.cleanup();
}

#[tokio::test]
async fn onboarding_api_rejects_unconfirmed_changed_and_nonapproved_application() {
    let fixture = fixture();
    let unconfirmed = submit_body("revise", false);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            owner_path(),
            Some(&fixture.owner_token),
            &unconfirmed,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_ledger_counts(&fixture, 0, 0, 0);

    let body = submit_body("revise", true);
    let (_, submitted) = call(
        &fixture.router,
        Method::POST,
        owner_path(),
        Some(&fixture.owner_token),
        &body,
    )
    .await;
    let mut changed_replay = body.clone();
    changed_replay["display_name"] = json!("Changed external pool");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            owner_path(),
            Some(&fixture.owner_token),
            &changed_replay,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let request_id = submitted["request_id"].as_str().unwrap();
    let review_path = format!("{}/{request_id}/review", admin_path());
    let review = review_body(&submitted, "review-revise", "changes_requested", true);
    let (status, reviewed) = call(
        &fixture.router,
        Method::POST,
        &review_path,
        Some(&fixture.reviewer_token),
        &review,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reviewed}");

    let application_path = format!("{}/{request_id}/application", admin_path());
    let application = application_body(&submitted, &reviewed, "apply-revise", true);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &application_path,
            Some(&fixture.applier_token),
            &application,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_ledger_counts(&fixture, 1, 1, 0);
    assert!(fixture
        .state
        .store
        .compute_provider_if_exists(submitted["provider_id"].as_str().unwrap())
        .unwrap()
        .is_none());
    fixture.cleanup();
}

fn fixture() -> Fixture {
    let database_path = std::env::temp_dir().join(format!(
        "elon_external_pool_onboarding_api_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&database_path).unwrap();
    let owner = user(&store, "external-owner", Some("admin"));
    let reviewer = user(&store, "external-reviewer", Some("admin"));
    let applier = user(&store, "external-applier", Some("admin"));
    let member = user(&store, "external-member", None);
    let owner_token = session(&store, &owner.id);
    let reviewer_token = session(&store, &reviewer.id);
    let applier_token = session(&store, &applier.id);
    let member_token = session(&store, &member.id);
    let root = std::env::temp_dir();
    let state = Arc::new(test_app_state(store, &root));
    let router = routes().with_state(state.clone());
    Fixture {
        state,
        router,
        owner,
        reviewer,
        applier,
        owner_token,
        reviewer_token,
        applier_token,
        member_token,
        database_path,
    }
}

fn user(store: &Store, prefix: &str, role: Option<&str>) -> PublicUser {
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
        .create_session(user_id, Some("external-pool-onboarding-api"), None)
        .unwrap()
        .0
}

fn owner_path() -> &'static str {
    "/api/me/compute/external-pool-onboarding-requests"
}

fn admin_path() -> &'static str {
    "/api/admin/compute/external-pool-onboarding-requests"
}

fn submit_body(case: &str, confirmed: bool) -> Value {
    json!({
        "request_id":format!("external-pool-request-{case}"),
        "idempotency_key":format!("onboard-{case}"),
        "submitted_at":"2026-08-11T00:00:00.000000000Z",
        "provider_id":format!("external-pool-provider-{case}"),
        "display_name":format!("External pool {case}"),
        "home_region":"cn-east",
        "task_kinds":["llm_inference","image_generation"],
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
            "non_bearer_credential_ref":format!("vault-ref:external-pool-{case}"),
            "credential_hint":"server-held credential"
        },
        "external_evidence_ref":format!("evidence-ref:external-pool-{case}"),
        "external_evidence_sha256":"5".repeat(64),
        "owner_note":"register metadata only; do not grant route authority",
        "confirm_submission":confirmed
    })
}

fn review_body(submitted: &Value, idempotency_key: &str, decision: &str, confirmed: bool) -> Value {
    json!({
        "idempotency_key":idempotency_key,
        "expected_request_digest":submitted["request_digest"],
        "decision":decision,
        "review_reason":if decision == "approved" { Value::Null } else { json!("revise declaration") },
        "confirm_review":confirmed
    })
}

fn application_body(
    submitted: &Value,
    reviewed: &Value,
    idempotency_key: &str,
    confirmed: bool,
) -> Value {
    json!({
        "idempotency_key":idempotency_key,
        "expected_request_digest":submitted["request_digest"],
        "expected_review_digest":reviewed["review_digest"],
        "confirm_application":confirmed
    })
}

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    body: &Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn error(body: &Value) -> &str {
    body["error"].as_str().unwrap_or_default()
}

fn assert_ledger_counts(fixture: &Fixture, requests: i64, reviews: i64, applications: i64) {
    let connection = fixture.state.store.conn().unwrap();
    for (table, expected) in [
        ("compute_external_pool_onboarding_requests", requests),
        ("compute_external_pool_onboarding_reviews", reviews),
        (
            "compute_external_pool_onboarding_applications",
            applications,
        ),
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, expected, "unexpected rows in {table}");
    }
}

impl Fixture {
    fn cleanup(self) {
        drop(self.router);
        drop(self.state);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", self.database_path.display(), suffix));
        }
    }
}
