use super::*;
use crate::{
    group_ai::types::CreateMatterRecord,
    task_settlement::{
        dispute_service,
        ledger::compute_mirror_postings,
        model::{
            CreateSettlementCorrection, CreateSettlementIntent, CreateSettlementReceipt,
            CreateUsageReceipt, OpenSettlementDisputeRequest, ResolveSettlementDisputeRequest,
            SettlementCorrectionDetail, RECEIPT_RECONCILED,
        },
        sui_projection_service,
    },
};
use serde_json::json;

struct Fixture {
    store: Store,
    path: std::path::PathBuf,
    user_id: String,
    project_id: String,
    correction: SettlementCorrectionDetail,
}

fn fixture(posted: bool) -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-sui-correction-projection-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let user = store
        .create_user(
            &format!(
                "sui-correction-{}@example.com",
                uuid::Uuid::new_v4().simple()
            ),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let project = store
        .create_project(&user.id, "Sui correction projection fixture", None, None)
        .unwrap()
        .project;
    let usage = store
        .insert_task_usage_receipt(CreateUsageReceipt {
            project_id: &project.id,
            subject_type: "task_assignment",
            subject_id: "assignment-sui-correction",
            source_type: "test",
            source_id: "source-sui-correction",
            source_digest: "source-digest-sui-correction",
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
            matter_id: Some("matter-sui-correction-original"),
            assignment_id: Some("assignment-sui-correction"),
            payer_user_id: &user.id,
            payee_user_id: Some(&user.id),
            idempotency_key: "sui-correction-original-intent",
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
                posting_key: "sui-correction-original-posting",
                status: RECEIPT_RECONCILED,
                compute_amount_micros: 1_000_000,
                provider_amount_micros: 800_000,
                platform_amount_micros: 200_000,
                outcome_reward_micros: 0,
                review_reward_micros: 0,
                currency: "CNY",
                accepted_matter_id: Some("matter-sui-correction-original"),
                reason: "Sui correction original fixture",
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
            summary: "原计量金额与节点证据不一致".into(),
            evidence_ref: Some("artifact:sui-correction-evidence".into()),
        },
    )
    .unwrap();
    let dispute = dispute_service::resolve(
        &store,
        &project.id,
        &dispute.dispute.id,
        &user.id,
        &ResolveSettlementDisputeRequest {
            decision: "accept".into(),
            note: "确认需要追加冲销和替换凭证".into(),
        },
    )
    .unwrap();
    let channel = store
        .list_project_space_channels(&user.id, &project.id)
        .unwrap()
        .into_iter()
        .find(|channel| channel.kind == "ai_development")
        .unwrap();
    let correction = store
        .create_task_settlement_correction_with_matter(
            CreateSettlementCorrection {
                project_id: &project.id,
                dispute_id: &dispute.dispute.id,
                corrected_compute_amount_micros: 600_000,
                corrected_provider_amount_micros: 500_000,
                summary: "根据原始节点日志重新核对影子金额",
                evidence_ref: Some("artifact:sui-correction-review"),
                actor_user_id: &user.id,
            },
            CreateMatterRecord {
                project_id: project.id.clone(),
                channel_id: channel.id,
                requester_user_id: user.id.clone(),
                source_message_id: None,
                title: "Sui 纠正投影核查".into(),
                brief: "只核查链外影子纠正，不移动资金".into(),
                collaboration_mode: "critic".into(),
                participant_user_ids: vec![user.id.clone()],
                node_policy_json: json!({}),
                acceptance_criteria: vec!["冲销与替换必须原子出现".into()],
                plan_json: json!({"roles": []}),
            },
        )
        .unwrap();
    let correction = if posted {
        store
            .update_project_ai_matter_status(
                &project.id,
                &correction.correction.correction_matter_id,
                "done",
                Some(&user.id),
                Some("accepted"),
            )
            .unwrap();
        store
            .post_task_settlement_correction(&project.id, &correction.correction.id, &user.id)
            .unwrap()
    } else {
        correction
    };
    Fixture {
        store,
        path,
        user_id: user.id,
        project_id: project.id,
        correction,
    }
}

#[test]
fn package_requires_posted_correction_and_binds_both_legs_idempotently() {
    let pending = fixture(false);
    assert!(prepare(
        &pending.store,
        &pending.project_id,
        &pending.correction.correction.id,
        &pending.user_id,
        "testnet",
    )
    .is_err());
    drop(pending.store);
    let _ = std::fs::remove_file(pending.path);

    let fixture = fixture(true);
    let first = prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "testnet",
    )
    .unwrap();
    let replay = prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "TESTNET",
    )
    .unwrap();
    assert_eq!(first.id, replay.id);
    assert!(first.envelope.atomic_bundle);
    assert_eq!(first.envelope.reversal.receipt_kind, "correction_reversal");
    assert_eq!(
        first.envelope.replacement.receipt_kind,
        "correction_replacement"
    );
    assert_eq!(first.integrity_status, "verified");
    assert_eq!(first.submission_readiness, "adapter_required");
    assert_eq!(first.network_submission, "not_submitted");
    assert_eq!(first.submission_attempts, 0);
    let mainnet = prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "mainnet",
    )
    .unwrap();
    assert_ne!(mainnet.id, first.id);
    assert_ne!(mainnet.projection_digest, first.projection_digest);
    assert_eq!(list(&fixture.store, &fixture.project_id).unwrap().len(), 2);
    assert!(sui_projection_service::prepare(
        &fixture.store,
        &fixture.project_id,
        &first.reversal_receipt_id,
        &fixture.user_id,
        "testnet",
    )
    .is_err());
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}

#[test]
fn replacement_dispute_blocks_readiness_and_tampering_persists_conflict() {
    let fixture = fixture(true);
    let package = prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "devnet",
    )
    .unwrap();
    dispute_service::open(
        &fixture.store,
        &fixture.project_id,
        &package.replacement_receipt_id,
        &fixture.user_id,
        &OpenSettlementDisputeRequest {
            reason_code: "source_evidence".into(),
            summary: "替换凭证出现新的来源证据疑问".into(),
            evidence_ref: Some("artifact:replacement-dispute".into()),
        },
    )
    .unwrap();
    assert_eq!(
        detail(&fixture.store, &fixture.project_id, &package.id)
            .unwrap()
            .submission_readiness,
        "dispute_blocked"
    );
    let conn = rusqlite::Connection::open(&fixture.path).unwrap();
    conn.execute(
        "UPDATE task_sui_correction_projection_packages
            SET projection_digest='tampered'
          WHERE id=?1",
        [&package.id],
    )
    .unwrap();
    drop(conn);
    let verified = verify(&fixture.store, &fixture.project_id, &package.id).unwrap();
    assert_eq!(verified.integrity_status, "conflict");
    assert_eq!(verified.submission_readiness, "integrity_conflict");
    assert!(verified.last_error.as_deref().unwrap().contains("禁止"));
    assert!(prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "devnet",
    )
    .is_err());
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}
