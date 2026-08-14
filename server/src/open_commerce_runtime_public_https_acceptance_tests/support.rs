use axum::Router;
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

use crate::open_commerce_model::{
    CreateCapabilityRequest, ACCESS_AUTHORIZED, ACCESS_PUBLIC, HANDLER_MERCHANT_RUNTIME,
};

pub(super) fn capabilities() -> Vec<CreateCapabilityRequest> {
    vec![
        CreateCapabilityRequest {
            capability_key: "catalog.search".to_string(),
            display_name: "Search catalog".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({
                "type":"object",
                "properties":{
                    "query":{"type":"string","maxLength":120},
                    "limit":{"type":"integer","minimum":1,"maximum":50}
                },
                "additionalProperties":false
            }),
            output_schema: json!({"type":"object"}),
            handler_type: HANDLER_MERCHANT_RUNTIME.to_string(),
            handler_config: None,
            unit_price_micros: 250,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
        CreateCapabilityRequest {
            capability_key: "order.quote.create".to_string(),
            display_name: "Create quote".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_AUTHORIZED.to_string(),
            input_schema: json!({
                "type":"object",
                "required":["items"],
                "properties":{
                    "items":{"type":"array","minItems":1,"maxItems":50,"items":{
                        "type":"object","required":["product_id","quantity"],
                        "properties":{"product_id":{"type":"string","format":"uuid"},"quantity":{"type":"integer","minimum":1,"maximum":100}},
                        "additionalProperties":false
                    }},
                    "note":{"type":"string","maxLength":500}
                },
                "additionalProperties":false
            }),
            output_schema: json!({"type":"object"}),
            handler_type: HANDLER_MERCHANT_RUNTIME.to_string(),
            handler_config: None,
            unit_price_micros: 1_000,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
        CreateCapabilityRequest {
            capability_key: "order.commit".to_string(),
            display_name: "Commit order".to_string(),
            description: String::new(),
            kind: "action".to_string(),
            access_level: ACCESS_AUTHORIZED.to_string(),
            input_schema: json!({
                "type":"object","required":["quote_id"],
                "properties":{"quote_id":{"type":"string","format":"uuid"}},
                "additionalProperties":false
            }),
            output_schema: json!({"type":"object"}),
            handler_type: HANDLER_MERCHANT_RUNTIME.to_string(),
            handler_config: None,
            unit_price_micros: 2_000,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
        CreateCapabilityRequest {
            capability_key: "order.status.read".to_string(),
            display_name: "Read order status".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_AUTHORIZED.to_string(),
            input_schema: json!({
                "type":"object","required":["order_id"],
                "properties":{"order_id":{"type":"string","format":"uuid"}},
                "additionalProperties":false
            }),
            output_schema: json!({"type":"object"}),
            handler_type: HANDLER_MERCHANT_RUNTIME.to_string(),
            handler_config: None,
            unit_price_micros: 500,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
    ]
}

pub(super) async fn bearer_post(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    body: &Value,
) -> Value {
    post(client, url, token, body, "bearer").await
}

pub(super) async fn session_post(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    body: &Value,
) -> Value {
    post(client, url, token, body, "session").await
}

pub(super) async fn discover_capability(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    requester_app_id: &str,
    capability_key: &str,
) -> Value {
    session_post(
        client,
        &format!("{base_url}/api/open-commerce/sandbox/discover"),
        token,
        &json!({
            "query":"Public HTTPS Coffee Acceptance",
            "capability_key":capability_key,
            "requester_app_id":requester_app_id,
            "ranking_policy":"merchant_name.v1",
            "include_ranking_receipt":true,
            "limit":10
        }),
    )
    .await
}

async fn post(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    body: &Value,
    identity: &str,
) -> Value {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let value: Value = response.json().await.unwrap();
    assert!(status.is_success(), "{identity} request failed: {value}");
    value
}

pub(super) async fn session_get(client: &reqwest::Client, url: &str, token: &str) -> Value {
    let response = client.get(url).bearer_auth(token).send().await.unwrap();
    let status = response.status();
    let value: Value = response.json().await.unwrap();
    assert!(status.is_success(), "session request failed: {value}");
    value
}

pub(super) fn required_env(name: &str) -> String {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("required acceptance environment is missing: {name}"))
}

pub(super) fn redact(value: &str) -> String {
    let mut redacted = value.to_string();
    if let Ok(secret) = std::env::var("OPEN_COMMERCE_RUNTIME_SECRET_COFFICE") {
        redacted = redacted.replace(&secret, "<redacted>");
    }
    redacted
}

pub(super) struct TcpServer {
    pub(super) address: std::net::SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl TcpServer {
    pub(super) async fn start(app: Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
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

    pub(super) async fn stop(self) {
        let _ = self.shutdown.send(());
        self.task.await.unwrap();
    }
}
