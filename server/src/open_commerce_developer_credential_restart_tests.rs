use std::{path::Path, process::Command, sync::Arc};

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
    open_commerce_developer_credential_api,
    open_commerce_developer_credential_model::{
        production_credentials_enabled, PRODUCTION_CREDENTIAL_ENV,
    },
    open_commerce_developer_event_api,
    open_commerce_developer_production_test_support::{approved_developer_app_for, test_app_state},
    store::Store,
};

const CHILD_ENV: &str = "ELON_TEST_PRODUCTION_CREDENTIAL_RESTART_CHILD";
const DB_PATH_ENV: &str = "ELON_TEST_PRODUCTION_CREDENTIAL_RESTART_DB";
const PROJECT_ID_ENV: &str = "ELON_TEST_PRODUCTION_CREDENTIAL_RESTART_PROJECT";
const APP_RECORD_ID_ENV: &str = "ELON_TEST_PRODUCTION_CREDENTIAL_RESTART_APP";
const CREDENTIAL_ID_ENV: &str = "ELON_TEST_PRODUCTION_CREDENTIAL_RESTART_CREDENTIAL";
const LIVE_TOKEN_ENV: &str = "ELON_TEST_PRODUCTION_CREDENTIAL_RESTART_TOKEN";
const OWNER_SESSION_ENV: &str = "ELON_TEST_PRODUCTION_CREDENTIAL_RESTART_SESSION";
const CHILD_TEST: &str = "open_commerce_developer_production_state_tests::credential_restart_tests::persisted_live_credential_defaults_closed_after_restart_child";

#[test]
fn persisted_live_credential_defaults_closed_after_restart() {
    let db_path = std::env::temp_dir().join(format!(
        "elon_open_commerce_credential_restart_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&db_path).unwrap();
    let owner = store
        .create_user(
            "credential-restart-owner@example.com",
            "secret1",
            Some("Credential Restart Owner"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Credential Restart", None, None)
        .unwrap()
        .project;
    let app = approved_developer_app_for(
        &store,
        &project.id,
        &owner.id,
        "consumer.production.restart",
        &["menu.preview"],
    );
    let secret = store
        .issue_open_commerce_developer_production_credential(
            &app.app,
            &app.admission_id,
            &["menu.preview".to_string()],
            "reviewer-user",
            &(Utc::now() + Duration::days(30)).to_rfc3339(),
        )
        .unwrap();
    let owner_session = store
        .create_session(&owner.id, Some("credential-restart-test"), None)
        .unwrap()
        .0;
    drop(store);

    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD_TEST, "--nocapture"])
        .env(CHILD_ENV, "1")
        .env(DB_PATH_ENV, &db_path)
        .env(PROJECT_ID_ENV, &project.id)
        .env(APP_RECORD_ID_ENV, &app.app.id)
        .env(CREDENTIAL_ID_ENV, &secret.credential.id)
        .env(LIVE_TOKEN_ENV, &secret.live_token)
        .env(OWNER_SESSION_ENV, owner_session)
        .env_remove(PRODUCTION_CREDENTIAL_ENV)
        .output()
        .expect("launch production credential restart test");
    assert!(
        output.status.success(),
        "restart child failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn persisted_live_credential_defaults_closed_after_restart_child() {
    if std::env::var(CHILD_ENV).as_deref() != Ok("1") {
        return;
    }
    assert!(!production_credentials_enabled());
    let db_path = required_env(DB_PATH_ENV);
    let project_id = required_env(PROJECT_ID_ENV);
    let app_record_id = required_env(APP_RECORD_ID_ENV);
    let credential_id = required_env(CREDENTIAL_ID_ENV);
    let live_token = required_env(LIVE_TOKEN_ENV);
    let owner_session = required_env(OWNER_SESSION_ENV);
    let store = Store::open(Path::new(&db_path)).unwrap();
    let root = std::env::temp_dir();
    let state = Arc::new(test_app_state(store, &root));
    let router = open_commerce_developer_event_api::routes()
        .merge(open_commerce_developer_credential_api::routes())
        .with_state(state);

    let (events_status, events) = call(
        &router,
        Method::GET,
        "/api/open-commerce/developer/events",
        &live_token,
        None,
    )
    .await;
    assert_eq!(events_status, StatusCode::UNAUTHORIZED);
    assert!(events.to_string().contains("当前未启用"));
    assert!(!events.to_string().contains(&live_token));

    let credentials_path = format!(
        "/api/projects/{project_id}/open-commerce/developer-apps/{app_record_id}/production-credentials"
    );
    let (list_status, listed) = call(
        &router,
        Method::GET,
        &credentials_path,
        &owner_session,
        None,
    )
    .await;
    assert_eq!(list_status, StatusCode::OK, "{listed}");
    assert_eq!(listed["issuance_enabled"], false);
    assert_eq!(listed["credentials"][0]["status"], "active");
    assert!(!listed.to_string().contains(&live_token));

    let (revoke_status, revoked) = call(
        &router,
        Method::POST,
        &format!("{credentials_path}/{credential_id}/revoke"),
        &owner_session,
        Some(&json!({"reason":"进程重启默认关闭后的应急撤销"})),
    )
    .await;
    assert_eq!(revoke_status, StatusCode::OK, "{revoked}");
    assert_eq!(revoked["status"], "revoked");
    assert!(!revoked.to_string().contains(&live_token));
}

fn required_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("missing {name}"))
}

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    bearer: &str,
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"));
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
