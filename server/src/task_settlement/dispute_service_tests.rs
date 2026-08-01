use super::*;
use crate::task_settlement::{
    ledger::LedgerPosting,
    model::{
        CreateSettlementIntent, CreateSettlementReceipt, CreateUsageReceipt, RECEIPT_RECONCILED,
    },
    service, sui_projection_service,
};

fn fixture() -> (Store, std::path::PathBuf, String, String, String) {
    let path = std::env::temp_dir().join(format!(
        "elon-settlement-dispute-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let user = store
        .create_user(
            &format!("dispute-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let project = store
        .create_project(&user.id, "Settlement dispute fixture", None, None)
        .unwrap()
        .project;
    let usage = store
        .insert_task_usage_receipt(CreateUsageReceipt {
            project_id: &project.id,
            subject_type: "task_assignment",
            subject_id: "assignment-dispute",
            source_type: "test",
            source_id: "source-dispute",
            source_digest: "source-digest",
            consumer_user_id: &user.id,
            provider_user_id: Some(&user.id),
            units: 100,
            amount_micros: 1_000_000,
            provider_amount_micros: 800_000,
            currency: "CNY",
            billing_source: "test",
            source_status: "settled",
            occurred_at: "2026-08-01T00:00:00Z",
        })
        .unwrap();
    let intent = store
        .create_task_settlement_intent(CreateSettlementIntent {
            project_id: &project.id,
            matter_id: Some("matter-dispute"),
            assignment_id: Some("assignment-dispute"),
            payer_user_id: &user.id,
            payee_user_id: Some(&user.id),
            idempotency_key: "dispute-intent",
            policy_version: "test.v1",
            policy_digest: "policy-digest",
            usage_receipt_id: &usage.id,
        })
        .unwrap();
    let postings = vec![
        LedgerPosting {
            account_key: "project:compute_expense".to_string(),
            user_id: Some(user.id.clone()),
            side: "debit",
            amount_micros: 1_000_000,
        },
        LedgerPosting {
            account_key: "provider:compute_income".to_string(),
            user_id: Some(user.id.clone()),
            side: "credit",
            amount_micros: 800_000,
        },
        LedgerPosting {
            account_key: "platform:compute_income".to_string(),
            user_id: None,
            side: "credit",
            amount_micros: 200_000,
        },
    ];
    let receipt = store
        .post_task_shadow_settlement(
            CreateSettlementReceipt {
                project_id: &project.id,
                intent_id: &intent.id,
                posting_key: "dispute-posting",
                status: RECEIPT_RECONCILED,
                compute_amount_micros: 1_000_000,
                provider_amount_micros: 800_000,
                platform_amount_micros: 200_000,
                outcome_reward_micros: 0,
                review_reward_micros: 0,
                currency: "CNY",
                accepted_matter_id: Some("matter-dispute"),
                reason: "dispute fixture",
                receipt_kind: "standard",
                correction_id: None,
            },
            &postings,
        )
        .unwrap();
    (store, path, user.id, project.id, receipt.id)
}

fn request(summary: &str) -> OpenSettlementDisputeRequest {
    OpenSettlementDisputeRequest {
        reason_code: "amount".to_string(),
        summary: summary.to_string(),
        evidence_ref: Some("artifact:billing-evidence".to_string()),
    }
}

#[test]
fn dispute_lifecycle_is_append_only_idempotent_and_reopenable_after_rejection() {
    let (store, path, user_id, project_id, receipt_id) = fixture();
    let opened = open(
        &store,
        &project_id,
        &receipt_id,
        &user_id,
        &request("计量金额与节点原始记录不一致"),
    )
    .unwrap();
    let replay = open(
        &store,
        &project_id,
        &receipt_id,
        &user_id,
        &request("计量金额与节点原始记录不一致"),
    )
    .unwrap();
    assert_eq!(opened.dispute.id, replay.dispute.id);
    assert_eq!(opened.events.len(), 1);
    assert!(open(
        &store,
        &project_id,
        &receipt_id,
        &user_id,
        &request("另一份争议内容不应覆盖待审核案件"),
    )
    .is_err());

    let rejected = resolve(
        &store,
        &project_id,
        &opened.dispute.id,
        &user_id,
        &ResolveSettlementDisputeRequest {
            decision: "reject".to_string(),
            note: "原始节点回执与计量结果一致".to_string(),
        },
    )
    .unwrap();
    assert_eq!(rejected.dispute.status, DISPUTE_REJECTED);
    assert_eq!(rejected.events.len(), 2);
    assert!(!rejected.blocks_projection);

    let reopened = open(
        &store,
        &project_id,
        &receipt_id,
        &user_id,
        &request("补充证据后重新提交金额争议"),
    )
    .unwrap();
    assert_ne!(reopened.dispute.id, opened.dispute.id);
    let withdrawn = withdraw(
        &store,
        &project_id,
        &reopened.dispute.id,
        &user_id,
        &WithdrawSettlementDisputeRequest {
            note: "证据仍需补充，先撤回".to_string(),
        },
    )
    .unwrap();
    assert_eq!(withdrawn.dispute.status, DISPUTE_WITHDRAWN);
    assert_eq!(list(&store, &project_id, &receipt_id).unwrap().len(), 2);
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn open_or_accepted_dispute_blocks_existing_and_future_sui_projection() {
    let (store, path, user_id, project_id, receipt_id) = fixture();
    let before = service::receipt_detail(&store, &project_id, &receipt_id).unwrap();
    let package =
        sui_projection_service::prepare(&store, &project_id, &receipt_id, &user_id, "testnet")
            .unwrap();
    assert_eq!(package.submission_readiness, "adapter_required");
    let corrupted =
        sui_projection_service::prepare(&store, &project_id, &receipt_id, &user_id, "devnet")
            .unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute(
        "UPDATE task_sui_projection_packages SET projection_digest='tampered' WHERE id=?1",
        [&corrupted.id],
    )
    .unwrap();
    drop(conn);
    assert_eq!(
        sui_projection_service::verify(&store, &project_id, &corrupted.id)
            .unwrap()
            .submission_readiness,
        "integrity_conflict"
    );
    let opened = open(
        &store,
        &project_id,
        &receipt_id,
        &user_id,
        &request("该影子凭证的策略版本需要重新复核"),
    )
    .unwrap();
    assert!(opened.blocks_projection);
    assert!(service::sui_envelope(&store, &project_id, &receipt_id).is_err());
    assert!(
        sui_projection_service::prepare(&store, &project_id, &receipt_id, &user_id, "mainnet",)
            .is_err()
    );
    let blocked = sui_projection_service::list(&store, &project_id).unwrap();
    assert_eq!(
        blocked
            .iter()
            .find(|item| item.id == package.id)
            .unwrap()
            .submission_readiness,
        "dispute_blocked"
    );
    assert_eq!(
        blocked
            .iter()
            .find(|item| item.id == corrupted.id)
            .unwrap()
            .submission_readiness,
        "integrity_conflict"
    );

    let accepted = resolve(
        &store,
        &project_id,
        &opened.dispute.id,
        &user_id,
        &ResolveSettlementDisputeRequest {
            decision: "accept".to_string(),
            note: "确认策略摘要与实际验收依据不一致，需另建纠正凭证".to_string(),
        },
    )
    .unwrap();
    assert_eq!(accepted.dispute.status, DISPUTE_ACCEPTED);
    let after = service::receipt_detail(&store, &project_id, &receipt_id).unwrap();
    assert_eq!(after.receipt.posting_key, before.receipt.posting_key);
    assert_eq!(
        after.receipt.compute_amount_micros,
        before.receipt.compute_amount_micros
    );
    assert_eq!(
        after.ledger_transaction.unwrap().entries.len(),
        before.ledger_transaction.unwrap().entries.len()
    );
    assert!(open(
        &store,
        &project_id,
        &receipt_id,
        &user_id,
        &request("已接受争议后不能重复建案"),
    )
    .is_err());
    assert_eq!(
        sui_projection_service::detail(&store, &project_id, &package.id)
            .unwrap()
            .submission_readiness,
        "dispute_blocked"
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejected_dispute_restores_projection_readiness_and_inputs_are_bounded() {
    let (store, path, user_id, project_id, receipt_id) = fixture();
    let opened = open(
        &store,
        &project_id,
        &receipt_id,
        &user_id,
        &request("来源证据需要人工确认后再生成投影"),
    )
    .unwrap();
    resolve(
        &store,
        &project_id,
        &opened.dispute.id,
        &user_id,
        &ResolveSettlementDisputeRequest {
            decision: "rejected".to_string(),
            note: "证据引用与不可变回执一致".to_string(),
        },
    )
    .unwrap();
    let package =
        sui_projection_service::prepare(&store, &project_id, &receipt_id, &user_id, "devnet")
            .unwrap();
    assert_eq!(package.submission_readiness, "adapter_required");

    let invalid = OpenSettlementDisputeRequest {
        reason_code: "unknown".to_string(),
        summary: "短".to_string(),
        evidence_ref: None,
    };
    assert!(open(&store, &project_id, &receipt_id, &user_id, &invalid).is_err());
    drop(store);
    let _ = std::fs::remove_file(path);
}
