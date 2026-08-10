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
    open_commerce_developer_admission_api::routes,
    open_commerce_developer_manifest_service,
    open_commerce_developer_model::{
        CreateDeveloperAppRequest, ReviewDeveloperAppManifestRequest,
        UpdateDeveloperAppManifestRequest,
    },
    open_commerce_developer_production_test_support::test_app_state,
    open_commerce_service::OpenCommerceActor,
    store::Store,
    types::AppState,
};

struct AdmissionApiFixture {
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
async fn admission_access_requires_app_management_authority() {
    let fixture = fixture();
    let path = fixture.admission_path(&fixture.project_id);

    assert_eq!(
        call(&fixture.router, Method::GET, &path, None, None)
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
        let (status, body) = call(&fixture.router, Method::GET, &path, Some(token), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert!(!body.to_string().contains("TEST-REG-ADMISSION"));
    }
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &fixture.admission_path(&fixture.other_project_id),
            Some(&fixture.owner_token),
            None,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    for token in [&fixture.app_owner_token, &fixture.owner_token] {
        let (status, body) = call(&fixture.router, Method::GET, &path, Some(token), None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["admission"].is_null());
        assert_eq!(body["production_credential_issued"], false);
        assert_eq!(body["network_access_enabled"], false);
    }
}

#[tokio::test]
async fn admission_submission_and_review_preserve_state_boundaries() {
    let fixture = fixture();
    let path = format!("{}/submit", fixture.admission_path(&fixture.project_id));
    let request = admission_request(fixture.manifest_revision, false);

    for token in [
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
                Some(&request),
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    assert!(fixture
        .state
        .store
        .open_commerce_developer_app_admission(&fixture.app_record_id)
        .unwrap()
        .is_none());

    let (unattested_status, _) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.app_owner_token),
        Some(&request),
    )
    .await;
    assert_eq!(unattested_status, StatusCode::BAD_REQUEST);

    let request = admission_request(fixture.manifest_revision, true);
    let (submit_status, submitted) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.app_owner_token),
        Some(&request),
    )
    .await;
    assert_eq!(submit_status, StatusCode::OK, "{submitted}");
    assert_eq!(submitted["status"], "submitted");
    assert_eq!(submitted["registration_id"], "TEST-REG-ADMISSION");
    assert_eq!(submitted["production_credential_issued"], false);
    assert_eq!(submitted["network_access_enabled"], false);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.app_owner_token),
            Some(&request),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let queue_path = "/api/admin/open-commerce/developer-app-admissions";
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
    assert_eq!(queue["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        queue["items"][0]["admission"]["registration_id"],
        "TEST-REG-ADMISSION"
    );

    let review_path = format!(
        "/api/admin/open-commerce/developer-app-admissions/{}/review",
        fixture.app_record_id
    );
    let review = json!({
        "expected_manifest_revision": fixture.manifest_revision,
        "decision": "approved",
        "risk_tier": "standard",
        "note": "admission API test"
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
    assert_eq!(reviewed["status"], "approved");
    assert_eq!(reviewed["risk_tier"], "standard");
    assert_eq!(reviewed["production_credential_issued"], false);
    assert_eq!(reviewed["network_access_enabled"], false);
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

impl AdmissionApiFixture {
    fn admission_path(&self, project_id: &str) -> String {
        format!(
            "/api/projects/{project_id}/open-commerce/developer-apps/{}/admission",
            self.app_record_id
        )
    }
}

fn admission_request(revision: i64, information_attested: bool) -> Value {
    json!({
        "expected_manifest_revision": revision,
        "organization_name": "Example Merchant Ltd",
        "jurisdiction": "Test Jurisdiction",
        "registration_id": "TEST-REG-ADMISSION",
        "information_attested": information_attested
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

fn fixture() -> AdmissionApiFixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_admission_api_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("admission API test store should open");
    let owner = user(&store, "admission-owner@example.com", None);
    let app_owner = user(&store, "admission-app-owner@example.com", None);
    let other_editor = user(&store, "admission-other-editor@example.com", None);
    let member = user(&store, "admission-member@example.com", None);
    let outsider = user(&store, "admission-outsider@example.com", None);
    let platform_admin = user(&store, "admission-admin@example.com", Some("admin"));
    let project = store
        .create_project(&owner.id, "Admission API", None, None)
        .unwrap()
        .project;
    let other_project = store
        .create_project(&owner.id, "Other Admission API", None, None)
        .unwrap()
        .project;
    for (account, role) in [
        ("admission-app-owner@example.com", "editor"),
        ("admission-other-editor@example.com", "editor"),
        ("admission-member@example.com", "member"),
    ] {
        store
            .add_project_member_by_account(&project.id, account, role)
            .unwrap();
    }
    let created = store
        .create_open_commerce_developer_app(
            &project.id,
            &app_owner.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.admission.api".to_string(),
                display_name: "Admission API Consumer".to_string(),
            },
        )
        .unwrap()
        .app;
    let actor = OpenCommerceActor {
        user_id: &app_owner.id,
        app_id: "pc-web",
        project_role: Some("editor"),
    };
    let app = open_commerce_developer_manifest_service::update_manifest(
        &store,
        &project.id,
        &created.id,
        UpdateDeveloperAppManifestRequest {
            expected_manifest_revision: created.manifest_revision,
            homepage_url: Some("https://shop.example.test/app".to_string()),
            privacy_policy_url: Some("https://shop.example.test/privacy".to_string()),
            terms_url: Some("https://shop.example.test/terms".to_string()),
            support_email: Some("support@example.test".to_string()),
            requested_scopes: vec!["menu.preview".to_string()],
        },
        &actor,
    )
    .unwrap();
    let app = store
        .issue_open_commerce_developer_app_domain_challenge(
            &project.id,
            &app.id,
            app.manifest_revision,
            "shop.example.test",
            "admission-api-local-hash",
            &(Utc::now() + Duration::hours(1)).to_rfc3339(),
        )
        .unwrap();
    let app = store
        .verify_open_commerce_developer_app_domain(&project.id, &app.id, app.manifest_revision)
        .unwrap();
    let app = open_commerce_developer_manifest_service::submit_manifest(
        &store,
        &project.id,
        &app.id,
        app.manifest_revision,
        &actor,
    )
    .unwrap();
    let app = open_commerce_developer_manifest_service::review_manifest(
        &store,
        &app.id,
        ReviewDeveloperAppManifestRequest {
            expected_manifest_revision: app.manifest_revision,
            decision: "approved".to_string(),
            note: "local admission fixture".to_string(),
        },
        &platform_admin.id,
    )
    .unwrap();

    let owner_token = session(&store, &owner.id);
    let app_owner_token = session(&store, &app_owner.id);
    let other_editor_token = session(&store, &other_editor.id);
    let member_token = session(&store, &member.id);
    let outsider_token = session(&store, &outsider.id);
    let platform_admin_token = session(&store, &platform_admin.id);
    let root = path.parent().unwrap().to_path_buf();
    let state = Arc::new(test_app_state(store, &root));
    let router = routes().with_state(state.clone());

    AdmissionApiFixture {
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
        .create_session(user_id, Some("admission-api-test"), None)
        .unwrap()
        .0
}
