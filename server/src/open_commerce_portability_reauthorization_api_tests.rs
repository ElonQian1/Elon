use axum::http::{Method, StatusCode};
use serde_json::{json, Value};

#[path = "open_commerce_portability_reauthorization_test_support.rs"]
mod support;
use support::*;

#[tokio::test]
async fn routes_enforce_login_project_membership_and_per_user_mapping_ownership() {
    let fixture = fixture();
    let collection = mappings_path(&fixture.destination_project_id);
    assert_eq!(
        send_json(&fixture.router, Method::GET, &collection, None, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send_json(
            &fixture.router,
            Method::GET,
            &collection,
            Some(&fixture.outsider_token),
            Value::Null,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (status, member_list) = send_json(
        &fixture.router,
        Method::GET,
        &collection,
        Some(&fixture.member_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(member_list["mappings"].as_array().unwrap().is_empty());

    let (status, unconfirmed) = send_json(
        &fixture.router,
        Method::POST,
        &collection,
        Some(&fixture.owner_token),
        json!({
            "import_id": fixture.import_id,
            "source_relationship_id": fixture.source_relationship_id,
            "target_merchant_id": fixture.target_merchant_id,
            "confirmed_by_user": false,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(unconfirmed["error"].as_str().unwrap().contains("明确确认"));

    let (status, mapping) = send_json(
        &fixture.router,
        Method::POST,
        &collection,
        Some(&fixture.owner_token),
        mapping_json(&fixture),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{mapping}");
    assert_eq!(mapping["status"], "active");
    let mapping_id = mapping["id"].as_str().unwrap();

    let (status, owner_list) = send_json(
        &fixture.router,
        Method::GET,
        &collection,
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(owner_list["mappings"].as_array().unwrap().len(), 1);

    let (status, second_project_list) = send_json(
        &fixture.router,
        Method::GET,
        &mappings_path(&fixture.second_project_id),
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(second_project_list["mappings"]
        .as_array()
        .unwrap()
        .is_empty());

    let (status, cross_project) = send_json(
        &fixture.router,
        Method::POST,
        &mappings_path(&fixture.second_project_id),
        Some(&fixture.owner_token),
        mapping_json(&fixture),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(cross_project["error"].as_str().unwrap().contains("不存在"));

    let (status, member_revoke) = send_json(
        &fixture.router,
        Method::POST,
        &revoke_path(&fixture.destination_project_id, mapping_id),
        Some(&fixture.member_token),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(member_revoke["error"].as_str().unwrap().contains("不存在"));
}

#[tokio::test]
async fn routes_create_a_new_request_without_restoring_grants_and_allow_owner_revoke() {
    let fixture = fixture();
    let (status, mapping) = send_json(
        &fixture.router,
        Method::POST,
        &mappings_path(&fixture.destination_project_id),
        Some(&fixture.owner_token),
        mapping_json(&fixture),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{mapping}");
    let mapping_id = mapping["id"].as_str().unwrap();

    let (status, wrong_app) = send_json(
        &fixture.router,
        Method::POST,
        &reauthorize_path(&fixture.destination_project_id, mapping_id),
        Some(&fixture.owner_token),
        json!({
            "requester_app_id": fixture.outsider_app_id,
            "scopes": ["preference.remember"],
            "purpose": "迁移后重新授权",
            "confirmed_by_user": true,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(wrong_app["error"].as_str().unwrap().contains("不能代表"));

    let (status, result) = send_json(
        &fixture.router,
        Method::POST,
        &reauthorize_path(&fixture.destination_project_id, mapping_id),
        Some(&fixture.owner_token),
        json!({
            "requester_app_id": fixture.app_id,
            "scopes": ["preference.remember"],
            "purpose": "迁移后重新授权",
            "confirmed_by_user": true,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["old_grant_restored"], false);
    assert_eq!(result["authorization_request"]["status"], "pending");
    assert_eq!(fixture.count("open_commerce_grants"), 0);

    let (status, revoked) = send_json(
        &fixture.router,
        Method::POST,
        &revoke_path(&fixture.destination_project_id, mapping_id),
        Some(&fixture.owner_token),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{revoked}");
    assert_eq!(revoked["status"], "revoked");
}

fn mapping_json(fixture: &Fixture) -> Value {
    json!({
        "import_id": fixture.import_id,
        "source_relationship_id": fixture.source_relationship_id,
        "target_merchant_id": fixture.target_merchant_id,
        "confirmed_by_user": true,
    })
}

fn mappings_path(project_id: &str) -> String {
    format!("/api/projects/{project_id}/open-commerce/portability-relationship-mappings")
}

fn revoke_path(project_id: &str, mapping_id: &str) -> String {
    format!("{}/{mapping_id}/revoke", mappings_path(project_id))
}

fn reauthorize_path(project_id: &str, mapping_id: &str) -> String {
    format!("{}/{mapping_id}/reauthorize", mappings_path(project_id))
}
