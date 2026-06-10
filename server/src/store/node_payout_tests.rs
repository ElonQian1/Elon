use rusqlite::params;

use super::{now, CreateNodePayout, Store};

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-node-payout-test-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

fn add_balance(store: &Store, user_id: &str, available_fen: i64) {
    let conn = store.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO node_balances
           (user_id, credits, available_fen, frozen_fen, paid_fen, updated_at)
         VALUES (?1, ?2, ?3, 0, 0, ?4)
         ON CONFLICT(user_id) DO UPDATE SET
           credits = ?2,
           available_fen = ?3,
           frozen_fen = 0,
           paid_fen = 0,
           updated_at = ?4",
        params![user_id, available_fen as f64 / 100.0, available_fen, now()],
    )
    .unwrap();
}

fn balance_fields(store: &Store, user_id: &str) -> (i64, i64, i64) {
    let conn = store.conn.lock().unwrap();
    conn.query_row(
        "SELECT available_fen, frozen_fen, paid_fen
         FROM node_balances
         WHERE user_id = ?1",
        params![user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .unwrap()
}

#[test]
fn create_payout_freezes_available_balance() {
    let (store, path) = temp_store();
    let user = store
        .create_user("node-payout-freeze@example.com", "secret1", None, None)
        .unwrap();
    add_balance(&store, &user.id, 1250);

    let payout = store
        .create_node_payout_request(CreateNodePayout {
            provider_user_id: &user.id,
            amount_fen: 500,
            payout_method: "wechat",
            payout_account: "wx-001",
            contact: Some("owner"),
        })
        .unwrap();

    assert_eq!(payout.status, "pending");
    assert_eq!(payout.amount_fen, 500);
    assert_eq!(store.get_node_balance_fen(&user.id).unwrap(), 750);
    assert_eq!(store.get_node_balance(&user.id).unwrap(), 7.50);
    assert_eq!(
        store.get_pending_node_payout_total_fen(&user.id).unwrap(),
        500
    );
    assert_eq!(store.get_pending_node_payout_total(&user.id).unwrap(), 5.0);
    assert_eq!(balance_fields(&store, &user.id), (750, 500, 0));
    let _ = std::fs::remove_file(path);
}

#[test]
fn reject_refunds_once_and_paid_moves_frozen_to_paid() {
    let (store, path) = temp_store();
    let user = store
        .create_user("node-payout-resolve@example.com", "secret1", None, None)
        .unwrap();
    add_balance(&store, &user.id, 1000);

    let rejected = store
        .create_node_payout_request(CreateNodePayout {
            provider_user_id: &user.id,
            amount_fen: 300,
            payout_method: "bank",
            payout_account: "bank-card",
            contact: None,
        })
        .unwrap();
    assert_eq!(balance_fields(&store, &user.id), (700, 300, 0));
    store
        .admin_reject_node_payout(&rejected.id, "admin", Some("资料不完整"))
        .unwrap();
    assert_eq!(balance_fields(&store, &user.id), (1000, 0, 0));
    store
        .admin_reject_node_payout(&rejected.id, "admin", Some("资料不完整"))
        .unwrap();
    assert_eq!(balance_fields(&store, &user.id), (1000, 0, 0));

    let paid = store
        .create_node_payout_request(CreateNodePayout {
            provider_user_id: &user.id,
            amount_fen: 400,
            payout_method: "usdt",
            payout_account: "wallet",
            contact: None,
        })
        .unwrap();
    assert_eq!(balance_fields(&store, &user.id), (600, 400, 0));
    store
        .admin_mark_node_payout_paid(&paid.id, "admin", Some("txid:1"))
        .unwrap();
    assert_eq!(store.get_node_balance_fen(&user.id).unwrap(), 600);
    assert_eq!(store.get_node_balance(&user.id).unwrap(), 6.0);
    assert_eq!(balance_fields(&store, &user.id), (600, 0, 400));
    let _ = std::fs::remove_file(path);
}

#[test]
fn insufficient_balance_is_rejected() {
    let (store, path) = temp_store();
    let user = store
        .create_user("node-payout-low@example.com", "secret1", None, None)
        .unwrap();
    add_balance(&store, &user.id, 100);

    let err = store
        .create_node_payout_request(CreateNodePayout {
            provider_user_id: &user.id,
            amount_fen: 200,
            payout_method: "wechat",
            payout_account: "wx-001",
            contact: None,
        })
        .unwrap_err();
    assert!(err.to_string().contains("余额不足"));
    assert_eq!(store.get_node_balance_fen(&user.id).unwrap(), 100);
    assert_eq!(balance_fields(&store, &user.id), (100, 0, 0));
    let _ = std::fs::remove_file(path);
}

#[test]
fn provider_can_cancel_pending_payout() {
    let (store, path) = temp_store();
    let user = store
        .create_user("node-payout-cancel@example.com", "secret1", None, None)
        .unwrap();
    add_balance(&store, &user.id, 800);
    let payout = store
        .create_node_payout_request(CreateNodePayout {
            provider_user_id: &user.id,
            amount_fen: 250,
            payout_method: "alipay",
            payout_account: "ali-001",
            contact: None,
        })
        .unwrap();
    assert_eq!(balance_fields(&store, &user.id), (550, 250, 0));

    let cancelled = store
        .cancel_node_payout_request(&user.id, &payout.id)
        .unwrap();
    assert_eq!(cancelled.status, "cancelled");
    assert_eq!(store.get_node_balance_fen(&user.id).unwrap(), 800);
    assert_eq!(balance_fields(&store, &user.id), (800, 0, 0));
    let _ = std::fs::remove_file(path);
}
