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
    },
};
use serde_json::json;

struct Fixture {
    store: Store,
    path: std::path::PathBuf,
    user_id: String,
    project_id: String,
    receipt_id: String,
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-settlement-lineage-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let user = store
        .create_user(
            &format!("lineage-{}@example.com", uuid::Uuid::new_v4().simple()),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let project = store
        .create_project(&user.id, "Settlement lineage fixture", None, None)
        .unwrap()
        .project;
    let usage = store
        .insert_task_usage_receipt(CreateUsageReceipt {
            project_id: &project.id,
            subject_type: "task_assignment",
            subject_id: "assignment-lineage",
            source_type: "test",
            source_id: "source-lineage",
            source_digest: "source-digest-lineage",
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
            matter_id: Some("matter-lineage-original"),
            assignment_id: Some("assignment-lineage"),
            payer_user_id: &user.id,
            payee_user_id: Some(&user.id),
            idempotency_key: "lineage-original-intent",
            policy_version: "test.v1",
            policy_digest: "policy-digest-lineage",
            usage_receipt_id: &usage.id,
        })
        .unwrap();
    let postings = compute_mirror_postings(&user.id, Some(&user.id), 1_000_000, 800_000).unwrap();
    let receipt = store
        .post_task_shadow_settlement(
            CreateSettlementReceipt {
                project_id: &project.id,
                intent_id: &intent.id,
                posting_key: "lineage-original-posting",
                status: RECEIPT_RECONCILED,
                compute_amount_micros: 1_000_000,
                provider_amount_micros: 800_000,
                platform_amount_micros: 200_000,
                outcome_reward_micros: 0,
                review_reward_micros: 0,
                currency: "CNY",
                accepted_matter_id: Some("matter-lineage-original"),
                reason: "lineage original fixture",
                receipt_kind: RECEIPT_KIND_STANDARD,
                correction_id: None,
            },
            &postings,
        )
        .unwrap();
    Fixture {
        store,
        path,
        user_id: user.id,
        project_id: project.id,
        receipt_id: receipt.id,
    }
}

fn create_correction(
    fixture: &Fixture,
    receipt_id: &str,
    compute: i64,
    provider: i64,
    posted: bool,
) -> SettlementCorrectionDetail {
    let dispute = dispute_service::open(
        &fixture.store,
        &fixture.project_id,
        receipt_id,
        &fixture.user_id,
        &OpenSettlementDisputeRequest {
            reason_code: "amount".into(),
            summary: format!("凭证 {} 的金额需要再次核查", short(receipt_id)),
            evidence_ref: Some(format!("artifact:lineage-{}", short(receipt_id))),
        },
    )
    .unwrap();
    let dispute = dispute_service::resolve(
        &fixture.store,
        &fixture.project_id,
        &dispute.dispute.id,
        &fixture.user_id,
        &ResolveSettlementDisputeRequest {
            decision: "accept".into(),
            note: "确认建立独立纠正 Matter".into(),
        },
    )
    .unwrap();
    let channel = fixture
        .store
        .list_project_space_channels(&fixture.user_id, &fixture.project_id)
        .unwrap()
        .into_iter()
        .find(|channel| channel.kind == "ai_development")
        .unwrap();
    let correction = fixture
        .store
        .create_task_settlement_correction_with_matter(
            CreateSettlementCorrection {
                project_id: &fixture.project_id,
                dispute_id: &dispute.dispute.id,
                corrected_compute_amount_micros: compute,
                corrected_provider_amount_micros: provider,
                summary: "根据最新证据重算计算与节点影子金额",
                evidence_ref: Some("artifact:lineage-correction"),
                actor_user_id: &fixture.user_id,
            },
            CreateMatterRecord {
                project_id: fixture.project_id.clone(),
                channel_id: channel.id,
                requester_user_id: fixture.user_id.clone(),
                source_message_id: None,
                title: format!("纠正链核查 {}", short(receipt_id)),
                brief: "只核查追加式影子结算纠正".into(),
                collaboration_mode: "critic".into(),
                participant_user_ids: vec![fixture.user_id.clone()],
                node_policy_json: json!({}),
                acceptance_criteria: vec!["原凭证不可改写".into()],
                plan_json: json!({"roles": []}),
            },
        )
        .unwrap();
    if !posted {
        return correction;
    }
    fixture
        .store
        .update_project_ai_matter_status(
            &fixture.project_id,
            &correction.correction.correction_matter_id,
            "done",
            Some(&fixture.user_id),
            Some("accepted"),
        )
        .unwrap();
    fixture
        .store
        .post_task_settlement_correction(
            &fixture.project_id,
            &correction.correction.id,
            &fixture.user_id,
        )
        .unwrap()
}

#[test]
fn uncorrected_standard_receipt_is_its_own_effective_root() {
    let fixture = fixture();
    let lineage = resolve(&fixture.store, &fixture.project_id, &fixture.receipt_id).unwrap();
    assert_eq!(lineage.requested_position, "effective_standard");
    assert_eq!(lineage.root_receipt.id, fixture.receipt_id);
    assert_eq!(lineage.effective_receipt.id, fixture.receipt_id);
    assert_eq!(lineage.depth, 0);
    assert!(lineage.posted_corrections.is_empty());
    assert!(!lineage.effective_has_blocking_dispute);
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}

#[test]
fn any_leg_resolves_two_step_chain_and_reports_pending_next_plan() {
    let fixture = fixture();
    let first = create_correction(&fixture, &fixture.receipt_id, 700_000, 550_000, true);
    let first_reversal = first.reversal_receipt.as_ref().unwrap().id.clone();
    let first_replacement = first.replacement_receipt.as_ref().unwrap().id.clone();
    let second = create_correction(&fixture, &first_replacement, 650_000, 500_000, true);
    let second_replacement = second.replacement_receipt.as_ref().unwrap().id.clone();
    let lineage = resolve(&fixture.store, &fixture.project_id, &first_reversal).unwrap();
    assert_eq!(lineage.requested_position, "correction_reversal");
    assert_eq!(lineage.root_receipt.id, fixture.receipt_id);
    assert_eq!(lineage.effective_receipt.id, second_replacement);
    assert_eq!(lineage.depth, 2);
    assert_eq!(
        lineage.posted_corrections[0].correction.id,
        first.correction.id
    );
    assert_eq!(
        lineage.posted_corrections[1].correction.id,
        second.correction.id
    );

    let pending = create_correction(&fixture, &second_replacement, 625_000, 475_000, false);
    let pending_lineage =
        resolve(&fixture.store, &fixture.project_id, &second_replacement).unwrap();
    assert_eq!(pending_lineage.requested_position, "effective_replacement");
    assert_eq!(pending_lineage.effective_receipt.id, second_replacement);
    assert_eq!(pending_lineage.depth, 2);
    assert_eq!(pending_lineage.non_posted_corrections.len(), 1);
    assert_eq!(
        pending_lineage.non_posted_corrections[0].correction.id,
        pending.correction.id
    );
    assert!(pending_lineage.effective_has_blocking_dispute);
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}

#[test]
fn corrupted_cycle_is_rejected_instead_of_guessing_an_effective_receipt() {
    let fixture = fixture();
    let correction = create_correction(&fixture, &fixture.receipt_id, 700_000, 550_000, true);
    let replacement = correction.replacement_receipt.as_ref().unwrap().id.clone();
    let conn = rusqlite::Connection::open(&fixture.path).unwrap();
    conn.execute(
        "UPDATE task_settlement_corrections
            SET original_settlement_receipt_id=?2
          WHERE id=?1",
        [&correction.correction.id, &replacement],
    )
    .unwrap();
    drop(conn);
    let error = resolve(&fixture.store, &fixture.project_id, &replacement).unwrap_err();
    assert!(format!("{error:#}").contains("循环"));
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}

#[test]
fn exactly_32_corrections_are_supported_and_the_33rd_is_rejected() {
    let fixture = fixture();
    let mut effective_receipt_id = fixture.receipt_id.clone();
    for index in 0..MAX_LINEAGE_DEPTH {
        let correction = create_correction(
            &fixture,
            &effective_receipt_id,
            700_000 - index as i64 * 1_000,
            550_000 - index as i64 * 1_000,
            true,
        );
        effective_receipt_id = correction.replacement_receipt.unwrap().id;
    }

    let lineage = resolve(&fixture.store, &fixture.project_id, &fixture.receipt_id).unwrap();
    assert_eq!(lineage.depth, MAX_LINEAGE_DEPTH);
    assert_eq!(lineage.effective_receipt.id, effective_receipt_id);

    let correction = create_correction(&fixture, &effective_receipt_id, 650_000, 500_000, true);
    let over_limit_receipt_id = correction.replacement_receipt.unwrap().id;
    let error = resolve(&fixture.store, &fixture.project_id, &over_limit_receipt_id).unwrap_err();
    assert!(format!("{error:#}").contains("超过最大深度 32"));
    drop(fixture.store);
    let _ = std::fs::remove_file(fixture.path);
}

fn short(value: &str) -> &str {
    value.get(..value.len().min(8)).unwrap_or(value)
}
