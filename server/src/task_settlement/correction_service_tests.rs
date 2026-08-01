use super::*;
use crate::{
    group_ai::types::CreateMatterRecord,
    task_settlement::{
        dispute_service,
        ledger::compute_mirror_postings,
        model::{
            CreateSettlementIntent, CreateSettlementReceipt, CreateUsageReceipt,
            OpenSettlementDisputeRequest, ResolveSettlementDisputeRequest, RECEIPT_RECONCILED,
        },
        service, sui_projection,
    },
};
use serde_json::json;

struct Fixture {
    store: Store,
    path: std::path::PathBuf,
    user_id: String,
    project_id: String,
    receipt_id: String,
    dispute_id: String,
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-settlement-correction-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let user = store
        .create_user(
            &format!("correction-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let project = store
        .create_project(&user.id, "Settlement correction fixture", None, None)
        .unwrap()
        .project;
    let usage = store
        .insert_task_usage_receipt(CreateUsageReceipt {
            project_id: &project.id,
            subject_type: "task_assignment",
            subject_id: "assignment-correction",
            source_type: "test",
            source_id: "source-correction",
            source_digest: "source-digest-correction",
            consumer_user_id: &user.id,
            provider_user_id: Some(&user.id),
            units: 100,
            amount_micros: 1_000_000,
            provider_amount_micros: 800_000,
            currency: "CNY",
            billing_source: "test",
            source_status: "settled",
            occurred_at: "2026-08-02T00:00:00Z",
        })
        .unwrap();
    let intent = store
        .create_task_settlement_intent(CreateSettlementIntent {
            project_id: &project.id,
            matter_id: Some("matter-original"),
            assignment_id: Some("assignment-correction"),
            payer_user_id: &user.id,
            payee_user_id: Some(&user.id),
            idempotency_key: "correction-original-intent",
            policy_version: "test.v1",
            policy_digest: "test-policy-digest",
            usage_receipt_id: &usage.id,
        })
        .unwrap();
    let postings = compute_mirror_postings(&user.id, Some(&user.id), 1_000_000, 800_000).unwrap();
    let receipt = store
        .post_task_shadow_settlement(
            CreateSettlementReceipt {
                project_id: &project.id,
                intent_id: &intent.id,
                posting_key: "correction-original-posting",
                status: RECEIPT_RECONCILED,
                compute_amount_micros: 1_000_000,
                provider_amount_micros: 800_000,
                platform_amount_micros: 200_000,
                outcome_reward_micros: 0,
                review_reward_micros: 0,
                currency: "CNY",
                accepted_matter_id: Some("matter-original"),
                reason: "correction original fixture",
                receipt_kind: "standard",
                correction_id: None,
            },
            &postings,
        )
        .unwrap();
    let dispute = dispute_service::open(
        &store,
        &project.id,
        &receipt.id,
        &user.id,
        &OpenSettlementDisputeRequest {
            reason_code: "amount".into(),
            summary: "原计量金额与证据记录不一致".into(),
            evidence_ref: Some("artifact:correction-evidence".into()),
        },
    )
    .unwrap();
    let accepted = dispute_service::resolve(
        &store,
        &project.id,
        &dispute.dispute.id,
        &user.id,
        &ResolveSettlementDisputeRequest {
            decision: "accept".into(),
            note: "确认需要通过追加凭证完成纠正".into(),
        },
    )
    .unwrap();
    Fixture {
        store,
        path,
        user_id: user.id,
        project_id: project.id,
        receipt_id: receipt.id,
        dispute_id: accepted.dispute.id,
    }
}

fn request(compute: i64, provider: i64) -> CreateSettlementCorrectionRequest {
    CreateSettlementCorrectionRequest {
        corrected_compute_amount_micros: compute,
        corrected_provider_amount_micros: provider,
        summary: "依据节点原始日志重新核对金额和节点分配".into(),
        evidence_ref: Some("artifact:correction-review".into()),
    }
}

#[test]
fn accepted_dispute_requires_matter_acceptance_then_posts_atomic_reversal_and_replacement() {
    let fixture = fixture();
    let created = create(
        &fixture.store,
        &fixture.project_id,
        &fixture.dispute_id,
        &fixture.user_id,
        &request(600_000, 500_000),
        &[],
    )
    .unwrap();
    assert_eq!(created.correction.status, "matter_pending");
    assert_eq!(created.correction.matter_status, "plan_ready");
    assert!(created.reversal_receipt.is_none());
    assert!(created.replacement_receipt.is_none());

    let replay = create(
        &fixture.store,
        &fixture.project_id,
        &fixture.dispute_id,
        &fixture.user_id,
        &request(600_000, 500_000),
        &[],
    )
    .unwrap();
    assert_eq!(replay.correction.id, created.correction.id);
    assert!(create(
        &fixture.store,
        &fixture.project_id,
        &fixture.dispute_id,
        &fixture.user_id,
        &request(650_000, 500_000),
        &[],
    )
    .is_err());
    assert!(fixture
        .store
        .post_task_settlement_correction(
            &fixture.project_id,
            &created.correction.id,
            &fixture.user_id,
        )
        .is_err());

    fixture
        .store
        .update_project_ai_matter_status(
            &fixture.project_id,
            &created.correction.correction_matter_id,
            "done",
            Some(&fixture.user_id),
            Some("accepted"),
        )
        .unwrap();
    assert!(finalize(
        &fixture.store,
        &fixture.project_id,
        &created.correction.id,
        &fixture.user_id,
    )
    .is_err());
    let posted = fixture
        .store
        .post_task_settlement_correction(
            &fixture.project_id,
            &created.correction.id,
            &fixture.user_id,
        )
        .unwrap();
    assert_eq!(posted.correction.status, "posted");
    let reversal = posted.reversal_receipt.as_ref().unwrap();
    let replacement = posted.replacement_receipt.as_ref().unwrap();
    assert_eq!(reversal.receipt_kind, "correction_reversal");
    assert_eq!(replacement.receipt_kind, "correction_replacement");
    assert_eq!(replacement.compute_amount_micros, 600_000);
    assert_eq!(replacement.provider_amount_micros, 500_000);
    assert_eq!(posted.events.len(), 2);
    assert!(dispute_service::open(
        &fixture.store,
        &fixture.project_id,
        &reversal.id,
        &fixture.user_id,
        &OpenSettlementDisputeRequest {
            reason_code: "amount".into(),
            summary: "冲销凭证不应成为新的争议来源".into(),
            evidence_ref: None,
        },
    )
    .is_err());

    for receipt in [reversal, replacement] {
        let ledger = fixture
            .store
            .task_ledger_transaction_for_receipt(&receipt.id)
            .unwrap()
            .unwrap();
        let debit: i64 = ledger
            .entries
            .iter()
            .filter(|entry| entry.side == "debit")
            .map(|entry| entry.amount_micros)
            .sum();
        let credit: i64 = ledger
            .entries
            .iter()
            .filter(|entry| entry.side == "credit")
            .map(|entry| entry.amount_micros)
            .sum();
        assert_eq!(debit, credit);
        assert!(sui_projection::envelope(receipt).is_err());
    }
    let overview = service::overview(&fixture.store, &fixture.project_id).unwrap();
    assert_eq!(overview.totals.compute_amount_micros, 600_000);
    assert_eq!(overview.totals.provider_amount_micros, 500_000);
    assert_eq!(overview.totals.platform_amount_micros, 100_000);
    let replay_post = fixture
        .store
        .post_task_settlement_correction(
            &fixture.project_id,
            &created.correction.id,
            &fixture.user_id,
        )
        .unwrap();
    assert_eq!(replay_post.correction.id, posted.correction.id);
    assert_eq!(
        fixture
            .store
            .list_task_settlement_receipts(&fixture.project_id, 10)
            .unwrap()
            .len(),
        3
    );
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}

#[test]
fn canceled_correction_writes_no_receipts_and_allows_replanning() {
    let fixture = fixture();
    let first = create(
        &fixture.store,
        &fixture.project_id,
        &fixture.dispute_id,
        &fixture.user_id,
        &request(700_000, 550_000),
        &[],
    )
    .unwrap();
    fixture
        .store
        .update_project_ai_matter_status(
            &fixture.project_id,
            &first.correction.correction_matter_id,
            "canceled",
            Some(&fixture.user_id),
            Some("canceled"),
        )
        .unwrap();
    assert_eq!(
        service::void_canceled_matter(
            &fixture.store,
            &fixture.project_id,
            &first.correction.correction_matter_id,
        )
        .unwrap(),
        1
    );
    let canceled = fixture
        .store
        .task_settlement_correction_detail(&fixture.project_id, &first.correction.id)
        .unwrap()
        .unwrap();
    assert_eq!(canceled.correction.status, "canceled");
    assert_eq!(canceled.events.len(), 2);
    assert_eq!(
        fixture
            .store
            .list_task_settlement_receipts(&fixture.project_id, 10)
            .unwrap()
            .len(),
        1
    );
    let replanned = create(
        &fixture.store,
        &fixture.project_id,
        &fixture.dispute_id,
        &fixture.user_id,
        &request(650_000, 500_000),
        &[],
    )
    .unwrap();
    assert_ne!(replanned.correction.id, first.correction.id);
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}

#[test]
fn store_rejects_unaccepted_dispute_and_invalid_provider_amount() {
    let fixture = fixture();
    assert!(create(
        &fixture.store,
        &fixture.project_id,
        &fixture.dispute_id,
        &fixture.user_id,
        &request(100_000, 100_001),
        &[],
    )
    .is_err());
    let channel = fixture
        .store
        .list_project_space_channels(&fixture.user_id, &fixture.project_id)
        .unwrap()
        .into_iter()
        .find(|channel| channel.kind == "ai_development")
        .unwrap();
    let fake = CreateMatterRecord {
        project_id: fixture.project_id.clone(),
        channel_id: channel.id,
        requester_user_id: fixture.user_id.clone(),
        source_message_id: None,
        title: "无效纠正".into(),
        brief: "无效纠正测试".into(),
        collaboration_mode: "solo".into(),
        participant_user_ids: vec![fixture.user_id.clone()],
        node_policy_json: json!({}),
        acceptance_criteria: vec!["必须拒绝".into()],
        plan_json: json!({"roles": []}),
    };
    assert!(fixture
        .store
        .create_task_settlement_correction_with_matter(
            CreateSettlementCorrection {
                project_id: &fixture.project_id,
                dispute_id: "missing-dispute",
                corrected_compute_amount_micros: 1,
                corrected_provider_amount_micros: 0,
                summary: "不存在的争议不能创建纠正流程",
                evidence_ref: None,
                actor_user_id: &fixture.user_id,
            },
            fake,
        )
        .is_err());
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}
