use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    open_commerce_developer_model::CreateDeveloperAppRequest,
    open_commerce_developer_production_test_support::test_app_state,
    open_commerce_developer_readiness_api::routes, store::Store, types::AppState,
};

struct RouteFixture {
    state: Arc<AppState>,
    router: Router,
    project_id: String,
    other_project_id: String,
    app_record_id: String,
    app_id: String,
    owner_token: String,
    app_owner_token: String,
    admin_token: String,
    other_editor_token: String,
    member_token: String,
    outsider_token: String,
}

#[tokio::test]
async fn readiness_route_enforces_login_project_and_app_management_boundaries() {
    let fixture = route_fixture();
    let path = fixture.path(&fixture.project_id);

    assert_eq!(
        get(&fixture.router, &path, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get(&fixture.router, &path, Some(&fixture.outsider_token))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(&fixture.router, &path, Some(&fixture.member_token))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(&fixture.router, &path, Some(&fixture.other_editor_token))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(
            &fixture.router,
            &fixture.path(&fixture.other_project_id),
            Some(&fixture.owner_token),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    for token in [
        &fixture.app_owner_token,
        &fixture.admin_token,
        &fixture.owner_token,
    ] {
        let (status, body) = get(&fixture.router, &path, Some(token)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(
            body["schema"],
            "open_commerce.developer_production_readiness.v1"
        );
        assert_eq!(body["app_record_id"], fixture.app_record_id);
        assert_eq!(body["app_id"], fixture.app_id);
        assert_eq!(body["next_action_code"], "manifest_not_approved");
        assert_eq!(body["production_invocation_ready"], false);
        assert_eq!(body["production_webhook_ready"], false);
        assert_no_sensitive_fields(&body);
    }
}

#[tokio::test]
async fn readiness_route_hides_app_existence_across_project_boundaries() {
    let fixture = route_fixture();
    let (cross_project_status, cross_project_body) = get(
        &fixture.router,
        &fixture.path(&fixture.other_project_id),
        Some(&fixture.owner_token),
    )
    .await;
    let (missing_status, missing_body) = get(
        &fixture.router,
        &fixture.path_for(&fixture.project_id, "devapp_missing"),
        Some(&fixture.owner_token),
    )
    .await;

    assert_eq!(cross_project_status, StatusCode::FORBIDDEN);
    assert_eq!(missing_status, StatusCode::FORBIDDEN);
    assert_eq!(cross_project_body, missing_body);
    assert!(missing_body.get("schema").is_none());
    assert_no_sensitive_fields(&missing_body);
}

#[tokio::test]
async fn readiness_route_reads_current_app_lifecycle_state_between_requests() {
    let fixture = route_fixture();
    let path = fixture.path(&fixture.project_id);

    fixture
        .state
        .store
        .disable_open_commerce_developer_app(&fixture.project_id, &fixture.app_record_id)
        .unwrap();
    let (disabled_status, disabled_body) =
        get(&fixture.router, &path, Some(&fixture.owner_token)).await;
    assert_eq!(disabled_status, StatusCode::OK, "{disabled_body}");
    assert_eq!(disabled_body["next_action_code"], "app_inactive");
    assert_eq!(disabled_body["blocker_codes"][0], "app_inactive");
    assert_eq!(disabled_body["steps"][0]["code"], "app");
    assert_eq!(disabled_body["steps"][0]["ready"], false);
    assert_eq!(disabled_body["production_invocation_ready"], false);
    assert_eq!(disabled_body["production_webhook_ready"], false);
    assert_no_sensitive_fields(&disabled_body);

    fixture
        .state
        .store
        .reactivate_open_commerce_developer_app(&fixture.project_id, &fixture.app_record_id)
        .unwrap();
    let (reactivated_status, reactivated_body) =
        get(&fixture.router, &path, Some(&fixture.owner_token)).await;
    assert_eq!(reactivated_status, StatusCode::OK, "{reactivated_body}");
    assert_eq!(
        reactivated_body["next_action_code"],
        "manifest_not_approved"
    );
    assert_eq!(reactivated_body["steps"][0]["ready"], true);
    assert_no_sensitive_fields(&reactivated_body);
}

impl RouteFixture {
    fn path(&self, project_id: &str) -> String {
        self.path_for(project_id, &self.app_record_id)
    }

    fn path_for(&self, project_id: &str, app_record_id: &str) -> String {
        format!(
            "/api/projects/{project_id}/open-commerce/developer-apps/{app_record_id}/production-readiness"
        )
    }
}

fn assert_no_sensitive_fields(body: &Value) {
    let serialized = body.to_string();
    for forbidden in [
        "owner_user_id",
        "test_token",
        "live_token",
        "registration_id",
        "challenge_hash",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }
}

async fn get(router: &Router, path: &str, bearer: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().uri(path);
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn route_fixture() -> RouteFixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_readiness_api_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("readiness API test store should open");
    let owner = user(&store, "readiness-owner@example.com");
    let app_owner = user(&store, "readiness-app-owner@example.com");
    let admin = user(&store, "readiness-admin@example.com");
    let other_editor = user(&store, "readiness-other-editor@example.com");
    let member = user(&store, "readiness-member@example.com");
    let outsider = user(&store, "readiness-outsider@example.com");
    let project = store
        .create_project(&owner.id, "Readiness API", None, None)
        .unwrap()
        .project;
    let other_project = store
        .create_project(&owner.id, "Other Readiness API", None, None)
        .unwrap()
        .project;
    for (account, role) in [
        ("readiness-app-owner@example.com", "editor"),
        ("readiness-admin@example.com", "admin"),
        ("readiness-other-editor@example.com", "editor"),
        ("readiness-member@example.com", "member"),
    ] {
        store
            .add_project_member_by_account(&project.id, account, role)
            .unwrap();
    }
    let app = store
        .create_open_commerce_developer_app(
            &project.id,
            &app_owner.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.readiness.api".to_string(),
                display_name: "Readiness API Consumer".to_string(),
            },
        )
        .unwrap()
        .app;
    let owner_token = session(&store, &owner.id);
    let app_owner_token = session(&store, &app_owner.id);
    let admin_token = session(&store, &admin.id);
    let other_editor_token = session(&store, &other_editor.id);
    let member_token = session(&store, &member.id);
    let outsider_token = session(&store, &outsider.id);
    let root = path.parent().unwrap().to_path_buf();
    let state: Arc<AppState> = Arc::new(test_app_state(store, &root));
    let router = routes().with_state(state.clone());

    RouteFixture {
        state,
        router,
        project_id: project.id,
        other_project_id: other_project.id,
        app_record_id: app.id,
        app_id: app.app_id,
        owner_token,
        app_owner_token,
        admin_token,
        other_editor_token,
        member_token,
        outsider_token,
    }
}

fn user(store: &Store, email: &str) -> crate::store::PublicUser {
    store.create_user(email, "secret1", None, None).unwrap()
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("readiness-api-test"), None)
        .unwrap()
        .0
}
