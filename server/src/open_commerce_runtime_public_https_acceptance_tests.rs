#[path = "open_commerce_runtime_public_https_acceptance_tests/support.rs"]
mod support;

use std::{collections::BTreeMap, process::Command, sync::Arc};

use chrono::{Duration, Utc};
use serde_json::json;
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
    open_commerce_model::CreateMerchantRequest,
    open_commerce_runtime_model::UpsertRuntimeBindingRequest,
    open_commerce_runtime_service,
    open_commerce_service::{self, OpenCommerceActor},
    router,
};

use support::{
    bearer_post, capabilities, discover_capability, redact, required_env, session_get,
    session_post, TcpServer,
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
    let secret = required_env(RUNTIME_SECRET_REF);
    assert_eq!(endpoint, "https://182.254.168.75");
    assert!(secret.len() >= 32);

    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD_TEST, "--nocapture"])
        .env(CHILD_ENV, "1")
        .env(PRODUCTION_CREDENTIAL_ENV, "1")
        .env(ACCEPTANCE_ENV, ACKNOWLEDGEMENT)
        .env("ELON_OPEN_COMMERCE_PUBLIC_HTTPS_ENDPOINT", endpoint)
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
    let developer = approved_developer_fixture_for(
        APP_ID,
        &[
            "catalog.search",
            "order.quote.create",
            "order.commit",
            "order.status.read",
        ],
    );
    let live = open_commerce_developer_credential_service::issue_credential(
        &developer.store,
        &developer.app.id,
        IssueDeveloperProductionCredentialRequest {
            expected_manifest_revision: developer.app.manifest_revision,
            scopes: vec![
                "catalog.search".to_string(),
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
    let merchant_session = developer
        .store
        .create_session(
            &merchant_owner.id,
            Some("public-https-merchant-approval"),
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
    let mut discovered = BTreeMap::new();
    for capability_key in [
        "catalog.search",
        "order.quote.create",
        "order.commit",
        "order.status.read",
    ] {
        let discovery = discover_capability(
            &client,
            &base_url,
            &consumer_session,
            APP_ID,
            capability_key,
        )
        .await;
        assert_eq!(discovery["schema"], "open_commerce.consumer_discovery.v1");
        assert_eq!(discovery["ranking_policy"], "merchant_name.v1");
        assert_eq!(discovery["ranking_is_paid"], false);
        let public_json = discovery.to_string();
        assert!(!public_json.contains(&merchant_project.id));
        assert!(!public_json.contains("https://182.254.168.75"));
        assert!(!public_json.contains(RUNTIME_SECRET_REF));
        assert!(!public_json.contains("handler_type"));
        let matches = discovery["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        let candidate = &matches[0];
        assert_eq!(candidate["merchant"]["id"], MERCHANT_ID);
        assert_eq!(candidate["capability"]["capability_key"], capability_key);
        discovered.insert(capability_key.to_string(), candidate.clone());
    }
    assert_eq!(discovered.len(), 4);
    assert_eq!(
        discovered["catalog.search"]["authorization"]["status"],
        "not_required"
    );
    let order_scopes = ["order.quote.create", "order.commit", "order.status.read"];
    for capability_key in order_scopes {
        assert_eq!(
            discovered[capability_key]["authorization"]["status"],
            "request_required"
        );
    }

    let catalog = bearer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &json!({
            "merchant_id":discovered["catalog.search"]["merchant"]["id"],
            "capability_key":discovered["catalog.search"]["capability"]["capability_key"],
            "idempotency_key":format!("public-catalog-{run_id}"),
            "input":{"query":format!("YILONG-PUBLIC-HTTPS-{run_id}"),"limit":5}
        }),
    )
    .await;
    assert_eq!(catalog["settlement_receipt"]["funds_moved"], false);
    let catalog_items = catalog["result"]["items"].as_array().unwrap();
    assert_eq!(catalog_items.len(), 1);
    assert_eq!(
        catalog_items[0]["sku"],
        format!("YILONG-PUBLIC-HTTPS-{run_id}")
    );
    let offer_id = catalog_items[0]["id"].as_str().unwrap().to_string();
    assert!(Uuid::parse_str(&offer_id).is_ok());

    let authorization = session_post(
        &client,
        &format!("{base_url}/api/open-commerce/authorization-requests"),
        &consumer_session,
        &json!({
            "merchant_id":MERCHANT_ID,
            "requester_app_id":APP_ID,
            "scopes":order_scopes,
            "purpose":"one unpaid order in the suspended acceptance store"
        }),
    )
    .await;
    assert_eq!(authorization["status"], "pending");
    let authorization_request_id = authorization["id"].as_str().unwrap();
    let approved = session_post(
        &client,
        &format!(
            "{base_url}/api/projects/{}/open-commerce/authorization-requests/{authorization_request_id}/approve",
            merchant_project.id
        ),
        &merchant_session,
        &json!({
            "reason":"dedicated suspended-store acceptance",
            "expires_at":(Utc::now() + Duration::hours(1)).to_rfc3339(),
            "max_invocations":3,
            "max_amount_micros":6000,
            "budget_currency":"CNY"
        }),
    )
    .await;
    assert_eq!(approved["status"], "approved");
    let grant_id = approved["grant_id"].as_str().unwrap().to_string();

    for capability_key in order_scopes {
        let discovery = discover_capability(
            &client,
            &base_url,
            &consumer_session,
            APP_ID,
            capability_key,
        )
        .await;
        let matches = discovery["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["authorization"]["status"], "granted");
        assert_eq!(matches[0]["authorization"]["grant_id"], grant_id);
    }

    let quote = bearer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &json!({
            "merchant_id":discovered["order.quote.create"]["merchant"]["id"],
            "capability_key":discovered["order.quote.create"]["capability"]["capability_key"],
            "grant_id":grant_id,
            "idempotency_key":format!("public-quote-{run_id}"),
            "input":{"items":[{"product_id":offer_id,"quantity":1}],"note":"public HTTPS acceptance"}
        }),
    )
    .await;
    assert_eq!(quote["settlement_receipt"]["funds_moved"], false);
    let quote_id = quote["result"]["quote_id"].as_str().unwrap();

    let action = json!({
        "merchant_id":discovered["order.commit"]["merchant"]["id"],
        "capability_key":discovered["order.commit"]["capability"]["capability_key"],
        "grant_id":grant_id,
        "idempotency_key":format!("public-commit-{run_id}"),
        "input":{"quote_id":quote_id}
    });
    let prepared = bearer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/action-confirmations"),
        &live.live_token,
        &action,
    )
    .await;
    let confirmation_id = prepared["id"].as_str().unwrap();
    let confirmed = bearer_post(
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
    let committed = bearer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &invocation,
    )
    .await;
    let replay = bearer_post(
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

    let order_status = bearer_post(
        &client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        &live.live_token,
        &json!({
            "merchant_id":discovered["order.status.read"]["merchant"]["id"],
            "capability_key":discovered["order.status.read"]["capability"]["capability_key"],
            "grant_id":grant_id,
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

    let closure = session_get(
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
        "discovered_offer_id":offer_id,
        "directory_discovery":true,
        "ranking_policy":"merchant_name.v1",
        "ranking_is_paid":false,
        "discovered_capability_count":discovered.len(),
        "authorization_request_id":authorization_request_id,
        "authorization_status":"approved",
        "grant_id":grant_id,
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
