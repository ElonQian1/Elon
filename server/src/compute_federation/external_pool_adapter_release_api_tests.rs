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

struct Fixture {
    state: Arc<crate::types::AppState>,
    router: Router,
    submitter: PublicUser,
    reviewer: PublicUser,
    applier: PublicUser,
    member: PublicUser,
    submitter_token: String,
    reviewer_token: String,
    applier_token: String,
    member_token: String,
    database_path: PathBuf,
}

#[tokio::test]
async fn administrator_api_enforces_auth_four_eyes_and_exact_replay() {
    let fixture = fixture();
    let body = submit_body("submit-v1", "1.0.0", true);

    assert_eq!(
        call(&fixture.router, Method::POST, release_path(), None, &body)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            release_path(),
            Some(&fixture.member_token),
            &body,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let mut actor_injection = body.clone();
    actor_injection["submitted_by_admin_user_id"] = json!(fixture.reviewer.id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            release_path(),
            Some(&fixture.submitter_token),
            &actor_injection,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let (status, submitted) = call(
        &fixture.router,
        Method::POST,
        release_path(),
        Some(&fixture.submitter_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{submitted}");
    assert_eq!(
        submitted["submitted_by_admin_user_id"],
        fixture.submitter.id
    );
    assert_eq!(submitted["status"], "submitted");
    assert_eq!(submitted["replayed"], false);
    assert!(submitted.get("candidate_artifact_ref").is_none());
    assert!(submitted.get("expected_credential_verifier").is_none());

    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        release_path(),
        Some(&fixture.submitter_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["request_id"], submitted["request_id"]);
    assert_eq!(replayed["replayed"], true);

    let request_id = submitted["request_id"].as_str().unwrap();
    let review_path = format!("{}/{request_id}/review", release_path());
    let review = review_body(&submitted, "review-v1", "approved", true);
    let (status, self_review) = call(
        &fixture.router,
        Method::POST,
        &review_path,
        Some(&fixture.submitter_token),
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
    assert_eq!(approved["reviewed_by_admin_user_id"], fixture.reviewer.id);
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

    let stage_path = format!("{}/{request_id}/stage", release_path());
    let mut stage = stage_body(&submitted, &approved, "stage-v1", true);
    stage["expected_review_digest"] = json!("f".repeat(64));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &stage_path,
            Some(&fixture.applier_token),
            &stage,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let stage = stage_body(&submitted, &approved, "stage-v1", true);
    let (status, staged) = call(
        &fixture.router,
        Method::POST,
        &stage_path,
        Some(&fixture.applier_token),
        &stage,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{staged}");
    assert_eq!(staged["applied_by_admin_user_id"], fixture.applier.id);
    assert_eq!(staged["status"], "staged");
    assert_eq!(staged["release_effect"], "staged_admission_only");
    assert_eq!(staged["replayed"], false);

    let (_, staged_replay) = call(
        &fixture.router,
        Method::POST,
        &stage_path,
        Some(&fixture.applier_token),
        &stage,
    )
    .await;
    assert_eq!(staged_replay["admission_id"], staged["admission_id"]);
    assert_eq!(staged_replay["replayed"], true);
    assert_ledger_counts(&fixture, 1, 1, 1);
    fixture.cleanup();
}

#[tokio::test]
async fn administrator_api_rejects_unconfirmed_and_changed_history() {
    let fixture = fixture();
    let unconfirmed = submit_body("submit-v2", "2.0.0", false);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            release_path(),
            Some(&fixture.submitter_token),
            &unconfirmed,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_ledger_counts(&fixture, 0, 0, 0);

    let body = submit_body("submit-v2", "2.0.0", true);
    let (_, submitted) = call(
        &fixture.router,
        Method::POST,
        release_path(),
        Some(&fixture.submitter_token),
        &body,
    )
    .await;
    let mut changed_replay = body.clone();
    changed_replay["candidate_artifact_ref"] = json!("artifact-ref:changed-v2");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            release_path(),
            Some(&fixture.submitter_token),
            &changed_replay,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let request_id = submitted["request_id"].as_str().unwrap();
    let review_path = format!("{}/{request_id}/review", release_path());
    let review = review_body(&submitted, "review-v2", "changes_requested", true);
    let (status, reviewed) = call(
        &fixture.router,
        Method::POST,
        &review_path,
        Some(&fixture.reviewer_token),
        &review,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reviewed}");

    let stage_path = format!("{}/{request_id}/stage", release_path());
    let stage = stage_body(&submitted, &reviewed, "stage-v2", true);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &stage_path,
            Some(&fixture.applier_token),
            &stage,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_ledger_counts(&fixture, 1, 1, 0);
    fixture.cleanup();
}

fn fixture() -> Fixture {
    let database_path = std::env::temp_dir().join(format!(
        "elon_external_pool_adapter_release_api_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&database_path).unwrap();
    let submitter = user(&store, "release-submitter", Some("admin"));
    let reviewer = user(&store, "release-reviewer", Some("admin"));
    let applier = user(&store, "release-applier", Some("admin"));
    let member = user(&store, "release-member", None);
    let submitter_token = session(&store, &submitter.id);
    let reviewer_token = session(&store, &reviewer.id);
    let applier_token = session(&store, &applier.id);
    let member_token = session(&store, &member.id);
    let root = std::env::temp_dir();
    let state = Arc::new(test_app_state(store, &root));
    let router = routes().with_state(state.clone());
    Fixture {
        state,
        router,
        submitter,
        reviewer,
        applier,
        member,
        submitter_token,
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
        .create_session(user_id, Some("external-pool-adapter-release-api"), None)
        .unwrap()
        .0
}

fn release_path() -> &'static str {
    "/api/admin/compute/external-pool-adapter-releases"
}

fn submit_body(idempotency_key: &str, version: &str, confirmed: bool) -> Value {
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
        "idempotency_key":idempotency_key,
        "adapter_id":"community-external-pool",
        "release_version":version,
        "candidate_artifact_ref":format!("artifact-ref:community-pool-{version}"),
        "declared_implementation_sha256":"1".repeat(64),
        "supported_capabilities":capabilities,
        "expected_credential_verifier":{
            "verification_kind":"signed_challenge",
            "verifier_id":"community-pool-verifier",
            "verifier_revision":1,
            "verifier_digest":"2".repeat(64)
        },
        "submission_note":"stage metadata only; execution remains disabled",
        "confirm_submission":confirmed
    })
}

fn review_body(submitted: &Value, idempotency_key: &str, decision: &str, confirmed: bool) -> Value {
    json!({
        "idempotency_key":idempotency_key,
        "expected_request_digest":submitted["request_digest"],
        "expected_request_material_digest":submitted["request_material_digest"],
        "decision":decision,
        "review_note":if decision == "approved" { Value::Null } else { json!("revise candidate") },
        "confirm_review":confirmed
    })
}

fn stage_body(
    submitted: &Value,
    reviewed: &Value,
    idempotency_key: &str,
    confirmed: bool,
) -> Value {
    json!({
        "idempotency_key":idempotency_key,
        "expected_request_digest":submitted["request_digest"],
        "expected_request_material_digest":submitted["request_material_digest"],
        "expected_review_digest":reviewed["review_digest"],
        "apply_note":"staged metadata is not execution authority",
        "confirm_stage":confirmed
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

fn assert_ledger_counts(fixture: &Fixture, requests: i64, reviews: i64, admissions: i64) {
    let connection = fixture.state.store.conn().unwrap();
    for (table, expected) in [
        ("compute_external_pool_adapter_release_requests", requests),
        ("compute_external_pool_adapter_release_reviews", reviews),
        (
            "compute_external_pool_adapter_release_admissions",
            admissions,
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
