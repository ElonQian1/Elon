use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{open_commerce_developer_production_test_support::test_app_state, store::Store};

use super::{
    model::{
        EskAllocationBatchMode, PaperAllocationBatchBody, PaperAllocationBatchEntryBody,
        PAPER_ALLOCATION_BATCH_CONFIRMATION,
    },
    prepare_paper_allocation_batch, EskAllocationInput,
};

static ESK_MODE_LOCK: Mutex<()> = Mutex::new(());

fn temp_store() -> (Store, PathBuf) {
    let path = std::env::temp_dir().join(format!("elon_esk_batch_{}.db", Uuid::new_v4().simple()));
    (
        Store::open(&path).expect("ESK batch test store should open"),
        path,
    )
}

fn create_user(store: &Store, label: &str) -> crate::store::PublicUser {
    store
        .create_user(
            &format!("esk-batch-{label}-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            Some(label),
            None,
        )
        .expect("ESK batch test user should be created")
}

fn body(batch_id: &str, entries: Vec<(&str, &str, &str, &str)>) -> PaperAllocationBatchBody {
    PaperAllocationBatchBody {
        batch_id: batch_id.to_string(),
        mode: EskAllocationBatchMode::DryRun,
        expected_request_digest: None,
        confirmation: String::new(),
        entries: entries
            .into_iter()
            .map(
                |(user_id, amount, reference, idempotency_key)| PaperAllocationBatchEntryBody {
                    user_id: user_id.to_string(),
                    amount: amount.to_string(),
                    reference: reference.to_string(),
                    idempotency_key: idempotency_key.to_string(),
                },
            )
            .collect(),
    }
}

#[test]
fn preparation_is_deterministic_and_rejects_ambiguous_batches() {
    let first = prepare_paper_allocation_batch(body(
        "first-users-2026-09-02",
        vec![
            ("usr_1", "10.5", "order-a", "esk-order-a"),
            ("usr_2", "2", "order-b", "esk-order-b"),
        ],
    ))
    .unwrap();
    let replay = prepare_paper_allocation_batch(body(
        "first-users-2026-09-02",
        vec![
            ("usr_1", "10.500000", "order-a", "esk-order-a"),
            ("usr_2", "2.000000", "order-b", "esk-order-b"),
        ],
    ))
    .unwrap();
    assert_eq!(first.request_digest, replay.request_digest);
    assert_eq!(first.total_base_units, 12_500_000);

    let duplicate_reference = body(
        "duplicates",
        vec![
            ("usr_1", "1", "same-order", "key-a"),
            ("usr_1", "2", "same-order", "key-b"),
        ],
    );
    assert!(prepare_paper_allocation_batch(duplicate_reference)
        .unwrap_err()
        .to_string()
        .contains("重复登记引用"));

    let duplicate_key = body(
        "duplicates",
        vec![
            ("usr_1", "1", "order-a", "same-key"),
            ("usr_2", "2", "order-b", "same-key"),
        ],
    );
    assert!(prepare_paper_allocation_batch(duplicate_key)
        .unwrap_err()
        .to_string()
        .contains("重复幂等键"));

    assert!(prepare_paper_allocation_batch(body("empty", vec![])).is_err());
}

#[test]
fn dry_run_is_read_only_and_commit_is_atomic_and_replay_safe() {
    let (store, path) = temp_store();
    let alice = create_user(&store, "alice");
    let bob = create_user(&store, "bob");
    let input = prepare_paper_allocation_batch(body(
        "paid-users-001",
        vec![
            (&alice.id, "12.5", "paid-order-a", "batch-key-a"),
            (&bob.id, "3.25", "paid-order-b", "batch-key-b"),
        ],
    ))
    .unwrap();

    store.validate_esk_paper_allocation_batch(&input).unwrap();
    assert_eq!(store.esk_account_ledger(&alice.id).unwrap().revision, 0);
    assert_eq!(store.esk_account_ledger(&bob.id).unwrap().revision, 0);
    assert_eq!(table_count(&store, "esk_paper_allocation_batches"), 0);

    let first = store.create_esk_paper_allocation_batch(&input).unwrap();
    assert!(!first.replayed);
    assert_eq!(first.entries.len(), 2);
    assert_eq!(first.total_base_units, 15_750_000);
    assert_eq!(
        store
            .esk_account_ledger(&alice.id)
            .unwrap()
            .total_base_units,
        12_500_000
    );
    assert_eq!(
        store.esk_account_ledger(&bob.id).unwrap().total_base_units,
        3_250_000
    );
    assert_eq!(table_count(&store, "esk_paper_allocation_batches"), 1);

    let replay = store.create_esk_paper_allocation_batch(&input).unwrap();
    assert!(replay.replayed);
    assert_eq!(
        replay
            .entries
            .iter()
            .map(|entry| entry.entry_id.as_str())
            .collect::<Vec<_>>(),
        first
            .entries
            .iter()
            .map(|entry| entry.entry_id.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(store.esk_account_ledger(&alice.id).unwrap().revision, 1);

    let drift = prepare_paper_allocation_batch(body(
        "paid-users-001",
        vec![
            (&alice.id, "99", "paid-order-a", "batch-key-a"),
            (&bob.id, "3.25", "paid-order-b", "batch-key-b"),
        ],
    ))
    .unwrap();
    assert!(store
        .create_esk_paper_allocation_batch(&drift)
        .unwrap_err()
        .to_string()
        .contains("批次 ID"));
    assert_eq!(
        store
            .esk_account_ledger(&alice.id)
            .unwrap()
            .total_base_units,
        12_500_000
    );

    let conn = store.conn().unwrap();
    assert!(conn
        .execute(
            "UPDATE esk_paper_allocation_batches SET entry_count = 1 WHERE batch_id = ?1",
            rusqlite::params![input.batch_id],
        )
        .unwrap_err()
        .to_string()
        .contains("append-only"));
    assert!(conn
        .execute(
            "DELETE FROM esk_paper_allocation_batch_entries WHERE batch_id = ?1",
            rusqlite::params![input.batch_id],
        )
        .unwrap_err()
        .to_string()
        .contains("append-only"));
    drop(conn);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn missing_user_or_preused_entry_key_rolls_back_every_entry() {
    let (store, path) = temp_store();
    let alice = create_user(&store, "rollback-alice");
    let bob = create_user(&store, "rollback-bob");

    let missing_user = prepare_paper_allocation_batch(body(
        "rollback-missing",
        vec![
            (&alice.id, "1", "rollback-order-a", "rollback-key-a"),
            (
                "usr_missing",
                "2",
                "rollback-order-missing",
                "rollback-key-missing",
            ),
        ],
    ))
    .unwrap();
    assert!(store
        .create_esk_paper_allocation_batch(&missing_user)
        .unwrap_err()
        .to_string()
        .contains("不存在"));
    assert_eq!(store.esk_account_ledger(&alice.id).unwrap().revision, 0);
    assert_eq!(table_count(&store, "esk_paper_allocation_batches"), 0);

    store
        .create_esk_paper_allocation(&EskAllocationInput {
            user_id: alice.id.clone(),
            amount_base_units: 1_000_000,
            reference: "single-order".to_string(),
            idempotency_key: "already-used".to_string(),
        })
        .unwrap();
    let occupied = prepare_paper_allocation_batch(body(
        "rollback-occupied",
        vec![
            (&bob.id, "5", "new-order", "new-key"),
            (&alice.id, "1", "occupied-order", "already-used"),
        ],
    ))
    .unwrap();
    assert!(store
        .create_esk_paper_allocation_batch(&occupied)
        .unwrap_err()
        .to_string()
        .contains("已被使用"));
    assert_eq!(store.esk_account_ledger(&bob.id).unwrap().revision, 0);
    assert_eq!(table_count(&store, "esk_paper_allocation_batches"), 0);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn admin_batch_api_requires_auth_digest_confirmation_and_returns_v1_receipt() {
    let _mode_guard = ESK_MODE_LOCK.lock().unwrap();
    let previous = std::env::var("ESK_ASSET_MODE").ok();
    std::env::set_var("ESK_ASSET_MODE", "paper");

    let root =
        std::env::temp_dir().join(format!("elon_esk_batch_http_{}", Uuid::new_v4().simple()));
    std::fs::create_dir_all(&root).unwrap();
    let store = Store::open(&root.join("esk.db")).unwrap();
    let user = create_user(&store, "http");
    let state = Arc::new(test_app_state(store, &root));
    let router = super::routes().with_state(Arc::clone(&state));
    let request_entries = json!([{
        "user_id": user.id,
        "amount": "6.000001",
        "reference": "http-order",
        "idempotency_key": "http-order-key"
    }]);

    let unauthorized = call_batch(
        &router,
        None,
        json!({
            "batch_id": "http-batch",
            "mode": "dry_run",
            "entries": request_entries
        }),
    )
    .await;
    assert_eq!(unauthorized.0, StatusCode::UNAUTHORIZED);

    let (status, preview) = call_batch(
        &router,
        Some("test"),
        json!({
            "batch_id": "http-batch",
            "mode": "dry_run",
            "entries": request_entries
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        preview["schema"],
        "yilong.esk.paper_allocation_batch_receipt.v1"
    );
    assert_eq!(preview["status"], "validated");
    assert_eq!(preview["total"], "6.000001");
    assert_eq!(preview["simulated"], true);
    assert_eq!(preview["funds_moved"], false);
    let digest = preview["request_digest"].as_str().unwrap();

    let (bad_status, _) = call_batch(
        &router,
        Some("test"),
        json!({
            "batch_id": "http-batch",
            "mode": "commit",
            "expected_request_digest": "0".repeat(64),
            "confirmation": PAPER_ALLOCATION_BATCH_CONFIRMATION,
            "entries": request_entries
        }),
    )
    .await;
    assert_eq!(bad_status, StatusCode::BAD_REQUEST);

    let (commit_status, committed) = call_batch(
        &router,
        Some("test"),
        json!({
            "batch_id": "http-batch",
            "mode": "commit",
            "expected_request_digest": digest,
            "confirmation": PAPER_ALLOCATION_BATCH_CONFIRMATION,
            "entries": request_entries
        }),
    )
    .await;
    assert_eq!(commit_status, StatusCode::CREATED);
    assert_eq!(committed["status"], "committed");
    assert_eq!(committed["replayed"], false);
    assert!(committed["entries"][0]["entry_id"].as_str().is_some());
    assert_eq!(
        state
            .store
            .esk_account_ledger(&user.id)
            .unwrap()
            .total_base_units,
        6_000_001
    );

    drop(router);
    drop(state);
    match previous {
        Some(value) => std::env::set_var("ESK_ASSET_MODE", value),
        None => std::env::remove_var("ESK_ASSET_MODE"),
    }
    let _ = std::fs::remove_dir_all(root);
}

fn table_count(store: &Store, table: &str) -> i64 {
    assert!(matches!(
        table,
        "esk_paper_allocation_batches" | "esk_paper_allocation_batch_entries"
    ));
    store
        .conn()
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

async fn call_batch(
    router: &axum::Router,
    bearer: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/admin/assets/esk/paper-allocation-batches")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
