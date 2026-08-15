use std::sync::Arc;

use axum::http::{Method, StatusCode};
use serde_json::{json, Value};

use crate::open_commerce_consumer_model::ConsumerPreferences;

#[path = "open_commerce_portability_adoption_test_support.rs"]
mod support;
use support::*;

#[tokio::test]
async fn routes_enforce_login_membership_project_and_consumer_ownership() {
    let fixture = fixture(true);
    let alternate_id = add_import(&fixture, Some(alternate_preferences()), "api-alternate");
    let router = super::routes().with_state(Arc::clone(&fixture.state));
    let path = plan_path(&fixture.target_project_id);
    let body = json!({"import_ids":[fixture.import_id, alternate_id]});

    assert_eq!(
        send_json(&router, Method::POST, &path, None, body.clone())
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send_json(
            &router,
            Method::POST,
            &path,
            Some(&fixture.outsider_token),
            body.clone(),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, member_error) = send_json(
        &router,
        Method::POST,
        &path,
        Some(&fixture.member_token),
        body.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(member_error["error"], "消费者外部数据包导入记录不存在");

    let (status, cross_project_error) = send_json(
        &router,
        Method::POST,
        &plan_path(&fixture.second_project_id),
        Some(&fixture.owner_token),
        body.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        cross_project_error["error"],
        "消费者外部数据包导入记录不存在"
    );

    let (status, plan) = send_json(
        &router,
        Method::POST,
        &path,
        Some(&fixture.owner_token),
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(
        plan["schema"],
        "open_commerce.consumer_portability_merge_plan.v1"
    );
    assert_eq!(plan["sources"].as_array().unwrap().len(), 2);
    assert_eq!(plan["fields"].as_array().unwrap().len(), 5);
    assert_eq!(plan["automatic_conflict_resolution"], false);
    assert_eq!(plan["automatic_relationship_restore"], false);
    assert_eq!(plan["automatic_business_write"], false);
}

#[tokio::test]
async fn routes_apply_list_and_rollback_only_the_callers_merge() {
    let fixture = fixture(true);
    let alternate_id = add_import(&fixture, Some(alternate_preferences()), "api-alternate");
    let router = super::routes().with_state(Arc::clone(&fixture.state));
    let apply_url = adoptions_path(&fixture.target_project_id);
    let import_ids = vec![fixture.import_id.clone(), alternate_id.clone()];

    let (status, rejected) = send_json(
        &router,
        Method::POST,
        &apply_url,
        Some(&fixture.owner_token),
        json!({
            "import_ids": import_ids,
            "expected_current_revision": 1,
            "selections": [{"field":"city","import_id":alternate_id}],
            "confirmed_by_user": false,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(rejected["error"].as_str().unwrap().contains("明确确认"));

    let (status, adoption) = send_json(
        &router,
        Method::POST,
        &apply_url,
        Some(&fixture.owner_token),
        json!({
            "import_ids": [fixture.import_id, alternate_id],
            "expected_current_revision": 1,
            "selections": [
                {"field":"categories","import_id":fixture.import_id},
                {"field":"city","import_id":alternate_id}
            ],
            "confirmed_by_user": true,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{adoption}");
    assert_eq!(adoption["resulting_revision"], 2);
    assert_eq!(adoption["applied_preferences"]["city"], "Guangzhou");
    assert_eq!(adoption["field_sources"].as_array().unwrap().len(), 2);
    let adoption_id = adoption["id"].as_str().unwrap();

    let (status, owner_list) = send_json(
        &router,
        Method::GET,
        &apply_url,
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(owner_list["adoptions"].as_array().unwrap().len(), 1);
    let (status, member_list) = send_json(
        &router,
        Method::GET,
        &apply_url,
        Some(&fixture.member_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(member_list["adoptions"].as_array().unwrap().is_empty());

    let rollback_url = rollback_path(&fixture.target_project_id, adoption_id);
    let (status, wrong_consumer) = send_json(
        &router,
        Method::POST,
        &rollback_url,
        Some(&fixture.member_token),
        json!({"expected_current_revision":2,"confirmed_by_user":true}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(wrong_consumer["error"], "消费者多来源偏好合并记录不存在");

    let (status, rolled_back) = send_json(
        &router,
        Method::POST,
        &rollback_url,
        Some(&fixture.owner_token),
        json!({"expected_current_revision":2,"confirmed_by_user":true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{rolled_back}");
    assert_eq!(rolled_back["status"], "rolled_back");
    assert_eq!(rolled_back["rollback_revision"], 3);
    assert_eq!(
        fixture.owner_profile().unwrap().preferences.city.as_deref(),
        Some("Beijing")
    );
}

fn plan_path(project_id: &str) -> String {
    format!("/api/projects/{project_id}/open-commerce/consumer-portability-merge-plan")
}

fn adoptions_path(project_id: &str) -> String {
    format!("/api/projects/{project_id}/open-commerce/consumer-portability-merge-adoptions")
}

fn rollback_path(project_id: &str, adoption_id: &str) -> String {
    format!(
        "/api/projects/{project_id}/open-commerce/consumer-portability-merge-adoptions/{adoption_id}/rollback"
    )
}

fn alternate_preferences() -> ConsumerPreferences {
    ConsumerPreferences {
        categories: vec!["bakery".to_string()],
        tags: vec!["quiet".to_string(), "wifi".to_string()],
        city: Some("Guangzhou".to_string()),
        max_unit_price_micros: Some(50_000_000),
        prefer_public: true,
    }
}
