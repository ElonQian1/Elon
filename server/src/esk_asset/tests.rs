use std::{
    path::PathBuf,
    sync::{Arc, Barrier},
};

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

use crate::{open_commerce_developer_production_test_support::test_app_state, store::Store};

use super::{
    format_esk_amount, model::EskAssetMode, parse_esk_amount, EskAllocationInput, EskSellbackInput,
};

fn temp_store() -> (Store, PathBuf) {
    let path = std::env::temp_dir().join(format!("elon_esk_asset_{}.db", Uuid::new_v4().simple()));
    (
        Store::open(&path).expect("ESK test store should open"),
        path,
    )
}

fn create_user(store: &Store, label: &str) -> crate::store::PublicUser {
    store
        .create_user(
            &format!("esk-{label}-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            Some(label),
            None,
        )
        .expect("ESK test user should be created")
}

fn allocation(user_id: &str, amount_base_units: i64, key: &str) -> EskAllocationInput {
    EskAllocationInput {
        user_id: user_id.to_string(),
        amount_base_units,
        reference: "paid-order-paper-proof".to_string(),
        idempotency_key: key.to_string(),
    }
}

fn sellback(user_id: &str, amount_base_units: i64, key: &str) -> EskSellbackInput {
    EskSellbackInput {
        user_id: user_id.to_string(),
        amount_base_units,
        idempotency_key: key.to_string(),
    }
}

#[test]
fn exact_amount_codec_rejects_float_ambiguity_and_overflow() {
    assert_eq!(parse_esk_amount("1").unwrap(), 1_000_000);
    assert_eq!(parse_esk_amount("12.000001").unwrap(), 12_000_001);
    assert_eq!(parse_esk_amount("0.1").unwrap(), 100_000);
    assert_eq!(format_esk_amount(12_000_001), "12.000001");
    assert_eq!(format_esk_amount(-1), "-0.000001");
    for invalid in ["", "0", "-1", "+1", ".1", "1.", "1.0000001", "1e6"] {
        assert!(parse_esk_amount(invalid).is_err(), "accepted {invalid:?}");
    }
    assert!(parse_esk_amount("9223372036854775807").is_err());
}

#[test]
fn mode_is_disabled_by_default_and_unknown_values_fail_closed() {
    assert_eq!(EskAssetMode::from_value(None), EskAssetMode::Disabled);
    assert_eq!(EskAssetMode::from_value(Some("")), EskAssetMode::Disabled);
    assert_eq!(EskAssetMode::from_value(Some("paper")), EskAssetMode::Paper);
    assert_eq!(
        EskAssetMode::from_value(Some("live")),
        EskAssetMode::Invalid
    );
    assert!(!EskAssetMode::Invalid.writes_enabled());
}

#[test]
fn allocation_is_append_only_idempotent_and_drift_safe() {
    let (store, path) = temp_store();
    let user = create_user(&store, "allocation");
    let input = allocation(&user.id, 12_500_000, "paid-order-1");

    let first = store.create_esk_paper_allocation(&input).unwrap();
    assert!(!first.replayed);
    let replay = store.create_esk_paper_allocation(&input).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.entry_id, first.entry_id);

    let drift = allocation(&user.id, 13_000_000, "paid-order-1");
    assert!(store
        .create_esk_paper_allocation(&drift)
        .unwrap_err()
        .to_string()
        .contains("幂等键"));
    let ledger = store.esk_account_ledger(&user.id).unwrap();
    assert_eq!(ledger.total_base_units, 12_500_000);
    assert_eq!(ledger.revision, 1);

    let conn = store.conn().unwrap();
    assert!(conn
        .execute(
            "UPDATE esk_asset_ledger_entries SET amount_base_units = 1 WHERE entry_id = ?1",
            rusqlite::params![first.entry_id],
        )
        .unwrap_err()
        .to_string()
        .contains("append-only"));
    drop(conn);
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn sellback_reserves_available_balance_and_cancel_releases_it() {
    let (store, path) = temp_store();
    let user = create_user(&store, "sellback");
    store
        .create_esk_paper_allocation(&allocation(&user.id, 10_000_000, "paid-order-2"))
        .unwrap();

    let input = sellback(&user.id, 4_250_000, "sellback-1");
    let submitted = store.create_esk_sellback_request(&input).unwrap();
    assert_eq!(submitted.status, "submitted");
    assert_eq!(
        store
            .esk_account_ledger(&user.id)
            .unwrap()
            .reserved_base_units,
        4_250_000
    );
    assert!(store
        .create_esk_sellback_request(&sellback(&user.id, 6_000_000, "sellback-too-much"))
        .unwrap_err()
        .to_string()
        .contains("超过"));

    let canceled = store
        .cancel_esk_sellback_request(&user.id, &submitted.request_id)
        .unwrap();
    assert_eq!(canceled.status, "canceled");
    assert_eq!(canceled.revision, 2);
    assert!(!canceled.replayed);
    let replay = store
        .cancel_esk_sellback_request(&user.id, &submitted.request_id)
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(
        store
            .esk_account_ledger(&user.id)
            .unwrap()
            .reserved_base_units,
        0
    );
    assert_eq!(
        store
            .list_esk_sellback_requests(&user.id, 20)
            .unwrap()
            .len(),
        1
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn concurrent_sellback_cannot_over_reserve_one_balance() {
    let (store, path) = temp_store();
    let user = create_user(&store, "concurrent");
    store
        .create_esk_paper_allocation(&allocation(&user.id, 10_000_000, "paid-order-3"))
        .unwrap();
    drop(store);

    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for index in 0..2 {
        let path = path.clone();
        let user_id = user.id.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            let store = Store::open(&path).unwrap();
            barrier.wait();
            store.create_esk_sellback_request(&sellback(
                &user_id,
                7_000_000,
                &format!("concurrent-{index}"),
            ))
        }));
    }
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(outcomes.iter().filter(|result| result.is_err()).count(), 1);

    let store = Store::open(&path).unwrap();
    assert_eq!(
        store
            .esk_account_ledger(&user.id)
            .unwrap()
            .reserved_base_units,
        7_000_000
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn account_http_requires_login_and_only_projects_authenticated_users_balance() {
    let root = std::env::temp_dir().join(format!("elon_esk_http_{}", Uuid::new_v4().simple()));
    std::fs::create_dir_all(&root).unwrap();
    let store = Store::open(&root.join("esk.db")).unwrap();
    let owner = create_user(&store, "owner");
    let outsider = create_user(&store, "outsider");
    store
        .create_esk_paper_allocation(&allocation(&owner.id, 8_000_000, "http-owner"))
        .unwrap();
    store
        .create_esk_paper_allocation(&allocation(&outsider.id, 3_000_000, "http-outsider"))
        .unwrap();
    let (owner_token, _) = store
        .create_session(&owner.id, Some("ESK test"), None)
        .unwrap();
    let (outsider_token, _) = store
        .create_session(&outsider.id, Some("ESK test"), None)
        .unwrap();
    let state = Arc::new(test_app_state(store, &root));
    let router = super::routes().with_state(Arc::clone(&state));

    assert_eq!(get_account(&router, None).await.0, StatusCode::UNAUTHORIZED);
    let (status, owner_body) = get_account(&router, Some(&owner_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(owner_body["asset"]["symbol"], "ESK");
    assert_eq!(owner_body["balance"]["total"], "8.000000");
    assert_eq!(owner_body["asset"]["chain_status"], "not_deployed");
    assert_eq!(owner_body["simulated"], true);
    assert_eq!(owner_body["funds_moved"], false);

    let (_, outsider_body) = get_account(&router, Some(&outsider_token)).await;
    assert_eq!(outsider_body["balance"]["total"], "3.000000");
    drop(router);
    drop(state);
    let _ = std::fs::remove_dir_all(root);
}

async fn get_account(router: &axum::Router, bearer: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder().uri("/api/me/assets/esk");
    if let Some(token) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
