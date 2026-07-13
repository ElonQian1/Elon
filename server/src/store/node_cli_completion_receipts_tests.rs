use super::*;

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-node-completion-receipts-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

fn input<'a>(
    event_id: &'a str,
    req_id: &'a str,
    compute_call_id: &'a str,
    payload_json: &'a str,
    payload_sha256: &'a str,
) -> NodeCliCompletionReceiptInput<'a> {
    NodeCliCompletionReceiptInput {
        event_id,
        req_id,
        compute_call_id,
        node_id: "node-a",
        user_id: "usr-a",
        payload_json,
        payload_sha256,
    }
}

fn insert_pending_receipt(store: &Store, event_id: &str) {
    let req_id = format!("req-{event_id}");
    let compute_call_id = format!("pc_agent_cli:{req_id}");
    let payload_json = format!(r#"{{"event_id":"{event_id}","req_id":"{req_id}"}}"#);
    let hash = "d".repeat(64);
    store
        .ingest_node_cli_completion_receipt(input(
            event_id,
            &req_id,
            &compute_call_id,
            &payload_json,
            &hash,
        ))
        .expect("receipt should be inserted");
}

#[test]
fn ingest_is_idempotent_and_rejects_hash_or_request_rebinding() {
    let (store, path) = temp_store();
    let hash_a = "a".repeat(64);
    let hash_b = "b".repeat(64);

    let inserted = store
        .ingest_node_cli_completion_receipt(input(
            "evt-a",
            "req-a",
            "pc_agent_cli:req-a",
            r#"{"event_id":"evt-a","req_id":"req-a"}"#,
            &hash_a,
        ))
        .unwrap();
    assert!(matches!(
        inserted,
        NodeCliCompletionIngestOutcome::Inserted(_)
    ));

    let duplicate = store
        .ingest_node_cli_completion_receipt(input(
            "evt-a",
            "req-a",
            "pc_agent_cli:req-a",
            r#"{"event_id":"evt-a","req_id":"req-a"}"#,
            &hash_a,
        ))
        .unwrap();
    assert!(duplicate.accepted());
    assert!(duplicate.deduplicated());

    let hash_conflict = store
        .ingest_node_cli_completion_receipt(input(
            "evt-a",
            "req-a",
            "pc_agent_cli:req-a",
            r#"{"event_id":"evt-a","req_id":"req-a","changed":true}"#,
            &hash_b,
        ))
        .unwrap();
    assert!(matches!(
        hash_conflict,
        NodeCliCompletionIngestOutcome::Conflict { ref reason, .. }
            if reason == "event_payload_hash_mismatch"
    ));

    let request_conflict = store
        .ingest_node_cli_completion_receipt(input(
            "evt-b",
            "req-a",
            "pc_agent_cli:req-b",
            r#"{"event_id":"evt-b","req_id":"req-a"}"#,
            &hash_b,
        ))
        .unwrap();
    assert!(matches!(
        request_conflict,
        NodeCliCompletionIngestOutcome::Conflict { ref reason, .. }
            if reason == "request_already_bound_to_other_event"
    ));

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn receipt_state_transitions_are_durable_and_terminal() {
    let (store, path) = temp_store();
    let hash = "c".repeat(64);
    store
        .ingest_node_cli_completion_receipt(input(
            "evt-state",
            "req-state",
            "pc_agent_cli:req-state",
            r#"{"event_id":"evt-state","req_id":"req-state"}"#,
            &hash,
        ))
        .unwrap();

    let retry = store
        .mark_node_cli_completion_retry("evt-state", "temporary database failure")
        .unwrap()
        .unwrap();
    assert_eq!(retry.status, "retry");
    assert_eq!(retry.attempt_count, 1);
    assert!(retry.last_attempt_at.is_some());
    assert_eq!(
        store
            .list_pending_node_cli_completion_receipts(10)
            .unwrap()
            .len(),
        1
    );

    let applied = store
        .mark_node_cli_completion_applied(
            "evt-state",
            Some("tok-state"),
            Some("bev-state"),
            Some("ntx-state"),
        )
        .unwrap()
        .unwrap();
    assert_eq!(applied.status, "applied");
    assert_eq!(applied.attempt_count, 2);
    assert_eq!(applied.token_usage_event_id.as_deref(), Some("tok-state"));
    assert_eq!(applied.billing_event_id.as_deref(), Some("bev-state"));
    assert_eq!(applied.node_transaction_id.as_deref(), Some("ntx-state"));
    assert!(applied.applied_at.is_some());
    assert!(store
        .list_pending_node_cli_completion_receipts(10)
        .unwrap()
        .is_empty());

    let unchanged = store
        .mark_node_cli_completion_rejected("evt-state", "must not downgrade applied")
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.status, "applied");
    assert_eq!(unchanged.attempt_count, 2);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn concurrent_processing_claim_succeeds_exactly_once() {
    let (store, path) = temp_store();
    insert_pending_receipt(&store, "evt-concurrent-claim");
    let store = std::sync::Arc::new(store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let handles = ["claim-a", "claim-b"].map(|claim_id| {
        let store = store.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            store
                .claim_node_cli_completion_receipt("evt-concurrent-claim", claim_id)
                .expect("claim should execute")
        })
    });

    barrier.wait();
    let outcomes = handles.map(|handle| handle.join().expect("claim thread should finish"));
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_some()).count(),
        1
    );
    let claimed = outcomes
        .into_iter()
        .flatten()
        .next()
        .expect("exactly one caller should own the claim");
    assert_eq!(claimed.status, "processing");
    assert_eq!(claimed.attempt_count, 1);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn wrong_processing_claim_owner_cannot_finalize() {
    let (store, path) = temp_store();
    insert_pending_receipt(&store, "evt-wrong-owner");
    store
        .claim_node_cli_completion_receipt("evt-wrong-owner", "claim-owner")
        .unwrap()
        .expect("first claim should succeed");

    assert!(!store
        .finish_node_cli_completion_claim_rejected(
            "evt-wrong-owner",
            "claim-attacker",
            "must not publish",
        )
        .unwrap());
    let receipt = store
        .get_node_cli_completion_receipt("evt-wrong-owner")
        .unwrap()
        .unwrap();
    assert_eq!(receipt.status, "processing");
    assert_eq!(receipt.reason.as_deref(), Some("claim-owner"));

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn right_processing_claim_owner_can_finalize() {
    let (store, path) = temp_store();
    insert_pending_receipt(&store, "evt-right-owner");
    store
        .claim_node_cli_completion_receipt("evt-right-owner", "claim-owner")
        .unwrap()
        .expect("first claim should succeed");

    assert!(store
        .finish_node_cli_completion_claim_applied(
            "evt-right-owner",
            "claim-owner",
            Some("tok-right-owner"),
            Some("billing-right-owner"),
            Some("transaction-right-owner"),
        )
        .unwrap());
    let receipt = store
        .get_node_cli_completion_receipt("evt-right-owner")
        .unwrap()
        .unwrap();
    assert_eq!(receipt.status, "applied");
    assert_eq!(
        receipt.token_usage_event_id.as_deref(),
        Some("tok-right-owner")
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn stale_processing_claim_can_be_reclaimed() {
    let (store, path) = temp_store();
    insert_pending_receipt(&store, "evt-stale-claim");
    store
        .claim_node_cli_completion_receipt("evt-stale-claim", "claim-stale")
        .unwrap()
        .expect("first claim should succeed");
    let stale_at = (chrono::Utc::now() - chrono::Duration::minutes(11)).to_rfc3339();
    store
        .conn
        .lock()
        .unwrap()
        .execute(
            "UPDATE node_cli_completion_receipts SET last_attempt_at = ?2 WHERE event_id = ?1",
            rusqlite::params!["evt-stale-claim", stale_at],
        )
        .unwrap();

    let reclaimed = store
        .claim_node_cli_completion_receipt("evt-stale-claim", "claim-fresh")
        .unwrap()
        .expect("stale claim should be reclaimable");
    assert_eq!(reclaimed.status, "processing");
    assert_eq!(reclaimed.reason.as_deref(), Some("claim-fresh"));
    assert_eq!(reclaimed.attempt_count, 2);
    assert!(!store
        .finish_node_cli_completion_claim_retry(
            "evt-stale-claim",
            "claim-stale",
            "old owner must not finalize",
        )
        .unwrap());
    assert!(store
        .finish_node_cli_completion_claim_retry("evt-stale-claim", "claim-fresh", "retry later",)
        .unwrap());

    drop(store);
    let _ = std::fs::remove_file(path);
}
