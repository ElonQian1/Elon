use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    open_commerce_developer_manifest_api::routes,
    open_commerce_developer_model::CreateDeveloperAppRequest,
    open_commerce_developer_production_test_support::test_app_state, store::Store, types::AppState,
};

struct ManifestApiFixture {
    state: Arc<AppState>,
    router: Router,
    project_id: String,
    other_project_id: String,
    app_record_id: String,
    manifest_revision: i64,
    owner_token: String,
    app_owner_token: String,
    other_editor_token: String,
    member_token: String,
    outsider_token: String,
    platform_admin_token: String,
}

#[tokio::test]
async fn manifest_update_enforces_app_management_and_revision_boundaries() {
    let fixture = fixture();
    let path = fixture.manifest_path(&fixture.project_id);
    let request = manifest_update(fixture.manifest_revision, "support@example.test");

    assert_eq!(
        call(&fixture.router, Method::POST, &path, None, Some(&request))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    for token in [
        &fixture.outsider_token,
        &fixture.member_token,
        &fixture.other_editor_token,
        &fixture.platform_admin_token,
    ] {
        assert_eq!(
            call(
                &fixture.router,
                Method::POST,
                &path,
                Some(token),
                Some(&request)
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &fixture.manifest_path(&fixture.other_project_id),
            Some(&fixture.owner_token),
            Some(&request),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (updated_status, updated) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.app_owner_token),
        Some(&request),
    )
    .await;
    assert_eq!(updated_status, StatusCode::OK, "{updated}");
    let updated_revision = fixture.manifest_revision + 1;
    assert_eq!(updated["manifest_revision"], updated_revision);
    assert_eq!(updated["manifest_status"], "draft");
    assert_eq!(updated["requested_scopes"], json!(["menu.preview"]));

    let (stale_status, _) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.app_owner_token),
        Some(&request),
    )
    .await;
    assert_eq!(stale_status, StatusCode::BAD_REQUEST);
    let current = fixture
        .state
        .store
        .open_commerce_developer_app_for_project(&fixture.project_id, &fixture.app_record_id)
        .unwrap();
    assert_eq!(current.manifest_revision, updated_revision);
    assert_eq!(
        current.support_email.as_deref(),
        Some("support@example.test")
    );

    let owner_update = manifest_update(updated_revision, "owner@example.test");
    let (owner_status, owner_body) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.owner_token),
        Some(&owner_update),
    )
    .await;
    assert_eq!(owner_status, StatusCode::OK, "{owner_body}");
    assert_eq!(owner_body["manifest_revision"], updated_revision + 1);
}

#[tokio::test]
async fn manifest_submission_and_review_enforce_platform_admin_boundary() {
    let fixture = fixture();
    let path = fixture.manifest_path(&fixture.project_id);
    let (_, updated) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.app_owner_token),
        Some(&manifest_update(
            fixture.manifest_revision,
            "support@example.test",
        )),
    )
    .await;
    let revision = updated["manifest_revision"].as_i64().unwrap();
    fixture
        .state
        .store
        .issue_open_commerce_developer_app_domain_challenge(
            &fixture.project_id,
            &fixture.app_record_id,
            revision,
            "shop.example.test",
            "manifest-api-local-hash",
            &(Utc::now() + Duration::hours(1)).to_rfc3339(),
        )
        .unwrap();
    fixture
        .state
        .store
        .verify_open_commerce_developer_app_domain(
            &fixture.project_id,
            &fixture.app_record_id,
            revision,
        )
        .unwrap();

    let submit_path = format!("{path}/submit");
    let (submit_status, submitted) = call(
        &fixture.router,
        Method::POST,
        &submit_path,
        Some(&fixture.app_owner_token),
        Some(&json!({"expected_manifest_revision": revision})),
    )
    .await;
    assert_eq!(submit_status, StatusCode::OK, "{submitted}");
    assert_eq!(submitted["manifest_status"], "submitted");

    let queue_path = "/api/admin/open-commerce/developer-app-manifests";
    assert_eq!(
        call(&fixture.router, Method::GET, queue_path, None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            queue_path,
            Some(&fixture.owner_token),
            None,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (queue_status, queue) = call(
        &fixture.router,
        Method::GET,
        queue_path,
        Some(&fixture.platform_admin_token),
        None,
    )
    .await;
    assert_eq!(queue_status, StatusCode::OK, "{queue}");
    assert_eq!(queue["apps"].as_array().unwrap().len(), 1);
    assert_eq!(queue["apps"][0]["id"], fixture.app_record_id);

    let review_path = format!(
        "/api/admin/open-commerce/developer-app-manifests/{}/review",
        fixture.app_record_id
    );
    let review = json!({
        "expected_manifest_revision": revision,
        "decision": "approved",
        "note": "manifest API test"
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &review_path,
            Some(&fixture.owner_token),
            Some(&review),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (review_status, reviewed) = call(
        &fixture.router,
        Method::POST,
        &review_path,
        Some(&fixture.platform_admin_token),
        Some(&review),
    )
    .await;
    assert_eq!(review_status, StatusCode::OK, "{reviewed}");
    assert_eq!(reviewed["manifest_status"], "approved");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &review_path,
            Some(&fixture.platform_admin_token),
            Some(&review),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

impl ManifestApiFixture {
    fn manifest_path(&self, project_id: &str) -> String {
        format!(
            "/api/projects/{project_id}/open-commerce/developer-apps/{}/manifest",
            self.app_record_id
        )
    }
}

fn manifest_update(revision: i64, support_email: &str) -> Value {
    json!({
        "expected_manifest_revision": revision,
        "homepage_url": "https://shop.example.test/app",
        "privacy_policy_url": "https://shop.example.test/privacy",
        "terms_url": "https://shop.example.test/terms",
        "support_email": support_email,
        "requested_scopes": ["menu.preview"]
    })
}

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = if let Some(value) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(value.to_string())
    } else {
        Body::empty()
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

fn fixture() -> ManifestApiFixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_manifest_api_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("manifest API test store should open");
    let owner = user(&store, "manifest-owner@example.com", None);
    let app_owner = user(&store, "manifest-app-owner@example.com", None);
    let other_editor = user(&store, "manifest-other-editor@example.com", None);
    let member = user(&store, "manifest-member@example.com", None);
    let outsider = user(&store, "manifest-outsider@example.com", None);
    let platform_admin = user(&store, "manifest-admin@example.com", Some("admin"));
    let project = store
        .create_project(&owner.id, "Manifest API", None, None)
        .unwrap()
        .project;
    let other_project = store
        .create_project(&owner.id, "Other Manifest API", None, None)
        .unwrap()
        .project;
    for (account, role) in [
        ("manifest-app-owner@example.com", "editor"),
        ("manifest-other-editor@example.com", "editor"),
        ("manifest-member@example.com", "member"),
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
                app_id: "consumer.manifest.api".to_string(),
                display_name: "Manifest API Consumer".to_string(),
            },
        )
        .unwrap()
        .app;
    let owner_token = session(&store, &owner.id);
    let app_owner_token = session(&store, &app_owner.id);
    let other_editor_token = session(&store, &other_editor.id);
    let member_token = session(&store, &member.id);
    let outsider_token = session(&store, &outsider.id);
    let platform_admin_token = session(&store, &platform_admin.id);
    let root = path.parent().unwrap().to_path_buf();
    let state = Arc::new(test_app_state(store, &root));
    let router = routes().with_state(state.clone());

    ManifestApiFixture {
        state,
        router,
        project_id: project.id,
        other_project_id: other_project.id,
        app_record_id: app.id,
        manifest_revision: app.manifest_revision,
        owner_token,
        app_owner_token,
        other_editor_token,
        member_token,
        outsider_token,
        platform_admin_token,
    }
}

fn user(store: &Store, email: &str, role: Option<&str>) -> crate::store::PublicUser {
    store.create_user(email, "secret1", None, role).unwrap()
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("manifest-api-test"), None)
        .unwrap()
        .0
}
