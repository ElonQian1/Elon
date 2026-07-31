use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{body::Bytes, extract::State, http::HeaderMap, routing::post, Json, Router};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, InvokeCapabilityRequest, ACCESS_PUBLIC,
        HANDLER_MERCHANT_RUNTIME,
    },
    open_commerce_runtime_model::UpsertRuntimeBindingRequest,
    open_commerce_runtime_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

const SECRET_REF: &str = "OPEN_COMMERCE_RUNTIME_SECRET_TEST_E2E";
const SECRET: &str = "test-only-runtime-secret-at-least-32-bytes";
const MANIFEST_SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[derive(Clone)]
struct RuntimeState {
    invocation_count: Arc<AtomicUsize>,
}

#[tokio::test]
async fn verified_runtime_is_signed_metered_audited_and_idempotent() {
    std::env::set_var(SECRET_REF, SECRET);
    let runtime_state = RuntimeState {
        invocation_count: Arc::new(AtomicUsize::new(0)),
    };
    let app = Router::new()
        .route("/commerce/v1/invoke", post(runtime_handler))
        .with_state(runtime_state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let runtime_task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let store = temp_store();
    let owner = store
        .create_user(
            "runtime-owner@example.com",
            "secret1",
            Some("Runtime Owner"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Runtime Project", None, None)
        .unwrap()
        .project;
    let actor = OpenCommerceActor {
        user_id: &owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let merchant = open_commerce_service::create_merchant(
        &store,
        &project.id,
        &actor,
        CreateMerchantRequest {
            display_name: "真实咖啡店".to_string(),
            slug: Some("runtime-coffee".to_string()),
            description: String::new(),
            node_mode: "self_hosted".to_string(),
            public_profile: json!({"category":"coffee"}),
        },
    )
    .unwrap();
    open_commerce_runtime_service::upsert_binding(
        &store,
        &project.id,
        &merchant.id,
        &actor,
        UpsertRuntimeBindingRequest {
            endpoint_base_url: format!("http://{address}"),
            credential_ref: SECRET_REF.to_string(),
            manifest_sha256: Some(MANIFEST_SHA256.to_string()),
            timeout_ms: 2_000,
        },
    )
    .unwrap();
    let verified =
        open_commerce_runtime_service::verify_binding(&store, &project.id, &merchant.id, &actor)
            .await
            .unwrap();
    assert_eq!(verified.status, "active");

    open_commerce_service::publish_capability(
        &store,
        &project.id,
        &merchant.id,
        &actor,
        CreateCapabilityRequest {
            capability_key: "catalog.search".to_string(),
            display_name: "搜索在售商品".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            handler_type: HANDLER_MERCHANT_RUNTIME.to_string(),
            handler_config: None,
            unit_price_micros: 1_000,
            currency: "CNY".to_string(),
            freshness_seconds: 30,
        },
    )
    .unwrap();

    let request = || InvokeCapabilityRequest {
        merchant_id: merchant.id.clone(),
        capability_key: "catalog.search".to_string(),
        requester_app_id: "pc-web".to_string(),
        grant_id: None,
        idempotency_key: "catalog-search-runtime-e2e".to_string(),
        input: json!({"query":"拿铁"}),
    };
    let first = open_commerce_service::invoke(&store, &actor, request())
        .await
        .unwrap();
    let replay = open_commerce_service::invoke(&store, &actor, request())
        .await
        .unwrap();

    assert_eq!(first["result"]["items"][0]["product_id"], "coffee-latte");
    assert_eq!(first["metering"]["amount_micros"], 1_000);
    assert_eq!(
        first["metering"]["settlement_status"],
        "recorded_not_charged"
    );
    assert_eq!(
        first["settlement_receipt"]["schema"],
        "open_commerce.settlement_receipt.v1"
    );
    assert_eq!(first["settlement_receipt"]["funds_moved"], false);
    assert_eq!(first["settlement_receipt"]["amount_micros"], 1_000);
    assert_eq!(replay["replayed"], true);
    assert_eq!(runtime_state.invocation_count.load(Ordering::SeqCst), 2);
    assert!(store
        .list_project_open_commerce_audit(&project.id, 20)
        .unwrap()
        .iter()
        .any(|event| event.action == "runtime.verified"));

    runtime_task.abort();
    std::env::remove_var(SECRET_REF);
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
    let actual_signature = headers
        .get("x-yilong-runtime-signature")
        .and_then(|value| value.to_str().ok())
        .unwrap();
    let expected_signature =
        crate::open_commerce_runtime_client::test_signature(SECRET, timestamp, &body);
    assert_eq!(actual_signature, format!("v1={expected_signature}"));
    assert_eq!(
        headers
            .get("x-yilong-runtime-key-id")
            .and_then(|value| value.to_str().ok()),
        Some(SECRET_REF)
    );

    let envelope: Value = serde_json::from_slice(&body).unwrap();
    let capability_key = envelope["capability_key"].as_str().unwrap();
    let result = if capability_key == "system.health" {
        json!({
            "merchant_id": envelope["merchant_id"],
            "status": "ok",
            "manifest_sha256": MANIFEST_SHA256
        })
    } else {
        json!({
            "items":[{
                "product_id":"coffee-latte",
                "name":"拿铁",
                "unit_price_minor":2600,
                "currency":"CNY"
            }]
        })
    };
    Json(json!({
        "schema":"merchant_runtime.result.v1",
        "invocation_id":envelope["invocation_id"],
        "capability_key":capability_key,
        "result":result
    }))
}

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_runtime_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("runtime test store should open")
}
