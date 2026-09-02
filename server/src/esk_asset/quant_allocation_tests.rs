use std::{
    path::PathBuf,
    sync::{Arc, Barrier},
};

use uuid::Uuid;

use crate::store::Store;

use super::{
    EskAllocationInput, EskQuantAllocationInput, EskQuantAllocationReceiptInput,
    ESK_QUANT_RISK_DISCLOSURE_REVISION,
};

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

fn quant_receipt(
    user_id: &str,
    request_id: &str,
    amount_base_units: i64,
    event: &str,
) -> EskQuantAllocationReceiptInput {
    let released = event == "released";
    EskQuantAllocationReceiptInput {
        user_id: user_id.to_owned(),
        participant_ref: "yp1_0123456789abcdef0123456789abcdef01234567".to_owned(),
        request_id: request_id.to_owned(),
        amount_base_units,
        risk_disclosure_revision: ESK_QUANT_RISK_DISCLOSURE_REVISION.to_owned(),
        event: event.to_owned(),
        binding_id: "eskbind_0123456789abcdef0123456789abcdef".to_owned(),
        receipt_id: if released {
            "eskrcpt_89abcdef0123456789abcdef01234567"
        } else {
            "eskrcpt_0123456789abcdef0123456789abcdef"
        }
        .to_owned(),
        receipt_digest: format!("sha256:{}", if released { "2" } else { "1" }.repeat(64)),
        receipt_key_id: "quant-receipt-key-1".to_owned(),
        previous_receipt_digest: released.then(|| format!("sha256:{}", "1".repeat(64))),
        quant_binding_revision: if released { 2 } else { 1 },
        occurred_at_unix: if released {
            1_788_192_020
        } else {
            1_788_192_010
        },
    }
}

#[test]
fn signed_binding_receipts_keep_accepted_reserved_and_release_it_idempotently() {
    let (store, path) = temp_store();
    let user = create_user(&store, "binding-lifecycle");
    credit(&store, &user.id, 10_000_000, "credit-binding-lifecycle");
    let request = store
        .create_esk_quant_allocation_request(&quant_request(
            &user.id,
            4_000_000,
            "binding-lifecycle",
        ))
        .unwrap();
    let accepted_input = quant_receipt(&user.id, &request.request_id, 4_000_000, "accepted");
    let accepted = store
        .apply_esk_quant_allocation_receipt(&accepted_input)
        .unwrap();
    assert_eq!(accepted.status, "accepted");
    assert_eq!(accepted.revision, 2);
    assert_eq!(
        accepted.binding_id.as_deref(),
        Some("eskbind_0123456789abcdef0123456789abcdef")
    );
    assert_eq!(
        store
            .esk_account_ledger(&user.id)
            .unwrap()
            .quant_reserved_base_units,
        4_000_000
    );
    assert!(
        store
            .apply_esk_quant_allocation_receipt(&accepted_input)
            .unwrap()
            .replayed
    );

    let other_request = store
        .create_esk_quant_allocation_request(&quant_request(
            &user.id,
            1_000_000,
            "binding-id-reuse",
        ))
        .unwrap();
    let mut reused_binding =
        quant_receipt(&user.id, &other_request.request_id, 1_000_000, "accepted");
    reused_binding.receipt_id = "eskrcpt_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
    reused_binding.receipt_digest = format!("sha256:{}", "a".repeat(64));
    assert!(store
        .apply_esk_quant_allocation_receipt(&reused_binding)
        .unwrap_err()
        .to_string()
        .contains("binding ID already"));
    store
        .cancel_esk_quant_allocation_request(&user.id, &other_request.request_id)
        .unwrap();

    let released_input = quant_receipt(&user.id, &request.request_id, 4_000_000, "released");
    let released = store
        .apply_esk_quant_allocation_receipt(&released_input)
        .unwrap();
    assert_eq!(released.status, "released");
    assert_eq!(released.revision, 3);
    assert_eq!(
        store
            .esk_account_ledger(&user.id)
            .unwrap()
            .quant_reserved_base_units,
        0
    );
    assert!(
        store
            .apply_esk_quant_allocation_receipt(&released_input)
            .unwrap()
            .replayed
    );

    let conn = store.conn().unwrap();
    assert!(conn
        .execute(
            "DELETE FROM esk_quant_allocation_binding_events WHERE request_id = ?1",
            rusqlite::params![request.request_id],
        )
        .unwrap_err()
        .to_string()
        .contains("append-only"));
    drop(conn);
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(
        reopened
            .esk_quant_allocation_request(&user.id, &request.request_id)
            .unwrap()
            .unwrap()
            .status,
        "released"
    );
    drop(reopened);
    let _ = std::fs::remove_file(path);
}

#[test]
fn cancel_and_signed_acceptance_are_serialized_to_one_winner() {
    let (store, path) = temp_store();
    let user = create_user(&store, "binding-race");
    credit(&store, &user.id, 8_000_000, "credit-binding-race");
    let request = store
        .create_esk_quant_allocation_request(&quant_request(&user.id, 3_000_000, "binding-race"))
        .unwrap();
    drop(store);

    let barrier = Arc::new(Barrier::new(2));
    let cancel_path = path.clone();
    let cancel_user = user.id.clone();
    let cancel_request = request.request_id.clone();
    let cancel_barrier = Arc::clone(&barrier);
    let cancel = std::thread::spawn(move || {
        let store = Store::open(&cancel_path).unwrap();
        cancel_barrier.wait();
        store.cancel_esk_quant_allocation_request(&cancel_user, &cancel_request)
    });
    let accept_path = path.clone();
    let accept_user = user.id.clone();
    let accept_request = request.request_id.clone();
    let accept_barrier = Arc::clone(&barrier);
    let accept = std::thread::spawn(move || {
        let store = Store::open(&accept_path).unwrap();
        let receipt = quant_receipt(&accept_user, &accept_request, 3_000_000, "accepted");
        accept_barrier.wait();
        store.apply_esk_quant_allocation_receipt(&receipt)
    });
    let results = [cancel.join().unwrap(), accept.join().unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let store = Store::open(&path).unwrap();
    let final_state = store
        .esk_quant_allocation_request(&user.id, &request.request_id)
        .unwrap()
        .unwrap();
    assert!(matches!(
        final_state.status.as_str(),
        "canceled" | "accepted"
    ));
    drop(store);
    let _ = std::fs::remove_file(path);
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
        .contains("风险披露"));
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
