//! Real production Router/Store and loopback HTTP; all identities and payments are synthetic.
#[path = "reconciliation_auth_http_tests.rs"]
mod auth_tests;
#[path = "reconciliation_http_support.rs"]
mod support;

use super::*;
use std::time::Duration;
use support::*;

#[tokio::test]
async fn prepared_recorded_canceled_and_reprepared_keys_are_exact_and_private() {
    let fixture = Fixture::new();
    let policy = enable_fixture_policy();
    let first = prepare(&fixture, 9).await;
    let prepared = get(&fixture).await;
    assert_eq!(prepared["prepared_count"], "1");
    assert_eq!(prepared["used_payment_keys"], json!([first["payment_key"]]));
    transition(&fixture, &first, "record").await;
    let recorded = get(&fixture).await;
    assert_eq!(recorded["prepared_count"], "0");
    assert_eq!(recorded["recorded_count"], "1");
    assert_eq!(recorded["used_payment_keys"], prepared["used_payment_keys"]);
    let second = prepare(&fixture, 1).await;
    assert_eq!(get(&fixture).await["key_count"], "2");
    transition(&fixture, &second, "cancel").await;
    let canceled = get(&fixture).await;
    assert_eq!(canceled["key_count"], "1");
    assert_eq!(canceled["used_payment_keys"], recorded["used_payment_keys"]);
    let replacement = prepare(&fixture, 1).await;
    assert_ne!(replacement["allocation_id"], second["allocation_id"]);
    assert_eq!(replacement["payment_key"], second["payment_key"]);
    drop(policy);
    let _disabled = PolicyGuard(POLICY_OVERRIDE.with(|value| value.replace(Some(None))));
    let before = business_state(&fixture);
    let snapshot = get(&fixture).await;
    assert_eq!(snapshot["prepared_count"], "1");
    assert_eq!(snapshot["recorded_count"], "1");
    assert_eq!(snapshot["key_count"], "2");
    assert_eq!(before, business_state(&fixture));
    assert_eq!(
        fixture
            .state
            .store
            .esk_account_ledger(&fixture.user_id)
            .unwrap()
            .total_base_units,
        9_000_000
    );
    assert_eq!(
        fixture
            .state
            .store
            .esk_platform_account(&fixture.user_id, &fixture.user_token, 20)
            .unwrap()
            .total_base_units,
        25_000_000
    );
    fixture.cleanup();
}

#[tokio::test]
async fn unpinned_policy_is_unavailable_without_creating_a_policy_or_empty_success() {
    let fixture = Fixture::new();
    let _configured = enable_fixture_policy();
    let before = business_state(&fixture);
    let (status, value) = raw(&fixture, "", auth(&fixture.admin_token), Body::empty()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(value, json!({"error": "ESK_PLATFORM_INVALID_POLICY"}));
    assert_eq!(before, business_state(&fixture));
    fixture.cleanup();
}

#[tokio::test]
async fn every_query_and_nonempty_get_body_is_rejected_without_echo_or_write() {
    let fixture = Fixture::new();
    let _policy = enable_fixture_policy();
    prepare(&fixture, 0).await;
    let before = business_state(&fixture);
    for suffix in [
        "?",
        "?limit=1",
        "?user_id=never-echo",
        "?cursor=never-echo",
        "?x=1&x=1",
        "?%75ser_id=never-echo",
    ] {
        let (status, value) =
            raw(&fixture, suffix, auth(&fixture.admin_token), Body::empty()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{suffix}");
        assert_eq!(value, json!({"error": "ESK_PLATFORM_INVALID_INPUT"}));
    }
    for body in [
        "null".to_owned(),
        "{}".into(),
        " ".into(),
        "never-echo".repeat(2000),
    ] {
        let (status, value) = raw(&fixture, "", auth(&fixture.admin_token), Body::from(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(value, json!({"error": "ESK_PLATFORM_INVALID_INPUT"}));
    }
    assert_eq!(before, business_state(&fixture));
    fixture.cleanup();
}

#[tokio::test]
async fn corrupt_policy_or_allocation_returns_private_fixed_failure_not_partial_history() {
    for policy in [true, false] {
        let fixture = Fixture::new();
        let _policy = enable_fixture_policy();
        prepare(&fixture, 0).await;
        let sql = if policy {
            "DROP TRIGGER trg_esk_platform_policy_no_update; UPDATE esk_platform_policy SET source_json='never-echo-corruption';"
        } else {
            "DROP TRIGGER trg_esk_platform_allocations_no_update; UPDATE esk_platform_allocations SET input_json='never-echo-corruption';"
        };
        fixture
            .state
            .store
            .conn()
            .unwrap()
            .execute_batch(sql)
            .unwrap();
        let before = business_state(&fixture);
        let (status, value) = raw(&fixture, "", auth(&fixture.admin_token), Body::empty()).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(value, json!({"error": "ESK_PLATFORM_LEDGER_INCONSISTENT"}));
        assert_eq!(before, business_state(&fixture));
        fixture.cleanup();
    }
}

#[tokio::test]
async fn real_loopback_http_reads_same_private_contract_and_leaves_business_rows_unchanged() {
    let fixture = Fixture::new();
    let policy = enable_fixture_policy();
    let prepared = prepare(&fixture, 0).await;
    transition(&fixture, &prepared, "record").await;
    drop(policy);
    let _disabled = PolicyGuard(POLICY_OVERRIDE.with(|value| value.replace(Some(None))));
    let before = business_state(&fixture);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    assert!(address.ip().is_loopback());
    let router = fixture.router.clone();
    let (shutdown, stopped) = tokio::sync::oneshot::channel();
    let mut server = AbortServer(tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    }));
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let url = format!("http://{address}{SNAPSHOT}");
    let denied = client.get(&url).send().await.unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    private(denied.headers());
    let denied: Value = denied.json().await.unwrap();
    assert_eq!(denied.as_object().unwrap().len(), 1);
    let response = client
        .get(&url)
        .bearer_auth(&fixture.admin_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    private(response.headers());
    let snapshot: Value = response.json().await.unwrap();
    assert_contract(&fixture, &snapshot);
    assert_eq!(
        snapshot["used_payment_keys"],
        json!([prepared["payment_key"]])
    );
    assert_eq!(snapshot["recorded_count"], "1");
    assert_eq!(before, business_state(&fixture));
    drop(client);
    shutdown.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), &mut server.0)
        .await
        .unwrap()
        .unwrap();
    drop(server);
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
    fixture.cleanup();
}
