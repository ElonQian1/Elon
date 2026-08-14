use std::{process::Command, sync::Arc};

use axum::Router;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};
use uuid::Uuid;

use crate::{
    open_commerce_action_confirmation_model::ACTION_CONFIRMATION_PHRASE,
    open_commerce_developer_credential_model::{
        IssueDeveloperProductionCredentialRequest, PRODUCTION_CREDENTIAL_ENV,
    },
    open_commerce_developer_credential_service,
    open_commerce_developer_production_test_support::{
        approved_developer_fixture_for, test_app_state,
    },
    open_commerce_directory_service,
    open_commerce_merchant_evidence_model::validate_optional_business_receipt,
    open_commerce_model::{
        CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest, ACCESS_AUTHORIZED,
        HANDLER_MERCHANT_RUNTIME,
    },
    open_commerce_runtime_model::UpsertRuntimeBindingRequest,
    open_commerce_runtime_service,
    open_commerce_service::{self, OpenCommerceActor},
    router,
};

const CHILD_ENV: &str = "ELON_OPEN_COMMERCE_PUBLIC_HTTPS_CHILD";
const ACCEPTANCE_ENV: &str = "ELON_OPEN_COMMERCE_PUBLIC_HTTPS_ACCEPTANCE";
const ACKNOWLEDGEMENT: &str = "I_ACCEPT_ONE_UNPAID_ORDER_IN_THE_SUSPENDED_ACCEPTANCE_STORE";
const APP_ID: &str = "consumer.public.https.acceptance.ai";
const MERCHANT_ID: &str = "merchant-cofficethinking-acceptance";
const RUNTIME_SECRET_REF: &str = "OPEN_COMMERCE_RUNTIME_SECRET_COFFICE";
const CHILD_TEST: &str = "open_commerce_runtime_service_tests::public_https_acceptance_tests::real_consumer_ai_order_reaches_public_coffee_erp_child";

#[test]
fn real_consumer_ai_order_reaches_public_coffee_erp() {
    if std::env::var(ACCEPTANCE_ENV).as_deref() != Ok(ACKNOWLEDGEMENT) {
        eprintln!(
            "public HTTPS acceptance skipped: explicit production-write acknowledgement missing"
        );
        return;
    }
    let endpoint = required_env("ELON_OPEN_COMMERCE_PUBLIC_HTTPS_ENDPOINT");
    let offer_id = required_env("ELON_OPEN_COMMERCE_PUBLIC_HTTPS_OFFER_ID");
    let secret = required_env(RUNTIME_SECRET_REF);
    assert_eq!(endpoint, "https://182.254.168.75");
    assert!(Uuid::parse_str(&offer_id).is_ok());
    assert!(secret.len() >= 32);

    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD_TEST, "--nocapture"])
        .env(CHILD_ENV, "1")
        .env(PRODUCTION_CREDENTIAL_ENV, "1")
        .env(ACCEPTANCE_ENV, ACKNOWLEDGEMENT)
        .env("ELON_OPEN_COMMERCE_PUBLIC_HTTPS_ENDPOINT", endpoint)
        .env("ELON_OPEN_COMMERCE_PUBLIC_HTTPS_OFFER_ID", offer_id)
        .env(RUNTIME_SECRET_REF, secret)
        .env("OPEN_COMMERCE_RUNTIME_ALLOWED_HOSTS", "182.254.168.75")
        .output()
        .expect("launch isolated public HTTPS acceptance");
    assert!(
        output.status.success(),
        "public HTTPS acceptance failed: stdout={} stderr={}",
        redact(&String::from_utf8_lossy(&output.stdout)),
        redact(&String::from_utf8_lossy(&output.stderr))
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

#[tokio::test]
async fn real_consumer_ai_order_reaches_public_coffee_erp_child() {
    if std::env::var(CHILD_ENV).as_deref() != Ok("1") {
        return;
    }
    assert_eq!(required_env(ACCEPTANCE_ENV), ACKNOWLEDGEMENT);
    let endpoint = required_env("ELON_OPEN_COMMERCE_PUBLIC_HTTPS_ENDPOINT");
    let offer_id = required_env("ELON_OPEN_COMMERCE_PUBLIC_HTTPS_OFFER_ID");

    let developer = approved_developer_fixture_for(
        APP_ID,
        &["order.quote.create", "order.commit", "order.status.read"],
    );
    let live = open_commerce_developer_credential_service::issue_credential(
        &developer.store,
        &developer.app.id,
        IssueDeveloperProductionCredentialRequest {
            expected_manifest_revision: developer.app.manifest_revision,
            scopes: vec![
                "order.quote.create".to_string(),
                "order.commit".to_string(),
                "order.status.read".to_string(),
            ],
            expires_in_days: 1,
        },
        "public-https-acceptance-reviewer",
    )
    .unwrap();

    let merchant_owner = developer
        .store
        .create_user(
            "public-https-merchant@example.test",
            "secret1",
            Some("Public HTTPS Merchant"),
            None,
        )
        .unwrap();
    let merchant_project = developer
        .store
        .create_project(
            &merchant_owner.id,
            "Public HTTPS Coffee Merchant",
            None,
            None,
        )
        .unwrap()
        .project;
    let merchant_actor = OpenCommerceActor {
        user_id: &merchant_owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let merchant = open_commerce_service::create_merchant(
        &developer.store,
        &merchant_project.id,
        &merchant_actor,
        CreateMerchantRequest {
            display_name: "Public HTTPS Coffee Acceptance".to_string(),
            slug: Some("public-https-coffee-acceptance".to_string()),
            description: String::new(),
            node_mode: "self_hosted".to_string(),
            public_profile: json!({"acceptance_only":true}),
        },
    )
    .unwrap();
    // The external runtime has a fixed merchant identity. Keep the platform row aligned without
    // changing the production database or introducing a second order ledger.
    developer
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_merchants SET id = ?1 WHERE id = ?2",
            rusqlite::params![MERCHANT_ID, merchant.id],
        )
        .unwrap();

    open_commerce_runtime_service::upsert_binding(
        &developer.store,
        &merchant_project.id,
        MERCHANT_ID,
        &merchant_actor,
        UpsertRuntimeBindingRequest {
            endpoint_base_url: endpoint,
            credential_ref: RUNTIME_SECRET_REF.to_string(),
            manifest_sha256: None,
            timeout_ms: 10_000,
        },
    )
    .unwrap();
    let verified = open_commerce_runtime_service::verify_binding(
        &developer.store,
        &merchant_project.id,
        MERCHANT_ID,
        &merchant_actor,
    )
    .await
    .unwrap();
    assert_eq!(verified.status, "active");

    for capability in capabilities() {
        open_commerce_service::publish_capability(
            &developer.store,
            &merchant_project.id,
            MERCHANT_ID,
            &merchant_actor,
            capability,
        )
        .unwrap();
    }
    open_commerce_directory_service::set_publication(
        &developer.store,
        &merchant_project.id,
        MERCHANT_ID,
        &merchant_actor,
        true,
    )
    .unwrap();
    let grant = open_commerce_service::create_grant(
        &developer.store,
        &merchant_project.id,
        &merchant_actor,
        CreateGrantRequest {
            merchant_id: MERCHANT_ID.to_string(),
            grantee_app_id: APP_ID.to_string(),
            scopes: vec![
                "order.quote.create".to_string(),
                "order.commit".to_string(),
                "order.status.read".to_string(),
            ],
            purpose: "isolated public HTTPS acceptance".to_string(),
            expires_at: Some((Utc::now() + Duration::hours(1)).to_rfc3339()),
            max_invocations: Some(3),
            max_amount_micros: Some(6_000),
            budget_currency: "CNY".to_string(),
        },
    )
    .unwrap();

    let root = std::env::temp_dir().join(format!(
        "elon_public_https_acceptance_{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let consumer_session = developer
        .store
        .create_session(
            &developer.owner_user_id,
            Some("public-https-acceptance"),
            None,
        )
        .unwrap()
        .0;
    let server = TcpServer::start(router::build_app(Arc::new(test_app_state(
        developer.store,
        &root,
    ))))
    .await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let base_url = format!("http://{}", server.address);
    let run_id = required_env("ELON_OPEN_COMMERCE_PUBLIC_HTTPS_RUN_ID");

    let quote = developer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &json!({
            "merchant_id":MERCHANT_ID,
            "capability_key":"order.quote.create",
            "grant_id":grant.id,
            "idempotency_key":format!("public-quote-{run_id}"),
            "input":{"items":[{"product_id":offer_id,"quantity":1}],"note":"public HTTPS acceptance"}
        }),
    )
    .await;
    assert_eq!(quote["settlement_receipt"]["funds_moved"], false);
    let quote_id = quote["result"]["quote_id"].as_str().unwrap();

    let action = json!({
        "merchant_id":MERCHANT_ID,
        "capability_key":"order.commit",
        "grant_id":grant.id,
        "idempotency_key":format!("public-commit-{run_id}"),
        "input":{"quote_id":quote_id}
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
    let replay = developer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &invocation,
    )
    .await;
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["invocation_id"], committed["invocation_id"]);
    assert_eq!(committed["result"]["payment_status"], "unpaid");
    assert_eq!(committed["settlement_receipt"]["funds_moved"], false);
    let invocation_id = committed["invocation_id"].as_str().unwrap();
    let order_id = committed["result"]["order_id"].as_str().unwrap();
    let merchant_order_id = committed["result"]["merchant_order_id"].as_str().unwrap();
    let receipt = validate_optional_business_receipt(&committed["result"])
        .unwrap()
        .unwrap();
    assert_eq!(receipt.entity_type, "order");
    assert_eq!(receipt.reference_id, merchant_order_id);

    let order_status = developer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &json!({
            "merchant_id":MERCHANT_ID,
            "capability_key":"order.status.read",
            "grant_id":grant.id,
            "idempotency_key":format!("public-status-{run_id}"),
            "input":{"order_id":order_id}
        }),
    )
    .await;
    assert_eq!(order_status["result"]["order_id"], order_id);
    assert_eq!(
        order_status["result"]["merchant_order_id"],
        merchant_order_id
    );
    assert_eq!(order_status["result"]["payment_status"], "unpaid");
    assert_eq!(order_status["settlement_receipt"]["funds_moved"], false);

    let closure = authenticated_get(
        &client,
        &format!("{base_url}/api/open-commerce/consumer-order-closures/{invocation_id}"),
        &consumer_session,
    )
    .await;
    assert_eq!(closure["merchant_order"]["reference_id"], merchant_order_id);
    assert_eq!(closure["closure_status"], "merchant_confirmed_erp_pending");
    assert_eq!(closure["funds_moved"], false);

    let acceptance_receipt = json!({
        "schema":"open_commerce.public_https_acceptance.v1",
        "run_id":run_id,
        "endpoint":required_env("ELON_OPEN_COMMERCE_PUBLIC_HTTPS_ENDPOINT"),
        "merchant_id":MERCHANT_ID,
        "invocation_id":invocation_id,
        "order_id":order_id,
        "unified_order_id":merchant_order_id,
        "commit_idempotency_key":invocation["idempotency_key"],
        "payment_status":"unpaid",
        "funds_moved":false,
        "idempotent_replay":true,
        "order_status_read":true,
        "runtime_status":"active",
        "platform_store":"isolated_temporary_sqlite"
    });
    let receipt_path = required_env("ELON_OPEN_COMMERCE_PUBLIC_HTTPS_RECEIPT_PATH");
    std::fs::write(
        receipt_path,
        serde_json::to_vec_pretty(&acceptance_receipt).unwrap(),
    )
    .unwrap();
    println!("{acceptance_receipt}");
    server.stop().await;
}

fn capabilities() -> Vec<CreateCapabilityRequest> {
    vec![
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

async fn developer_post(client: &reqwest::Client, url: &str, token: &str, body: &Value) -> Value {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let value: Value = response.json().await.unwrap();
    assert!(status.is_success(), "developer request failed: {value}");
    value
}

async fn authenticated_get(client: &reqwest::Client, url: &str, token: &str) -> Value {
    let response = client.get(url).bearer_auth(token).send().await.unwrap();
    let status = response.status();
    let value: Value = response.json().await.unwrap();
    assert!(status.is_success(), "authenticated request failed: {value}");
    value
}

fn required_env(name: &str) -> String {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("required acceptance environment is missing: {name}"))
}

fn redact(value: &str) -> String {
    let mut redacted = value.to_string();
    for name in [RUNTIME_SECRET_REF] {
        if let Ok(secret) = std::env::var(name) {
            redacted = redacted.replace(&secret, "<redacted>");
        }
    }
    redacted
}

struct TcpServer {
    address: std::net::SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl TcpServer {
    async fn start(app: Router) -> Self {
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

    async fn stop(self) {
        let _ = self.shutdown.send(());
        self.task.await.unwrap();
    }
}
