use std::process::Command;

use crate::{
    open_commerce_developer_credential_model::{
        production_credentials_enabled, IssueDeveloperProductionCredentialRequest,
        PRODUCTION_CREDENTIAL_ENV,
    },
    open_commerce_developer_credential_service,
    open_commerce_developer_production_test_support::{
        approved_developer_fixture_for, test_app_state,
    },
};

use super::*;

const CHILD_ENV: &str = "ELON_TEST_OPEN_COMMERCE_PRODUCTION_RUNTIME_CHILD";
const PRODUCTION_APP_ID: &str = "consumer.production.runtime.ai";
const CHILD_TEST: &str = "open_commerce_runtime_service_tests::adapter_tcp_tests::production_credential_tests::production_live_credential_reaches_runtime_and_adapter_claim_child";

#[test]
fn production_live_credential_reaches_runtime_and_adapter_claim_in_isolated_process() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD_TEST, "--nocapture"])
        .env(CHILD_ENV, "1")
        .env(PRODUCTION_CREDENTIAL_ENV, "1")
        .output()
        .expect("launch isolated production credential test");
    assert!(
        output.status.success(),
        "isolated production credential test failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn production_live_credential_reaches_runtime_and_adapter_claim_child() {
    if std::env::var(CHILD_ENV).as_deref() != Ok("1") {
        return;
    }
    assert!(production_credentials_enabled());
    std::env::set_var(RUNTIME_SECRET_REF, RUNTIME_SECRET);
    let runtime_state = RuntimeState {
        invocation_count: Arc::new(AtomicUsize::new(0)),
        envelopes: Arc::new(Mutex::new(Vec::new())),
    };
    let runtime_server = TcpServer::start(
        Router::new()
            .route("/commerce/v1/invoke", post(runtime_handler))
            .with_state(runtime_state.clone()),
    )
    .await;

    let developer =
        approved_developer_fixture_for(PRODUCTION_APP_ID, &["menu.preview", "order.commit"]);
    let live = open_commerce_developer_credential_service::issue_credential(
        &developer.store,
        &developer.app.id,
        IssueDeveloperProductionCredentialRequest {
            expected_manifest_revision: developer.app.manifest_revision,
            scopes: vec!["menu.preview".to_string(), "order.commit".to_string()],
            expires_in_days: 30,
        },
        "reviewer-user",
    )
    .unwrap();
    assert!(live.live_token.starts_with("oc_live_"));
    assert_eq!(live.credential.environment, "production");
    assert_eq!(live.credential.scopes, vec!["menu.preview", "order.commit"]);

    let merchant_owner = developer
        .store
        .create_user(
            "production-runtime-merchant@example.com",
            "secret1",
            Some("Production Runtime Merchant"),
            None,
        )
        .unwrap();
    let merchant_project = developer
        .store
        .create_project(
            &merchant_owner.id,
            "Production Runtime Merchant",
            None,
            None,
        )
        .unwrap()
        .project;
    let merchant_actor = owner_actor(&merchant_owner.id);
    let merchant = open_commerce_service::create_merchant(
        &developer.store,
        &merchant_project.id,
        &merchant_actor,
        CreateMerchantRequest {
            display_name: "生产凭据咖啡店".to_string(),
            slug: Some("production-runtime-cafe".to_string()),
            description: String::new(),
            node_mode: "self_hosted".to_string(),
            public_profile: json!({"category":"coffee"}),
        },
    )
    .unwrap();
    open_commerce_runtime_service::upsert_binding(
        &developer.store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        UpsertRuntimeBindingRequest {
            endpoint_base_url: format!("http://{}", runtime_server.address),
            credential_ref: RUNTIME_SECRET_REF.to_string(),
            manifest_sha256: Some(RUNTIME_MANIFEST_SHA256.to_string()),
            timeout_ms: 2_000,
        },
    )
    .unwrap();
    open_commerce_runtime_service::verify_binding(
        &developer.store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
    )
    .await
    .unwrap();
    open_commerce_service::publish_capability(
        &developer.store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        CreateCapabilityRequest {
            capability_key: "order.commit".to_string(),
            display_name: "提交订单".to_string(),
            description: String::new(),
            kind: "action".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({
                "type":"object",
                "required":["quote_id"],
                "properties":{"quote_id":{"type":"string"}},
                "additionalProperties":false
            }),
            output_schema: json!({"type":"object"}),
            handler_type: HANDLER_MERCHANT_RUNTIME.to_string(),
            handler_config: None,
            unit_price_micros: 2_000,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
    )
    .unwrap();
    open_commerce_service::publish_capability(
        &developer.store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        CreateCapabilityRequest {
            capability_key: "menu.preview".to_string(),
            display_name: "预览菜单".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({
                "type":"object",
                "properties":{"category":{"type":"string"}},
                "additionalProperties":false
            }),
            output_schema: json!({
                "type":"object",
                "required":["items"],
                "properties":{"items":{"type":"array"}},
                "additionalProperties":false
            }),
            handler_type: HANDLER_MERCHANT_RUNTIME.to_string(),
            handler_config: None,
            unit_price_micros: 500,
            currency: "CNY".to_string(),
            freshness_seconds: 60,
        },
    )
    .unwrap();
    open_commerce_directory_service::set_publication(
        &developer.store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        true,
    )
    .unwrap();
    let integration = open_commerce_service::create_integration(
        &developer.store,
        &merchant_project.id,
        &merchant_actor,
        CreateIntegrationRequest {
            merchant_id: merchant.id.clone(),
            integration_key: "merchant.erp.production.runtime".to_string(),
            provider_key: "merchant_erp".to_string(),
            display_name: "生产凭据 ERP 适配器".to_string(),
            connection_mode: "local_adapter".to_string(),
            scopes: vec!["orders.write".to_string()],
            data_domains: vec!["orders".to_string()],
        },
    )
    .unwrap();
    let adapter_token = open_commerce_adapter_service::rotate_credential(
        &developer.store,
        &merchant_project.id,
        &integration.id,
        90,
        true,
        &merchant_actor,
    )
    .unwrap()
    .adapter_token;

    let root = std::env::temp_dir().join(format!(
        "elon-open-commerce-production-runtime-tcp-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let state = Arc::new(test_app_state(developer.store, &root));
    let platform_server = TcpServer::start(router::build_app(Arc::clone(&state))).await;
    let base_url = format!("http://{}", platform_server.address);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let action = json!({
        "merchant_id":merchant.id,
        "capability_key":"order.commit",
        "idempotency_key":"production-runtime-adapter-order-1",
        "input":{"quote_id":"quote-production-runtime-1"}
    });

    let prepared = developer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/action-confirmations"),
        &live.live_token,
        &action,
    )
    .await;
    let confirmation_id = prepared["id"].as_str().unwrap();
    let confirmed = developer_post(
        &client,
        &format!(
            "{base_url}/api/open-commerce/developer/action-confirmations/{confirmation_id}/confirm"
        ),
        &live.live_token,
        &json!({"confirmation_phrase":ACTION_CONFIRMATION_PHRASE}),
    )
    .await;
    assert_eq!(confirmed["status"], "confirmed");

    let mut invocation = action;
    invocation["action_confirmation_id"] = json!(confirmation_id);
    let committed = developer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &invocation,
    )
    .await;
    let invocation_id = committed["invocation_id"].as_str().unwrap();
    assert_eq!(committed["result"]["order_id"], "order-consumer-runtime-1");
    assert_eq!(committed["settlement_receipt"]["funds_moved"], false);

    let replayed = developer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &invocation,
    )
    .await;
    assert_eq!(replayed["replayed"], true);
    assert_eq!(replayed["invocation_id"], invocation_id);
    assert_eq!(replayed["result"], committed["result"]);
    assert_eq!(runtime_state.invocation_count.load(Ordering::SeqCst), 2);

    let query = json!({
        "merchant_id":merchant.id,
        "capability_key":"menu.preview",
        "idempotency_key":"production-runtime-menu-1",
        "input":{"category":"coffee"}
    });
    let queried = developer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &query,
    )
    .await;
    let query_invocation_id = queried["invocation_id"].as_str().unwrap();
    assert_eq!(queried["replayed"], false);
    assert_eq!(queried["result"]["items"][0], "拿铁");
    assert_eq!(queried["settlement_receipt"]["funds_moved"], false);

    let replayed_query = developer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &query,
    )
    .await;
    assert_eq!(replayed_query["replayed"], true);
    assert_eq!(replayed_query["invocation_id"], query_invocation_id);
    assert_eq!(replayed_query["result"], queried["result"]);
    assert_eq!(runtime_state.invocation_count.load(Ordering::SeqCst), 3);

    let events = developer_get(
        &client,
        &format!("{base_url}/api/open-commerce/developer/events?limit=10"),
        &live.live_token,
    )
    .await;
    assert_eq!(events["app_id"], PRODUCTION_APP_ID);
    assert_eq!(events["credential_environment"], "production");
    let production_events = events["events"].as_array().unwrap();
    assert_eq!(production_events.len(), 2);
    for expected_invocation_id in [invocation_id, query_invocation_id] {
        let event = production_events
            .iter()
            .find(|event| event["invocation_id"] == expected_invocation_id)
            .unwrap();
        assert_eq!(event["credential_id"], live.credential.id);
        assert_eq!(event["result_available"], true);
        assert_eq!(event["funds_moved"], false);
    }
    assert_developer_event_redacted(&events, &live.live_token);
    let production_cursor = events["next_cursor"].as_str().unwrap();

    let sandbox_events = developer_get(
        &client,
        &format!("{base_url}/api/open-commerce/developer/events"),
        &developer.test_token,
    )
    .await;
    assert_eq!(sandbox_events["app_id"], PRODUCTION_APP_ID);
    assert_eq!(sandbox_events["credential_environment"], "sandbox");
    assert!(sandbox_events["events"].as_array().unwrap().is_empty());
    assert_developer_event_redacted(&sandbox_events, &developer.test_token);

    let (cross_environment_status, cross_environment_body) = developer_get_response(
        &client,
        &format!("{base_url}/api/open-commerce/developer/events?cursor={production_cursor}"),
        &developer.test_token,
    )
    .await;
    assert_eq!(cross_environment_status, StatusCode::BAD_REQUEST);
    assert_developer_event_redacted(&cross_environment_body, &developer.test_token);

    let event_detail = developer_get(
        &client,
        &format!("{base_url}/api/open-commerce/developer/events/{invocation_id}"),
        &live.live_token,
    )
    .await;
    assert_eq!(event_detail["event"]["invocation_id"], invocation_id);
    assert_eq!(
        event_detail["event"]["credential_environment"],
        "production"
    );
    assert_eq!(event_detail["result"], committed["result"]);
    assert_developer_event_redacted(&event_detail, &live.live_token);

    let (sandbox_detail_status, sandbox_detail) = developer_get_response(
        &client,
        &format!("{base_url}/api/open-commerce/developer/events/{invocation_id}"),
        &developer.test_token,
    )
    .await;
    assert_eq!(sandbox_detail_status, StatusCode::NOT_FOUND);
    assert_developer_event_redacted(&sandbox_detail, &developer.test_token);

    let claims_url = format!("{base_url}/api/open-commerce/adapter/business-handoff-claims");
    let claimed = adapter_post(
        &client,
        &claims_url,
        &adapter_token,
        &json!({"lease_seconds":300}),
    )
    .await;
    assert_eq!(claimed["claimed"], true);
    assert_eq!(claimed["issue"]["claim"]["invocation_id"], invocation_id);
    let claim_id = claimed["issue"]["claim"]["id"].as_str().unwrap();
    let lease_token = claimed["issue"]["lease_token"].as_str().unwrap();
    let completed = adapter_post(
        &client,
        &format!("{claims_url}/{claim_id}/complete"),
        &adapter_token,
        &json!({
            "lease_token":lease_token,
            "receipt_key":"production-runtime-adapter-applied-1",
            "status":"applied",
            "target_domain":"erp",
            "target_reference":"erp-order-production-runtime-1",
            "completed_at":Utc::now().to_rfc3339()
        }),
    )
    .await;
    assert_eq!(completed["invocation_id"], invocation_id);
    assert_eq!(completed["status"], "applied");
    assert_eq!(completed["funds_moved"], false);

    assert_eq!(runtime_state.invocation_count.load(Ordering::SeqCst), 3);
    let envelopes = runtime_state.envelopes.lock().unwrap();
    let order = envelopes
        .iter()
        .find(|value| value["capability_key"] == "order.commit")
        .unwrap();
    assert_eq!(order["requester_user_id"], developer.owner_user_id);
    assert_eq!(order["requester_app_id"], PRODUCTION_APP_ID);
    assert_eq!(order["credential_environment"], "production");
    assert_eq!(order["credential_id"], live.credential.id);
    assert_eq!(order["action_confirmation_id"], confirmation_id);
    let query_envelope = envelopes
        .iter()
        .find(|value| value["capability_key"] == "menu.preview")
        .unwrap();
    assert_eq!(query_envelope["requester_user_id"], developer.owner_user_id);
    assert_eq!(query_envelope["requester_app_id"], PRODUCTION_APP_ID);
    assert_eq!(query_envelope["credential_environment"], "production");
    assert_eq!(query_envelope["credential_id"], live.credential.id);
    assert!(query_envelope["action_confirmation_id"].is_null());
    drop(envelopes);

    platform_server.stop().await;
    runtime_server.stop().await;
    std::env::remove_var(RUNTIME_SECRET_REF);
}

async fn developer_post(client: &reqwest::Client, url: &str, token: &str, body: &Value) -> Value {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .unwrap();
    response_json(response).await
}

async fn developer_get(client: &reqwest::Client, url: &str, token: &str) -> Value {
    let (status, body) = developer_get_response(client, url, token).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn developer_get_response(
    client: &reqwest::Client,
    url: &str,
    token: &str,
) -> (StatusCode, Value) {
    let response = client.get(url).bearer_auth(token).send().await.unwrap();
    let status = response.status();
    let body = response.json().await.unwrap();
    (status, body)
}

fn assert_developer_event_redacted(value: &Value, live_token: &str) {
    let serialized = value.to_string();
    for field in [
        "request_hash",
        "request_shape",
        "grant_id",
        "requester_user_id",
        "project_id",
        "live_token",
        "adapter_token",
        "lease_token",
    ] {
        assert!(!serialized.contains(field), "leaked field: {field}");
    }
    assert!(!serialized.contains(live_token), "leaked live credential");
}
