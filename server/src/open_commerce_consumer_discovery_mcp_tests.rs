use anyhow::Result;
use serde_json::{json, Value};

use crate::{
    open_commerce_consumer, open_commerce_consumer_model::ConsumerDiscoveryRequest,
    open_commerce_developer_model::CreateDeveloperAppRequest,
};

use super::test_support::{fixture, AUTHORIZED_QUERY, PUBLIC_ACTION, PUBLIC_QUERY};

const DISCOVER: &str = "open_commerce_discover_for_consumer";

async fn call(
    fixture: &super::test_support::Fixture,
    project_id: &str,
    user_id: &str,
    app_id: &str,
    arguments: Value,
) -> Result<Value> {
    let response = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        project_id,
        user_id,
        "owner",
        app_id,
        json!({"name":DISCOVER,"arguments":arguments}),
    )
    .await?;
    Ok(response["structuredContent"].clone())
}

#[tokio::test]
async fn default_identity_discovers_public_and_requires_app_for_authorized_without_writes() {
    let fixture = fixture();
    let before = fixture.snapshot();
    let public = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "mcp-client",
        json!({"capability_key":PUBLIC_QUERY}),
    )
    .await
    .unwrap();
    assert_eq!(public["schema"], "open_commerce.consumer_discovery.v1");
    assert_eq!(public["requester_app_id"], "pc-web");
    assert_eq!(public["ranking_is_paid"], false);
    assert_eq!(public["matches"].as_array().unwrap().len(), 1);
    assert_eq!(
        public["matches"][0]["authorization"]["status"],
        "not_required"
    );

    let authorized = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "mcp-client",
        json!({"capability_key":AUTHORIZED_QUERY}),
    )
    .await
    .unwrap();
    assert_eq!(authorized["requester_app_id"], "pc-web");
    assert_eq!(
        authorized["matches"][0]["authorization"]["status"],
        "app_registration_required"
    );
    let serialized = authorized.to_string();
    for internal_id in [
        fixture.merchant_owner_id.as_str(),
        fixture.merchant_project_id.as_str(),
        fixture.consumer_id.as_str(),
        fixture.consumer_project_id.as_str(),
    ] {
        assert!(!serialized.contains(internal_id), "leaked {internal_id}");
    }
    assert_eq!(before, fixture.snapshot());
}

#[tokio::test]
async fn owned_app_mcp_result_matches_the_shared_domain_service_and_reflects_grant() {
    let fixture = fixture();
    fixture.grant(AUTHORIZED_QUERY, Some(5), Some(20_000), "CNY");
    let before = fixture.snapshot();
    let arguments = json!({
        "query":"执行计划咖啡店",
        "capability_key":AUTHORIZED_QUERY,
        "ranking_policy":"lowest_unit_price.v1",
        "capability_kind":"query",
        "access_level":"authorized",
        "price_currency":"CNY",
        "preferences":{
            "categories":["cafe"],
            "max_unit_price_micros":1_000
        },
        "require_category_match":true,
        "limit":5
    });
    let actual = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        &fixture.app_id,
        arguments.clone(),
    )
    .await
    .unwrap();

    let mut direct_arguments = arguments;
    direct_arguments["requester_app_id"] = json!(fixture.app_id);
    let direct_request: ConsumerDiscoveryRequest =
        serde_json::from_value(direct_arguments).unwrap();
    let expected = serde_json::to_value(
        open_commerce_consumer::discover(&fixture.store, &fixture.consumer_id, direct_request)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual["matches"][0]["authorization"]["status"], "granted");
    assert_eq!(
        actual["matches"][0]["authorization"]["grant_budget_status"],
        "available"
    );
    assert_eq!(before, fixture.snapshot());
}

#[tokio::test]
async fn explicit_identity_must_be_active_owned_and_in_the_current_project() {
    let fixture = fixture();
    let other_project = fixture
        .store
        .create_project(
            &fixture.consumer_id,
            "Discovery Other Consumer Project",
            None,
            None,
        )
        .unwrap()
        .project;
    let cross_project_app = fixture
        .store
        .create_open_commerce_developer_app(
            &other_project.id,
            &fixture.consumer_id,
            CreateDeveloperAppRequest {
                app_id: "consumer.discovery-cross-project".to_string(),
                display_name: "Cross Project Discovery".to_string(),
            },
        )
        .unwrap();
    let disabled_app = fixture
        .store
        .create_open_commerce_developer_app(
            &fixture.consumer_project_id,
            &fixture.consumer_id,
            CreateDeveloperAppRequest {
                app_id: "consumer.discovery-disabled".to_string(),
                display_name: "Disabled Discovery".to_string(),
            },
        )
        .unwrap();
    fixture
        .store
        .disable_open_commerce_developer_app(&fixture.consumer_project_id, &disabled_app.app.id)
        .unwrap();
    let before = fixture.snapshot();

    for (app_id, expected) in [
        (fixture.other_app_id.as_str(), "不能代表该开发者应用"),
        (cross_project_app.app.app_id.as_str(), "不属于当前项目"),
        (disabled_app.app.app_id.as_str(), "开发者应用已停用"),
        ("consumer.discovery-missing", "开发者应用不存在"),
    ] {
        let error = call(
            &fixture,
            &fixture.consumer_project_id,
            &fixture.consumer_id,
            app_id,
            json!({"capability_key":PUBLIC_QUERY}),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains(expected), "{app_id}: {error:#}");
    }
    for (name, arguments) in [
        (DISCOVER, json!({"capability_key":PUBLIC_QUERY})),
        (
            super::PLAN_CONSUMER_CAPABILITY,
            json!({
                "merchant_id":fixture.merchant_id,
                "capability_key":PUBLIC_QUERY,
                "input":{"sku":"coffee"}
            }),
        ),
        (
            super::REQUEST_CONSUMER_AUTHORIZATION,
            json!({
                "merchant_id":fixture.merchant_id,
                "capability_key":AUTHORIZED_QUERY,
                "purpose":"验证跨项目身份阻断",
                "confirmation_phrase":"REQUEST_AUTHORIZATION"
            }),
        ),
    ] {
        let error = super::call_if_handled(
            &fixture.store,
            &fixture.consumer_project_id,
            &fixture.consumer_id,
            &cross_project_app.app.app_id,
            false,
            name,
            arguments,
        )
        .unwrap_err();
        assert!(error.to_string().contains("不属于当前项目"));
    }
    assert_eq!(before, fixture.snapshot());
}

#[tokio::test]
async fn filters_receipt_and_entry_identity_override_are_bounded_and_read_only() {
    let fixture = fixture();
    let before = fixture.snapshot();
    let response = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        &fixture.app_id,
        json!({
            "requester_app_id":fixture.other_app_id,
            "capability_key":PUBLIC_ACTION,
            "ranking_policy":"lowest_unit_price.v1",
            "include_ranking_receipt":true,
            "capability_kind":"action",
            "access_level":"public",
            "price_currency":"cny",
            "preferences":{
                "categories":["cafe"],
                "max_unit_price_micros":1_000,
                "prefer_public":true
            },
            "require_category_match":true,
            "limit":1
        }),
    )
    .await
    .unwrap();
    assert_eq!(response["requester_app_id"], fixture.app_id);
    assert_eq!(response["ranking_policy"], "lowest_unit_price.v1");
    assert_eq!(response["ranking_is_user_selected"], true);
    assert_eq!(response["price_filter"]["currency"], "CNY");
    assert_eq!(response["capability_filter"]["kind"], "action");
    assert_eq!(response["matches"].as_array().unwrap().len(), 1);
    assert_eq!(response["ranking_receipt"]["signed_by_operator"], false);
    assert_eq!(
        response["candidate_scope"]["kind"],
        "current_operator_public_directory.v1"
    );
    assert_eq!(response["candidate_scope"]["operator_exhaustive"], false);
    assert_eq!(before, fixture.snapshot());
}

#[tokio::test]
async fn schema_and_domain_invalid_inputs_fail_closed_without_writes() {
    let fixture = fixture();
    let before = fixture.snapshot();
    for arguments in [
        json!({"capability_key":PUBLIC_QUERY,"limit":0}),
        json!({"capability_key":PUBLIC_QUERY,"limit":51}),
        json!({"capability_key":PUBLIC_QUERY,"unknown_filter":true}),
        json!({"capability_key":PUBLIC_QUERY,"price_currency":"YUAN"}),
        json!({"capability_key":PUBLIC_QUERY,"require_city_match":true}),
    ] {
        assert!(call(
            &fixture,
            &fixture.consumer_project_id,
            &fixture.consumer_id,
            &fixture.app_id,
            arguments,
        )
        .await
        .is_err());
    }
    assert_eq!(before, fixture.snapshot());
}

#[test]
fn definition_exposes_read_only_non_paid_discovery_contract() {
    let definition = super::definitions()
        .into_iter()
        .find(|definition| definition["name"] == DISCOVER)
        .unwrap();
    assert_eq!(definition["annotations"]["readOnlyHint"], true);
    assert_eq!(definition["annotations"]["destructiveHint"], false);
    assert_eq!(definition["annotations"]["idempotentHint"], true);
    assert_eq!(definition["annotations"]["openWorldHint"], true);
    assert_eq!(definition["inputSchema"]["additionalProperties"], false);
    assert!(definition["inputSchema"]["properties"]
        .get("requester_app_id")
        .is_none());
    assert!(
        crate::open_commerce_mcp_protocol::initialize_response()["instructions"]
            .as_str()
            .unwrap()
            .contains("不会自动调用或下单")
    );
}
