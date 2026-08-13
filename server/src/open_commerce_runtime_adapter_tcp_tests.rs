use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use axum::{
    body::Bytes, extract::State, http::HeaderMap, http::StatusCode, routing::post, Json, Router,
};
use chrono::Utc;
use serde_json::{json, Value};
use tokio::{sync::oneshot, task::JoinHandle};
use uuid::Uuid;

use crate::{
    open_commerce_action_confirmation_model::ACTION_CONFIRMATION_PHRASE,
    open_commerce_adapter_service,
    open_commerce_developer_model::{
        CreateDeveloperAppRequest, OpenCommerceDeveloperAppCredential,
    },
    open_commerce_developer_production_test_support::test_app_state,
    open_commerce_directory_service,
    open_commerce_integration_model::CreateIntegrationRequest,
    open_commerce_merchant_evidence_model::BUSINESS_RECEIPT_SCHEMA,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_MERCHANT_RUNTIME,
    },
    open_commerce_runtime_model::UpsertRuntimeBindingRequest,
    open_commerce_runtime_service,
    open_commerce_service::{self, OpenCommerceActor},
    router,
    store::Store,
};

const RUNTIME_SECRET_REF: &str = "OPEN_COMMERCE_RUNTIME_SECRET_CONSUMER_ADAPTER_TCP";
const RUNTIME_SECRET: &str = "consumer-adapter-runtime-secret-at-least-32-bytes";
const RUNTIME_MANIFEST_SHA256: &str =
    "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const CONSUMER_APP_ID: &str = "consumer.runtime.adapter.ai";

#[derive(Clone)]
struct RuntimeState {
    invocation_count: Arc<AtomicUsize>,
    envelopes: Arc<Mutex<Vec<Value>>>,
}

struct TcpServer {
    address: SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl TcpServer {
    async fn start(app: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });
        Self {
            address,
            shutdown,
            task,
        }
    }

    async fn stop(self) {
        let _ = self.shutdown.send(());
        tokio::time::timeout(Duration::from_secs(5), self.task)
            .await
            .expect("TCP test server should stop")
            .expect("TCP test server task should join");
    }
}

#[tokio::test]
async fn consumer_app_order_is_claimed_by_erp_adapter_over_real_tcp() {
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

    let store = temp_store();
    let merchant_owner = store
        .create_user(
            "runtime-adapter-merchant@example.com",
            "secret1",
            Some("Runtime Adapter Merchant"),
            None,
        )
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Runtime Adapter Merchant", None, None)
        .unwrap()
        .project;
    let merchant_actor = owner_actor(&merchant_owner.id);
    let merchant = open_commerce_service::create_merchant(
        &store,
        &merchant_project.id,
        &merchant_actor,
        CreateMerchantRequest {
            display_name: "消费者直连咖啡店".to_string(),
            slug: Some("consumer-runtime-adapter-cafe".to_string()),
            description: String::new(),
            node_mode: "self_hosted".to_string(),
            public_profile: json!({"category":"coffee"}),
        },
    )
    .unwrap();
    open_commerce_runtime_service::upsert_binding(
        &store,
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
        &store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
    )
    .await
    .unwrap();
    open_commerce_service::publish_capability(
        &store,
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
    open_commerce_directory_service::set_publication(
        &store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        true,
    )
    .unwrap();
    let integration = open_commerce_service::create_integration(
        &store,
        &merchant_project.id,
        &merchant_actor,
        CreateIntegrationRequest {
            merchant_id: merchant.id.clone(),
            integration_key: "merchant.erp.consumer.runtime".to_string(),
            provider_key: "merchant_erp".to_string(),
            display_name: "消费者订单 ERP 适配器".to_string(),
            connection_mode: "local_adapter".to_string(),
            scopes: vec!["orders.write".to_string()],
            data_domains: vec!["orders".to_string()],
        },
    )
    .unwrap();
    let adapter_token = open_commerce_adapter_service::rotate_credential(
        &store,
        &merchant_project.id,
        &integration.id,
        90,
        true,
        &merchant_actor,
    )
    .unwrap()
    .adapter_token;

    let consumer = store
        .create_user(
            "runtime-adapter-consumer@example.com",
            "secret1",
            Some("Runtime Adapter Consumer"),
            None,
        )
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Consumer AI App", None, None)
        .unwrap()
        .project;
    let consumer_app = store
        .create_open_commerce_developer_app(
            &consumer_project.id,
            &consumer.id,
            CreateDeveloperAppRequest {
                app_id: CONSUMER_APP_ID.to_string(),
                display_name: "Consumer Runtime Adapter AI".to_string(),
            },
        )
        .unwrap();
    assert_consumer_app(&consumer_app);
    let consumer_token = session(&store, &consumer.id);
    let merchant_owner_token = session(&store, &merchant_owner.id);

    let root = std::env::temp_dir().join(format!(
        "elon-open-commerce-runtime-adapter-tcp-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let state = Arc::new(test_app_state(store, &root));
    let platform_server = TcpServer::start(router::build_app(Arc::clone(&state))).await;
    let base_url = format!("http://{}", platform_server.address);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let action = json!({
        "merchant_id": merchant.id,
        "capability_key": "order.commit",
        "requester_app_id": CONSUMER_APP_ID,
        "idempotency_key": "consumer-runtime-adapter-order-1",
        "input":{"quote_id":"quote-consumer-runtime-1"}
    });

    let prepared = consumer_post(
        &client,
        &format!("{base_url}/api/open-commerce/action-confirmations"),
        &consumer_token,
        &action,
    )
    .await;
    let confirmation_id = prepared["id"].as_str().unwrap();
    let confirmed = consumer_post(
        &client,
        &format!("{base_url}/api/open-commerce/action-confirmations/{confirmation_id}/confirm"),
        &consumer_token,
        &json!({"confirmation_phrase":ACTION_CONFIRMATION_PHRASE}),
    )
    .await;
    assert_eq!(confirmed["status"], "confirmed");

    let mut invocation = action;
    invocation["action_confirmation_id"] = json!(confirmation_id);
    let committed = consumer_post(
        &client,
        &format!("{base_url}/api/open-commerce/invoke"),
        &consumer_token,
        &invocation,
    )
    .await;
    let invocation_id = committed["invocation_id"].as_str().unwrap();
    assert_eq!(committed["result"]["order_id"], "order-consumer-runtime-1");
    assert_eq!(committed["settlement_receipt"]["funds_moved"], false);

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
    assert_eq!(
        claimed["issue"]["task"]["result"]["order_id"],
        "order-consumer-runtime-1"
    );
    assert_eq!(
        claimed["issue"]["task"]["evidence"]["source_authority"],
        "merchant_runtime_asserted"
    );
    let claim_id = claimed["issue"]["claim"]["id"].as_str().unwrap();
    let lease_token = claimed["issue"]["lease_token"].as_str().unwrap();
    let target_reference = "erp-order-consumer-runtime-1";
    let completed = adapter_post(
        &client,
        &format!("{claims_url}/{claim_id}/complete"),
        &adapter_token,
        &json!({
            "lease_token":lease_token,
            "receipt_key":"consumer-runtime-adapter-applied-1",
            "status":"applied",
            "target_domain":"erp",
            "target_reference":target_reference,
            "completed_at":Utc::now().to_rfc3339()
        }),
    )
    .await;
    assert_eq!(completed["invocation_id"], invocation_id);
    assert_eq!(completed["status"], "applied");
    assert_eq!(completed["funds_moved"], false);
    assert_eq!(
        completed["target_reference_sha256"].as_str().unwrap().len(),
        64
    );
    assert!(!completed.to_string().contains(target_reference));
    assert!(!completed.to_string().contains(lease_token));

    let listed = client
        .get(format!(
            "{base_url}/api/projects/{}/open-commerce/adapter-handoff-claims",
            merchant_project.id
        ))
        .bearer_auth(&merchant_owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed: Value = listed.json().await.unwrap();
    assert_eq!(listed["claims"].as_array().unwrap().len(), 1);
    assert_eq!(listed["claims"][0]["invocation_id"], invocation_id);
    assert_eq!(listed["claims"][0]["status"], "completed");

    assert_eq!(runtime_state.invocation_count.load(Ordering::SeqCst), 2);
    let envelopes = runtime_state.envelopes.lock().unwrap();
    let order = envelopes
        .iter()
        .find(|value| value["capability_key"] == "order.commit")
        .unwrap();
    assert_eq!(order["requester_user_id"], consumer.id);
    assert_eq!(order["requester_app_id"], CONSUMER_APP_ID);
    assert_eq!(order["credential_environment"], "platform");
    assert_eq!(order["action_confirmation_id"], confirmation_id);
    drop(envelopes);

    platform_server.stop().await;
    runtime_server.stop().await;
    std::env::remove_var(RUNTIME_SECRET_REF);
}

fn assert_consumer_app(secret: &OpenCommerceDeveloperAppCredential) {
    assert_eq!(secret.app.app_id, CONSUMER_APP_ID);
    assert!(secret.test_token.starts_with("oc_test_"));
}

async fn consumer_post(client: &reqwest::Client, url: &str, token: &str, body: &Value) -> Value {
    let response = client
        .post(url)
        .bearer_auth(token)
        .header("x-elon-app-id", CONSUMER_APP_ID)
        .json(body)
        .send()
        .await
        .unwrap();
    response_json(response).await
}

async fn adapter_post(client: &reqwest::Client, url: &str, token: &str, body: &Value) -> Value {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .unwrap();
    response_json(response).await
}

async fn response_json(response: reqwest::Response) -> Value {
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn runtime_handler(
    State(state): State<RuntimeState>,
    headers: HeaderMap,
    body: Bytes,
) -> Json<Value> {
    state.invocation_count.fetch_add(1, Ordering::SeqCst);
    let timestamp = headers
        .get("x-yilong-runtime-timestamp")
        .and_then(|value| value.to_str().ok())
        .unwrap();
    let signature = headers
        .get("x-yilong-runtime-signature")
        .and_then(|value| value.to_str().ok())
        .unwrap();
    let expected =
        crate::open_commerce_runtime_client::test_signature(RUNTIME_SECRET, timestamp, &body);
    assert_eq!(signature, format!("v1={expected}"));
    assert_eq!(
        headers
            .get("x-yilong-runtime-key-id")
            .and_then(|value| value.to_str().ok()),
        Some(RUNTIME_SECRET_REF)
    );
    let envelope: Value = serde_json::from_slice(&body).unwrap();
    state.envelopes.lock().unwrap().push(envelope.clone());
    let capability_key = envelope["capability_key"].as_str().unwrap();
    let result = if capability_key == "system.health" {
        json!({
            "merchant_id":envelope["merchant_id"],
            "status":"ok",
            "manifest_sha256":RUNTIME_MANIFEST_SHA256
        })
    } else {
        json!({
            "order_id":"order-consumer-runtime-1",
            "status":"confirmed",
            "_yilong_business_receipt":{
                "schema":BUSINESS_RECEIPT_SCHEMA,
                "entity_type":"order",
                "reference_id":"order-consumer-runtime-1",
                "state":"confirmed",
                "occurred_at":"2026-08-13T03:00:00Z",
                "amount_minor":2600,
                "currency":"CNY"
            }
        })
    };
    Json(json!({
        "schema":"merchant_runtime.result.v1",
        "invocation_id":envelope["invocation_id"],
        "capability_key":capability_key,
        "result":result
    }))
}

fn owner_actor(user_id: &str) -> OpenCommerceActor<'_> {
    OpenCommerceActor {
        user_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    }
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("runtime-adapter-tcp-test"), None)
        .unwrap()
        .0
}

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_runtime_adapter_tcp_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("runtime adapter TCP test store should open")
}

#[path = "open_commerce_production_runtime_adapter_tcp_tests.rs"]
mod production_credential_tests;
