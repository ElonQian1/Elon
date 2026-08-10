use axum::http::{Method, StatusCode};
use serde_json::{json, Value};

#[path = "open_commerce_portability_adoption_test_support.rs"]
mod support;
use support::*;

#[tokio::test]
async fn routes_enforce_login_project_membership_and_per_user_import_ownership() {
    let fixture = fixture(true);
    let plan_url = plan_path(&fixture.target_project_id, &fixture.import_id);

    assert_eq!(
        send_json(&fixture.router, Method::GET, &plan_url, None, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send_json(
            &fixture.router,
            Method::GET,
            &plan_url,
            Some(&fixture.outsider_token),
            Value::Null,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, body) = send_json(
        &fixture.router,
        Method::GET,
        &plan_url,
        Some(&fixture.member_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("不存在"));

    let (status, plan) = send_json(
        &fixture.router,
        Method::GET,
        &plan_url,
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(plan["current_profile_revision"], 1);
    assert_eq!(plan["preference_changes"].as_array().unwrap().len(), 5);
    assert!(plan["preference_changes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|change| change["changed"] == true));

    let (status, body) = send_json(
        &fixture.router,
        Method::GET,
        &plan_path(&fixture.second_project_id, &fixture.import_id),
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("不存在"));
}

#[tokio::test]
async fn routes_apply_list_and_rollback_only_the_callers_snapshot() {
    let fixture = fixture(true);
    let apply_path = apply_path(&fixture.target_project_id, &fixture.import_id);
    let (status, invalid) = send_json(
        &fixture.router,
        Method::POST,
        &apply_path,
        Some(&fixture.owner_token),
        json!({
            "expected_current_revision": 1,
            "selected_fields": ["private_notes"],
            "confirmed_by_user": true,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(invalid["error"].as_str().unwrap().contains("不支持"));

    let (status, adoption) = send_json(
        &fixture.router,
        Method::POST,
        &apply_path,
        Some(&fixture.owner_token),
        json!({
            "expected_current_revision": 1,
            "selected_fields": ["tags", "city"],
            "confirmed_by_user": true,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{adoption}");
    assert_eq!(adoption["selected_fields"], json!(["tags", "city"]));
    assert_eq!(adoption["applied_preferences"]["city"], "Shanghai");
    assert_eq!(
        adoption["applied_preferences"]["categories"],
        json!(["tea"])
    );
    let adoption_id = adoption["id"].as_str().unwrap();

    let list_path = list_path(&fixture.target_project_id);
    let (status, owner_list) = send_json(
        &fixture.router,
        Method::GET,
        &list_path,
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(owner_list["adoptions"].as_array().unwrap().len(), 1);
    let (status, member_list) = send_json(
        &fixture.router,
        Method::GET,
        &list_path,
        Some(&fixture.member_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(member_list["adoptions"].as_array().unwrap().is_empty());

    let rollback_path = rollback_path(&fixture.target_project_id, adoption_id);
    assert_eq!(
        send_json(
            &fixture.router,
            Method::POST,
            &rollback_path,
            Some(&fixture.member_token),
            json!({"expected_current_revision": 2, "confirmed_by_user": true}),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send_json(
            &fixture.router,
            Method::POST,
            &rollback_path,
            Some(&fixture.owner_token),
            json!({"expected_current_revision": 2, "confirmed_by_user": false}),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (status, rolled_back) = send_json(
        &fixture.router,
        Method::POST,
        &rollback_path,
        Some(&fixture.owner_token),
        json!({"expected_current_revision": 2, "confirmed_by_user": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{rolled_back}");
    assert_eq!(rolled_back["status"], "rolled_back");
    assert_eq!(rolled_back["rollback_revision"], 3);
    assert_eq!(
        fixture.owner_profile().unwrap().preferences.city.unwrap(),
        "Beijing"
    );
}

fn plan_path(project_id: &str, import_id: &str) -> String {
    format!(
        "/api/projects/{project_id}/open-commerce/consumer-portability-imports/{import_id}/adoption-plan"
    )
}

fn apply_path(project_id: &str, import_id: &str) -> String {
    format!(
        "/api/projects/{project_id}/open-commerce/consumer-portability-imports/{import_id}/adopt-preferences"
    )
}

fn list_path(project_id: &str) -> String {
    format!("/api/projects/{project_id}/open-commerce/consumer-portability-adoptions")
}

fn rollback_path(project_id: &str, adoption_id: &str) -> String {
    format!(
        "/api/projects/{project_id}/open-commerce/consumer-portability-adoptions/{adoption_id}/rollback"
    )
}
