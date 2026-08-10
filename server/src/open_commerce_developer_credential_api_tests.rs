use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use crate::{
    open_commerce_developer_credential_api::routes,
    open_commerce_developer_credential_model::production_credentials_enabled,
    open_commerce_developer_production_test_support::{
        approved_developer_fixture, issue_local_credential, test_app_state,
    },
    store::Store,
    types::AppState,
};

struct CredentialApiFixture {
    state: Arc<AppState>,
    router: Router,
    project_id: String,
    other_project_id: String,
    app_record_id: String,
    manifest_revision: i64,
    credential_id: String,
    live_token: String,
    owner_token: String,
    other_editor_token: String,
    member_token: String,
    outsider_token: String,
    platform_admin_token: String,
}

#[tokio::test]
async fn credential_listing_and_revocation_require_app_management() {
    let fixture = fixture();
    let path = fixture.credentials_path(&fixture.project_id);

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
        assert!(!body.to_string().contains(&fixture.live_token));
    }
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &fixture.credentials_path(&fixture.other_project_id),
            Some(&fixture.owner_token),
            None,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (list_status, listed) = call(
        &fixture.router,
        Method::GET,
        &path,
        Some(&fixture.owner_token),
        None,
    )
    .await;
    assert_eq!(list_status, StatusCode::OK, "{listed}");
    assert_eq!(listed["credentials"].as_array().unwrap().len(), 1);
    assert_eq!(listed["credentials"][0]["id"], fixture.credential_id);
    assert_eq!(listed["credentials"][0]["status"], "active");
    assert_eq!(listed["issuance_enabled"], production_credentials_enabled());
    assert_eq!(listed["funds_moved"], false);
    assert!(listed["credentials"][0].get("live_token").is_none());
    assert!(!listed.to_string().contains(&fixture.live_token));

    let revoke_path = format!("{}/{}/revoke", path, fixture.credential_id);
    let short_reason = json!({"reason": "bad"});
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &revoke_path,
            Some(&fixture.owner_token),
            Some(&short_reason),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(current_status(&fixture), "active");

    for token in [&fixture.member_token, &fixture.other_editor_token] {
        assert_eq!(
            call(
                &fixture.router,
                Method::POST,
                &revoke_path,
                Some(token),
                Some(&json!({"reason": "unauthorized revocation"})),
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(current_status(&fixture), "active");

    let reason = json!({"reason": "owner requested credential rotation"});
    let (revoke_status, revoked) = call(
        &fixture.router,
        Method::POST,
        &revoke_path,
        Some(&fixture.owner_token),
        Some(&reason),
    )
    .await;
    assert_eq!(revoke_status, StatusCode::OK, "{revoked}");
    assert_eq!(revoked["status"], "revoked");
    assert_eq!(
        revoked["revocation_reason"],
        "owner requested credential rotation"
    );
    assert!(revoked.get("live_token").is_none());
    assert!(!revoked.to_string().contains(&fixture.live_token));
    let (repeat_status, repeated) = call(
        &fixture.router,
        Method::POST,
        &revoke_path,
        Some(&fixture.owner_token),
        Some(&reason),
    )
    .await;
    assert_eq!(repeat_status, StatusCode::OK, "{repeated}");
    assert_eq!(repeated["status"], "revoked");
    assert_eq!(repeated["revocation_reason"], revoked["revocation_reason"]);
    assert!(repeated.get("live_token").is_none());
}

#[tokio::test]
async fn credential_issuance_is_platform_admin_only_and_respects_feature_gate() {
    let fixture = fixture();
    let path = format!(
        "/api/admin/open-commerce/developer-apps/{}/production-credentials/issue",
        fixture.app_record_id
    );
    let request = json!({
        "expected_manifest_revision": fixture.manifest_revision,
        "scopes": ["menu.preview"],
        "expires_in_days": 30
    });

    assert_eq!(
        call(&fixture.router, Method::POST, &path, None, Some(&request))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.owner_token),
            Some(&request),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let before = credential_count(&fixture);
    let (status, body) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.platform_admin_token),
        Some(&request),
    )
    .await;
    if production_credentials_enabled() {
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["token_visible_once"], true);
        assert_eq!(body["funds_moved"], false);
        assert!(body["live_token"]
            .as_str()
            .is_some_and(|token| token.starts_with("oc_live_")));
        assert_eq!(credential_count(&fixture), before + 1);
    } else {
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(credential_count(&fixture), before);
        assert!(body.get("live_token").is_none());
    }
}

impl CredentialApiFixture {
    fn credentials_path(&self, project_id: &str) -> String {
        format!(
            "/api/projects/{project_id}/open-commerce/developer-apps/{}/production-credentials",
            self.app_record_id
        )
    }
}

fn current_status(fixture: &CredentialApiFixture) -> String {
    fixture
        .state
        .store
        .list_open_commerce_developer_production_credentials(
            &fixture.project_id,
            &fixture.app_record_id,
            20,
        )
        .unwrap()
        .into_iter()
        .find(|credential| credential.id == fixture.credential_id)
        .unwrap()
        .status
}

fn credential_count(fixture: &CredentialApiFixture) -> usize {
    fixture
        .state
        .store
        .list_open_commerce_developer_production_credentials(
            &fixture.project_id,
            &fixture.app_record_id,
            20,
        )
        .unwrap()
        .len()
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

fn fixture() -> CredentialApiFixture {
    let fixture = approved_developer_fixture();
    let secret = issue_local_credential(&fixture);
    let other_editor = user(&fixture.store, "credential-editor@example.com", None);
    let member = user(&fixture.store, "credential-member@example.com", None);
    let outsider = user(&fixture.store, "credential-outsider@example.com", None);
    let platform_admin = user(
        &fixture.store,
        "credential-platform-admin@example.com",
        Some("admin"),
    );
    for (account, role) in [
        ("credential-editor@example.com", "editor"),
        ("credential-member@example.com", "member"),
    ] {
        fixture
            .store
            .add_project_member_by_account(&fixture.project_id, account, role)
            .unwrap();
    }
    let other_project = fixture
        .store
        .create_project(&fixture.owner_user_id, "Other Credential API", None, None)
        .unwrap()
        .project;
    let owner_token = session(&fixture.store, &fixture.owner_user_id);
    let other_editor_token = session(&fixture.store, &other_editor.id);
    let member_token = session(&fixture.store, &member.id);
    let outsider_token = session(&fixture.store, &outsider.id);
    let platform_admin_token = session(&fixture.store, &platform_admin.id);
    let root = std::env::temp_dir();
    let state = Arc::new(test_app_state(fixture.store, &root));
    let router = routes().with_state(state.clone());

    CredentialApiFixture {
        state,
        router,
        project_id: fixture.project_id,
        other_project_id: other_project.id,
        app_record_id: fixture.app.id,
        manifest_revision: fixture.app.manifest_revision,
        credential_id: secret.credential.id,
        live_token: secret.live_token,
        owner_token,
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
        .create_session(user_id, Some("credential-api-test"), None)
        .unwrap()
        .0
}
