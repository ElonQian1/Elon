use std::{
    path::PathBuf,
    sync::{Arc, Barrier},
};

use uuid::Uuid;

use crate::store::Store;

use super::{EskAllocationInput, EskQuantAllocationInput, ESK_QUANT_RISK_DISCLOSURE_REVISION};

fn temp_store() -> (Store, PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon_esk_quant_allocation_{}.db",
        Uuid::new_v4().simple()
    ));
    (
        Store::open(&path).expect("ESK quant allocation store should open"),
        path,
    )
}

fn create_user(store: &Store, label: &str) -> crate::store::PublicUser {
    store
        .create_user(
            &format!("esk-quant-{label}-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            Some(label),
            None,
        )
        .expect("ESK quant test user should be created")
}

fn credit(store: &Store, user_id: &str, amount_base_units: i64, key: &str) {
    store
        .create_esk_paper_allocation(&EskAllocationInput {
            user_id: user_id.to_owned(),
            amount_base_units,
            reference: "paper-test-credit".to_owned(),
            idempotency_key: key.to_owned(),
        })
        .expect("paper ESK should be credited");
}

fn quant_request(user_id: &str, amount_base_units: i64, key: &str) -> EskQuantAllocationInput {
    EskQuantAllocationInput {
        user_id: user_id.to_owned(),
        amount_base_units,
        idempotency_key: key.to_owned(),
        risk_disclosure_revision: ESK_QUANT_RISK_DISCLOSURE_REVISION.to_owned(),
    }
}

#[test]
fn quant_request_reserves_available_balance_and_cancel_releases_it() {
    let (store, path) = temp_store();
    let user = create_user(&store, "lifecycle");
    credit(&store, &user.id, 12_000_000, "credit-lifecycle");

    let input = quant_request(&user.id, 4_500_000, "quant-lifecycle");
    let created = store.create_esk_quant_allocation_request(&input).unwrap();
    assert_eq!(created.status, "submitted");
    assert!(!created.replayed);

    let ledger = store.esk_account_ledger(&user.id).unwrap();
    assert_eq!(ledger.total_base_units, 12_000_000);
    assert_eq!(ledger.sellback_reserved_base_units, 0);
    assert_eq!(ledger.quant_reserved_base_units, 4_500_000);
    assert_eq!(ledger.reserved_base_units, 4_500_000);

    let replay = store.create_esk_quant_allocation_request(&input).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.request_id, created.request_id);

    let canceled = store
        .cancel_esk_quant_allocation_request(&user.id, &created.request_id)
        .unwrap();
    assert_eq!(canceled.status, "canceled");
    assert_eq!(canceled.revision, 2);
    assert!(!canceled.replayed);
    assert_eq!(
        store
            .esk_account_ledger(&user.id)
            .unwrap()
            .quant_reserved_base_units,
        0
    );

    let repeated_cancel = store
        .cancel_esk_quant_allocation_request(&user.id, &created.request_id)
        .unwrap();
    assert!(repeated_cancel.replayed);
    let listed = store
        .list_esk_quant_allocation_requests(&user.id, 20)
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].request_id, repeated_cancel.request_id);
    assert_eq!(listed[0].status, "canceled");

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn quant_request_rejects_drift_overage_and_cross_user_cancel() {
    let (store, path) = temp_store();
    let owner = create_user(&store, "owner");
    let outsider = create_user(&store, "outsider");
    credit(&store, &owner.id, 5_000_000, "credit-drift");

    let input = quant_request(&owner.id, 3_000_000, "quant-drift");
    let created = store.create_esk_quant_allocation_request(&input).unwrap();

    let amount_drift = quant_request(&owner.id, 2_000_000, "quant-drift");
    assert!(store
        .create_esk_quant_allocation_request(&amount_drift)
        .unwrap_err()
        .to_string()
        .contains("幂等键"));
    let mut disclosure_drift = input.clone();
    disclosure_drift.risk_disclosure_revision = "other-revision".to_owned();
    assert!(store
        .create_esk_quant_allocation_request(&disclosure_drift)
        .unwrap_err()
        .to_string()
        .contains("幂等键"));
    assert!(store
        .create_esk_quant_allocation_request(&quant_request(&owner.id, 3_000_000, "quant-overage"))
        .unwrap_err()
        .to_string()
        .contains("超过"));
    assert!(store
        .cancel_esk_quant_allocation_request(&outsider.id, &created.request_id)
        .unwrap_err()
        .to_string()
        .contains("不存在"));

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn sellback_and_quant_request_cannot_concurrently_over_reserve() {
    let (store, path) = temp_store();
    let user = create_user(&store, "concurrent");
    credit(&store, &user.id, 10_000_000, "credit-concurrent");
    drop(store);

    let barrier = Arc::new(Barrier::new(2));
    let quant_path = path.clone();
    let quant_user = user.id.clone();
    let quant_barrier = Arc::clone(&barrier);
    let quant = std::thread::spawn(move || {
        let store = Store::open(&quant_path).unwrap();
        quant_barrier.wait();
        store.create_esk_quant_allocation_request(&quant_request(
            &quant_user,
            7_000_000,
            "quant-concurrent",
        ))
    });

    let sellback_path = path.clone();
    let sellback_user = user.id.clone();
    let sellback_barrier = Arc::clone(&barrier);
    let sellback = std::thread::spawn(move || {
        let store = Store::open(&sellback_path).unwrap();
        sellback_barrier.wait();
        store.create_esk_sellback_request(&super::EskSellbackInput {
            user_id: sellback_user,
            amount_base_units: 7_000_000,
            idempotency_key: "sellback-concurrent".to_owned(),
        })
    });

    let successes = [
        quant.join().unwrap().is_ok(),
        sellback.join().unwrap().is_ok(),
    ]
    .into_iter()
    .filter(|success| *success)
    .count();
    assert_eq!(successes, 1);

    let store = Store::open(&path).unwrap();
    let ledger = store.esk_account_ledger(&user.id).unwrap();
    assert_eq!(ledger.reserved_base_units, 7_000_000);
    assert_eq!(
        ledger.sellback_reserved_base_units + ledger.quant_reserved_base_units,
        7_000_000
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn quant_request_tables_are_append_only_and_survive_restart() {
    let (store, path) = temp_store();
    let user = create_user(&store, "append-only");
    credit(&store, &user.id, 9_000_000, "credit-append-only");
    let created = store
        .create_esk_quant_allocation_request(&quant_request(
            &user.id,
            2_000_000,
            "quant-append-only",
        ))
        .unwrap();

    let conn = store.conn().unwrap();
    assert!(conn
        .execute(
            "UPDATE esk_quant_allocation_requests SET amount_base_units = 1 WHERE request_id = ?1",
            rusqlite::params![created.request_id],
        )
        .unwrap_err()
        .to_string()
        .contains("append-only"));
    assert!(conn
        .execute(
            "DELETE FROM esk_quant_allocation_request_events WHERE request_id = ?1",
            rusqlite::params![created.request_id],
        )
        .unwrap_err()
        .to_string()
        .contains("append-only"));
    drop(conn);
    drop(store);

    let reopened = Store::open(&path).unwrap();
    let requests = reopened
        .list_esk_quant_allocation_requests(&user.id, 20)
        .unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].request_id, created.request_id);
    assert_eq!(
        reopened
            .esk_account_ledger(&user.id)
            .unwrap()
            .quant_reserved_base_units,
        2_000_000
    );
    drop(reopened);
    let _ = std::fs::remove_file(path);
}
