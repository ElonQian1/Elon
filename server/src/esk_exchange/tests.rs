use std::{
    path::PathBuf,
    sync::{Arc, Barrier, Mutex},
};

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    esk_asset::{
        override_esk_asset_mode_for_test, EskAllocationInput, EskAssetMode, EskSellbackInput,
    },
    open_commerce_developer_production_test_support::test_app_state,
    store::{override_now_for_test, Store},
};

use super::*;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp_store(label: &str) -> (Store, PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon_esk_exchange_{label}_{}.db",
        Uuid::new_v4().simple()
    ));
    (
        Store::open(&path).expect("exchange store should open"),
        path,
    )
}

fn create_user(store: &Store, label: &str) -> crate::store::PublicUser {
    store
        .create_user(
            &format!(
                "esk-exchange-{label}-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            Some(label),
            None,
        )
        .expect("exchange user should be created")
}

fn credit(store: &Store, user_id: &str, units: i64, key: &str) {
    store
        .create_paper_usdt_credit(&PaperUsdtCreditInput {
            user_id: user_id.to_string(),
            amount_units: units,
            reference: "paper-usdt-test-credit".to_string(),
            idempotency_key: key.to_string(),
        })
        .unwrap();
}

fn allocate_esk(store: &Store, user_id: &str, units: i64, key: &str) {
    store
        .create_esk_paper_allocation(&EskAllocationInput {
            user_id: user_id.to_string(),
            amount_base_units: units,
            reference: "paper-esk-test-credit".to_string(),
            idempotency_key: key.to_string(),
        })
        .unwrap();
}

fn quote_input(
    user_id: &str,
    direction: EskExchangeDirection,
    input_units: i64,
    price_units: i64,
    fee_bps: u16,
) -> EskExchangeQuoteInput {
    let (gross_output_units, fee_units, net_output_units) =
        calculate_quote(direction, input_units, price_units, fee_bps).unwrap();
    EskExchangeQuoteInput {
        user_id: user_id.to_string(),
        direction,
        input_units,
        price_units,
        fee_bps,
        config_revision: "a".repeat(64),
        gross_output_units,
        fee_units,
        net_output_units,
    }
}

#[test]
fn both_exchange_directions_are_atomic_balanced_and_idempotent() {
    let (store, path) = temp_store("both");
    let user = create_user(&store, "both");
    allocate_esk(&store, &user.id, 20_000_000, "esk-seed-both");
    credit(&store, &user.id, 100_000_000, "usdt-seed-both");

    let buy_quote = store
        .create_esk_exchange_quote(&quote_input(
            &user.id,
            EskExchangeDirection::UsdtToEsk,
            10_000_000,
            2_000_000,
            30,
        ))
        .unwrap();
    let buy_input = EskExchangeExecutionInput {
        user_id: user.id.clone(),
        quote_id: buy_quote.quote_id.clone(),
        idempotency_key: "buy-esk-1".to_string(),
        config_revision: "a".repeat(64),
    };
    let buy = store.execute_esk_exchange(&buy_input).unwrap();
    assert!(!buy.replayed);
    assert!(store.execute_esk_exchange(&buy_input).unwrap().replayed);
    assert!(store
        .execute_esk_exchange(&EskExchangeExecutionInput {
            idempotency_key: "buy-esk-1".to_string(),
            quote_id: "other-quote".to_string(),
            user_id: user.id.clone(),
            config_revision: "a".repeat(64),
        })
        .unwrap_err()
        .to_string()
        .contains("幂等键"));
    assert_eq!(
        store
            .esk_exchange_account_ledger(&user.id)
            .unwrap()
            .usdt_units,
        90_000_000
    );
    assert_eq!(
        store.esk_account_ledger(&user.id).unwrap().total_base_units,
        24_985_000
    );

    let sell_quote = store
        .create_esk_exchange_quote(&quote_input(
            &user.id,
            EskExchangeDirection::EskToUsdt,
            4_000_000,
            2_000_000,
            30,
        ))
        .unwrap();
    store
        .execute_esk_exchange(&EskExchangeExecutionInput {
            user_id: user.id.clone(),
            quote_id: sell_quote.quote_id,
            idempotency_key: "sell-esk-1".to_string(),
            config_revision: "a".repeat(64),
        })
        .unwrap();
    assert_eq!(
        store
            .esk_exchange_account_ledger(&user.id)
            .unwrap()
            .usdt_units,
        97_976_000
    );
    assert_eq!(
        store.esk_account_ledger(&user.id).unwrap().total_base_units,
        20_985_000
    );
    assert_eq!(store.list_esk_exchanges(&user.id, 20).unwrap().len(), 2);

    let conn = store.conn().unwrap();
    for asset in ["ESK", "USDT"] {
        let sum: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount_units), 0) FROM esk_exchange_ledger_entries WHERE asset=?1",
            rusqlite::params![asset], |row| row.get(0),
        ).unwrap();
        assert_eq!(sum, 0, "{asset} postings must conserve units");
    }
    let fee_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM esk_exchange_ledger_entries WHERE entry_kind='platform_fee'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(fee_rows, 2);
    assert!(conn
        .execute(
            "UPDATE esk_exchange_ledger_entries SET amount_units=1 WHERE posting_group_id=?1",
            rusqlite::params![buy.execution_id],
        )
        .unwrap_err()
        .to_string()
        .contains("append-only"));
    drop(conn);
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn reservations_and_expired_quotes_fail_closed() {
    let (store, path) = temp_store("guards");
    let user = create_user(&store, "guards");
    allocate_esk(&store, &user.id, 10_000_000, "esk-seed-guards");
    store
        .create_esk_sellback_request(&EskSellbackInput {
            user_id: user.id.clone(),
            amount_base_units: 8_000_000,
            idempotency_key: "reserve-eight".to_string(),
        })
        .unwrap();
    assert!(store
        .create_esk_exchange_quote(&quote_input(
            &user.id,
            EskExchangeDirection::EskToUsdt,
            3_000_000,
            1_000_000,
            30,
        ))
        .unwrap_err()
        .to_string()
        .contains("超过"));

    credit(&store, &user.id, 5_000_000, "usdt-seed-guards");
    let _clock = override_now_for_test("2026-09-03T00:00:00Z").unwrap();
    let quote = store
        .create_esk_exchange_quote(&quote_input(
            &user.id,
            EskExchangeDirection::UsdtToEsk,
            1_000_000,
            1_000_000,
            30,
        ))
        .unwrap();
    assert!(store
        .execute_esk_exchange(&EskExchangeExecutionInput {
            user_id: user.id.clone(),
            quote_id: quote.quote_id.clone(),
            idempotency_key: "stale-config".to_string(),
            config_revision: "b".repeat(64),
        })
        .unwrap_err()
        .to_string()
        .contains("配置已经更新"));
    drop(_clock);
    let _expired = override_now_for_test("2026-09-03T00:01:01Z").unwrap();
    assert!(store
        .execute_esk_exchange(&EskExchangeExecutionInput {
            user_id: user.id.clone(),
            quote_id: quote.quote_id,
            idempotency_key: "expired-execution".to_string(),
            config_revision: "a".repeat(64),
        })
        .unwrap_err()
        .to_string()
        .contains("过期"));
    drop(_expired);
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn concurrent_quotes_cannot_overspend_one_usdt_balance() {
    let (store, path) = temp_store("race");
    let user = create_user(&store, "race");
    credit(&store, &user.id, 10_000_000, "usdt-seed-race");
    let quotes = (0..2)
        .map(|_| {
            store
                .create_esk_exchange_quote(&quote_input(
                    &user.id,
                    EskExchangeDirection::UsdtToEsk,
                    8_000_000,
                    1_000_000,
                    30,
                ))
                .unwrap()
        })
        .collect::<Vec<_>>();
    drop(store);
    let barrier = Arc::new(Barrier::new(2));
    let handles = quotes
        .into_iter()
        .enumerate()
        .map(|(index, quote)| {
            let path = path.clone();
            let user_id = user.id.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let store = Store::open(&path).unwrap();
                barrier.wait();
                store.execute_esk_exchange(&EskExchangeExecutionInput {
                    user_id,
                    quote_id: quote.quote_id,
                    idempotency_key: format!("race-{index}"),
                    config_revision: "a".repeat(64),
                })
            })
        })
        .collect::<Vec<_>>();
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|value| value.is_ok()).count(), 1);
    assert_eq!(outcomes.iter().filter(|value| value.is_err()).count(), 1);
    let store = Store::open(&path).unwrap();
    assert_eq!(
        store
            .esk_exchange_account_ledger(&user.id)
            .unwrap()
            .usdt_units,
        2_000_000
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn api_requires_login_fails_closed_and_returns_safe_receipts() {
    let _env = ENV_LOCK.lock().unwrap();
    let _asset_mode = override_esk_asset_mode_for_test(EskAssetMode::Paper);
    let previous = [
        "ESK_PAPER_EXCHANGE_MODE",
        "ESK_PAPER_USDT_PER_ESK",
        "ESK_PAPER_EXCHANGE_FEE_BPS",
    ]
    .map(|key| (key, std::env::var(key).ok()));
    std::env::set_var("ESK_PAPER_EXCHANGE_MODE", "paper");
    std::env::set_var("ESK_PAPER_USDT_PER_ESK", "2.000000");
    std::env::set_var("ESK_PAPER_EXCHANGE_FEE_BPS", "30");

    let root = std::env::temp_dir().join(format!(
        "elon_esk_exchange_http_{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let store = Store::open(&root.join("exchange.db")).unwrap();
    let user = create_user(&store, "http");
    credit(&store, &user.id, 10_000_000, "usdt-http");
    let (token, _) = store
        .create_session(&user.id, Some("exchange test"), None)
        .unwrap();
    let state = Arc::new(test_app_state(store, &root));
    let router = super::routes().with_state(Arc::clone(&state));

    let unauthorized = call_json(
        &router,
        "/api/me/assets/esk/exchange-account",
        "GET",
        None,
        None,
    )
    .await;
    assert_eq!(unauthorized.0, StatusCode::UNAUTHORIZED);
    let account = call_json(
        &router,
        "/api/me/assets/esk/exchange-account",
        "GET",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(account.0, StatusCode::OK);
    assert_eq!(account.1["balances"]["usdt"]["available"], "10.000000");
    assert_eq!(account.1["funds_moved"], false);
    assert_eq!(account.1["on_chain_settlement"], false);

    let quote = call_json(
        &router,
        "/api/me/assets/esk/exchange-quotes",
        "POST",
        Some(&token),
        Some(json!({
            "direction": "usdt_to_esk", "input_amount": "4.000000"
        })),
    )
    .await;
    assert_eq!(quote.0, StatusCode::CREATED);
    assert_eq!(quote.1["fee_amount"], "0.006000");
    let execution = call_json(
        &router,
        "/api/me/assets/esk/exchanges",
        "POST",
        Some(&token),
        Some(json!({
            "quote_id": quote.1["quote_id"], "idempotency_key": "http-exchange-1",
            "confirmation": "CONFIRM PAPER ESK USDT EXCHANGE"
        })),
    )
    .await;
    assert_eq!(execution.0, StatusCode::CREATED);
    assert_eq!(execution.1["funds_moved"], false);
    assert_eq!(execution.1["quote"]["net_output_amount"], "1.994000");

    std::env::set_var("ESK_PAPER_EXCHANGE_MODE", "live");
    let disabled = call_json(
        &router,
        "/api/me/assets/esk/exchange-quotes",
        "POST",
        Some(&token),
        Some(json!({
            "direction": "usdt_to_esk", "input_amount": "1.000000"
        })),
    )
    .await;
    assert_eq!(disabled.0, StatusCode::SERVICE_UNAVAILABLE);

    for (key, value) in previous {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    drop(router);
    drop(state);
    let _ = std::fs::remove_dir_all(root);
}

async fn call_json(
    router: &axum::Router,
    path: &str,
    method: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(path).method(method);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    let response = router
        .clone()
        .oneshot(
            request
                .body(
                    body.map(|value| Body::from(value.to_string()))
                        .unwrap_or_else(Body::empty),
                )
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
