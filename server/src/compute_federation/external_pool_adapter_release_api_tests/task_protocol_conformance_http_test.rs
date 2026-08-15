use super::*;

const RELEASE_ID: &str = "missing-v272-release";

fn currentness_path() -> String {
    format!(
        "/api/admin/compute/external-pool-adapter-registry-releases/{RELEASE_ID}/task-protocol-conformance-runs/currentness"
    )
}

fn create_path() -> String {
    format!(
        "/api/admin/compute/external-pool-adapter-registry-releases/{RELEASE_ID}/task-protocol-conformance-runs"
    )
}

#[tokio::test]
async fn task_protocol_conformance_http_enforces_auth_and_shape_before_unavailable_runtime() {
    let fixture = fixture();
    let currentness_path = currentness_path();
    let create_path = create_path();

    let (status, _) = call(
        &fixture.router,
        Method::GET,
        &currentness_path,
        None,
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = call(
        &fixture.router,
        Method::POST,
        &create_path,
        Some(&fixture.member_token),
        &json!({"unexpected": true}),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = call(
        &fixture.router,
        Method::GET,
        &currentness_path,
        Some(lifecycle_support::LOCAL_OWNER_TOKEN),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = call(
        &fixture.router,
        Method::POST,
        &create_path,
        Some(&fixture.submitter_token),
        &json!({"unexpected": true}),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, body) = call(
        &fixture.router,
        Method::GET,
        &currentness_path,
        Some(&fixture.submitter_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(error(&body).contains("unavailable"), "{body}");

    fixture.cleanup();
}
