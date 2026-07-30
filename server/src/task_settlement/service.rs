use anyhow::{anyhow, bail, Result};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{open_commerce_model::OpenCommerceInvocation, store::Store};

use super::{
    ledger::compute_mirror_postings,
    model::{
        CreateSettlementIntent, CreateSettlementReceipt, CreateUsageReceipt,
        SettlementReceiptDetail, TaskEconomyOverview, TaskEconomyTotals, UsageReceipt,
        CURRENCY_CNY, ECONOMY_SCHEMA, INTENT_PENDING, INTENT_POSTED, INTENT_VOIDED,
        RECEIPT_RECONCILED, SUBJECT_COMMERCE_INVOCATION, SUBJECT_TASK_ASSIGNMENT,
    },
    sui_projection,
};

const POLICY_VERSION: &str = "task-shadow-settlement.v1";
const RUNTIME_FLAG: &str = "ELON_TASK_SHADOW_SETTLEMENT_ENABLED";
const MICROS_PER_FEN: i64 = 10_000;
const ACCEPTED_MATTER_STATUS: &str = "done";

pub(crate) fn runtime_enabled() -> bool {
    std::env::var(RUNTIME_FLAG)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub(crate) fn overview(store: &Store, project_id: &str) -> Result<TaskEconomyOverview> {
    let setting = store.task_economy_project_setting(project_id)?;
    let usage_receipts = store.list_task_usage_receipts(project_id, 100)?;
    let intents = store.list_task_settlement_intents(project_id, 100)?;
    let settlement_receipts = store.list_task_settlement_receipts(project_id, 100)?;
    let totals = TaskEconomyTotals {
        usage_receipts: usage_receipts.len(),
        pending_intents: intents
            .iter()
            .filter(|intent| intent.status == INTENT_PENDING)
            .count(),
        posted_intents: intents
            .iter()
            .filter(|intent| intent.status == INTENT_POSTED)
            .count(),
        voided_intents: intents
            .iter()
            .filter(|intent| intent.status == INTENT_VOIDED)
            .count(),
        settlement_receipts: settlement_receipts.len(),
        compute_amount_micros: checked_sum(
            settlement_receipts
                .iter()
                .map(|receipt| receipt.compute_amount_micros),
            "计算金额",
        )?,
        provider_amount_micros: checked_sum(
            settlement_receipts
                .iter()
                .map(|receipt| receipt.provider_amount_micros),
            "节点金额",
        )?,
        platform_amount_micros: checked_sum(
            settlement_receipts
                .iter()
                .map(|receipt| receipt.platform_amount_micros),
            "平台金额",
        )?,
    };
    Ok(TaskEconomyOverview {
        schema: ECONOMY_SCHEMA,
        project_id: project_id.trim().to_string(),
        runtime_enabled: runtime_enabled(),
        project_setting: setting,
        shadow_only: true,
        totals,
        usage_receipts,
        intents,
        settlement_receipts,
    })
}

pub(crate) fn receipt_detail(
    store: &Store,
    project_id: &str,
    receipt_id: &str,
) -> Result<SettlementReceiptDetail> {
    let receipt = store
        .task_settlement_receipt(project_id, receipt_id)?
        .ok_or_else(|| anyhow!("影子结算凭证不存在"))?;
    let intent = store
        .task_settlement_intent(&receipt.intent_id)?
        .ok_or_else(|| anyhow!("影子结算意图不存在"))?;
    let usage_receipts = store.task_usage_receipts_for_intent(&intent.id)?;
    let ledger_transaction = store.task_ledger_transaction_for_receipt(&receipt.id)?;
    Ok(SettlementReceiptDetail {
        receipt,
        intent,
        usage_receipts,
        ledger_transaction,
    })
}

pub(crate) fn sui_envelope(
    store: &Store,
    project_id: &str,
    receipt_id: &str,
) -> Result<super::model::SuiSettlementEnvelope> {
    let detail = receipt_detail(store, project_id, receipt_id)?;
    sui_projection::envelope(&detail.receipt)
}

pub(crate) fn capture_task_assignment(
    store: &Store,
    project_id: &str,
    matter_id: &str,
    assignment_id: &str,
    compute_call_id: &str,
) -> Result<Option<UsageReceipt>> {
    if !project_active(store, project_id)? {
        return Ok(None);
    }
    capture_task_assignment_facts(store, project_id, matter_id, assignment_id, compute_call_id)
        .map(Some)
}

fn capture_task_assignment_facts(
    store: &Store,
    project_id: &str,
    matter_id: &str,
    assignment_id: &str,
    compute_call_id: &str,
) -> Result<UsageReceipt> {
    let matter = store
        .get_project_ai_matter(project_id, matter_id)?
        .ok_or_else(|| anyhow!("Matter 不存在"))?;
    let assignment = store
        .get_project_ai_matter_assignment(assignment_id)?
        .ok_or_else(|| anyhow!("Matter assignment 不存在"))?;
    if assignment.matter_id != matter.id {
        bail!("Matter assignment 不属于当前 Matter");
    }
    let run = store
        .get_node_compute_run_by_compute_call_id(compute_call_id)?
        .ok_or_else(|| anyhow!("节点执行记录不存在，不能创建影子用量凭证"))?;
    if run.node_id != assignment.node_id {
        bail!("节点执行记录与 Assignment 节点不匹配");
    }
    let amount_micros = fen_to_micros(run.billed_cost_rmb_fen)?;
    let provider_amount_micros = fen_to_micros(run.provider_earned_fen)?;
    let units = run
        .prompt_tokens
        .checked_add(run.completion_tokens)
        .ok_or_else(|| anyhow!("Token 用量溢出"))?
        .max(0);
    let provider_user_id = run
        .provider_user_id
        .as_deref()
        .or(Some(assignment.provider_user_id.as_str()));
    let source_digest = digest_json(&json!({
        "schema": "task_usage.node_compute_run.v1",
        "compute_call_id": run.compute_call_id,
        "node_id": run.node_id,
        "consumer_user_id": run.consumer_user_id,
        "provider_user_id": provider_user_id,
        "status": run.status,
        "billing_source": run.billing_source,
        "prompt_tokens": run.prompt_tokens,
        "completion_tokens": run.completion_tokens,
        "billed_cost_rmb_fen": run.billed_cost_rmb_fen,
        "provider_earned_fen": run.provider_earned_fen,
        "settlement_status": run.settlement_status
    }))?;
    let usage = store.insert_task_usage_receipt(CreateUsageReceipt {
        project_id,
        subject_type: SUBJECT_TASK_ASSIGNMENT,
        subject_id: assignment_id,
        source_type: "node_compute_run",
        source_id: &run.compute_call_id,
        source_digest: &source_digest,
        consumer_user_id: &run.consumer_user_id,
        provider_user_id,
        units,
        amount_micros,
        provider_amount_micros,
        currency: CURRENCY_CNY,
        billing_source: &run.billing_source,
        source_status: run.settlement_status.as_deref().unwrap_or(&run.status),
        occurred_at: run.finished_at.as_deref().unwrap_or(&run.updated_at),
    })?;
    let idempotency_key = format!(
        "task-shadow-intent:v1:{}:{}:{}",
        project_id.trim(),
        matter_id.trim(),
        assignment_id.trim()
    );
    let policy_digest = digest_text(POLICY_VERSION);
    store.create_task_settlement_intent(CreateSettlementIntent {
        project_id,
        matter_id: Some(matter_id),
        assignment_id: Some(assignment_id),
        payer_user_id: &matter.requester_user_id,
        payee_user_id: provider_user_id,
        idempotency_key: &idempotency_key,
        policy_version: POLICY_VERSION,
        policy_digest: &policy_digest,
        usage_receipt_id: &usage.id,
    })?;
    Ok(usage)
}

pub(crate) fn capture_commerce_invocation(
    store: &Store,
    invocation: &OpenCommerceInvocation,
    merchant_owner_user_id: &str,
) -> Result<Option<UsageReceipt>> {
    if !project_active(store, &invocation.project_id)? {
        return Ok(None);
    }
    let source_digest = digest_json(&json!({
        "schema": "task_usage.open_commerce_invocation.v1",
        "invocation_id": invocation.id,
        "merchant_id": invocation.merchant_id,
        "capability_id": invocation.capability_id,
        "request_hash": invocation.request_hash,
        "status": invocation.status,
        "units": invocation.units,
        "unit_price_micros": invocation.unit_price_micros,
        "amount_micros": invocation.amount_micros,
        "currency": invocation.currency,
        "settlement_status": invocation.settlement_status
    }))?;
    store
        .insert_task_usage_receipt(CreateUsageReceipt {
            project_id: &invocation.project_id,
            subject_type: SUBJECT_COMMERCE_INVOCATION,
            subject_id: &invocation.id,
            source_type: "open_commerce_invocation",
            source_id: &invocation.id,
            source_digest: &source_digest,
            consumer_user_id: &invocation.requester_user_id,
            provider_user_id: Some(merchant_owner_user_id),
            units: invocation.units,
            amount_micros: invocation.amount_micros,
            provider_amount_micros: 0,
            currency: &invocation.currency,
            billing_source: "open_commerce_metered_not_charged",
            source_status: &invocation.settlement_status,
            occurred_at: invocation
                .completed_at
                .as_deref()
                .unwrap_or(&invocation.created_at),
        })
        .map(Some)
}

pub(crate) fn post_accepted_matter(
    store: &Store,
    project_id: &str,
    matter_id: &str,
) -> Result<usize> {
    if !project_active(store, project_id)? {
        return Ok(0);
    }
    post_accepted_matter_facts(store, project_id, matter_id)
}

fn post_accepted_matter_facts(store: &Store, project_id: &str, matter_id: &str) -> Result<usize> {
    let matter = store
        .get_project_ai_matter(project_id, matter_id)?
        .ok_or_else(|| anyhow!("Matter 不存在"))?;
    if matter.status != ACCEPTED_MATTER_STATUS
        || matter.final_decision.as_deref() != Some("accepted")
    {
        bail!("只有通过人工验收的 Matter 才能生成影子结算凭证");
    }

    let intents = store.list_task_settlement_intents_for_matter(project_id, matter_id)?;
    let mut posted = 0;
    for intent in intents
        .iter()
        .filter(|intent| intent.status == INTENT_PENDING)
    {
        let sources = store.task_usage_receipts_for_intent(&intent.id)?;
        if sources.is_empty() {
            continue;
        }
        let compute_amount_micros = checked_sum(
            sources.iter().map(|source| source.amount_micros),
            "计算金额",
        )?;
        let provider_amount_micros = checked_sum(
            sources.iter().map(|source| source.provider_amount_micros),
            "节点金额",
        )?;
        let platform_amount_micros = compute_amount_micros
            .checked_sub(provider_amount_micros)
            .ok_or_else(|| anyhow!("平台影子金额不能为负数"))?;
        let postings = compute_mirror_postings(
            &intent.payer_user_id,
            intent.payee_user_id.as_deref(),
            compute_amount_micros,
            provider_amount_micros,
        )?;
        let posting_key = format!("task-shadow-post:v1:{}", intent.id);
        store.post_task_shadow_settlement(
            CreateSettlementReceipt {
                project_id,
                intent_id: &intent.id,
                posting_key: &posting_key,
                status: RECEIPT_RECONCILED,
                compute_amount_micros,
                provider_amount_micros,
                platform_amount_micros,
                outcome_reward_micros: 0,
                review_reward_micros: 0,
                currency: CURRENCY_CNY,
                accepted_matter_id: Some(matter_id),
                reason: "matter accepted after review gate; mirror existing compute facts only",
            },
            &postings,
        )?;
        posted += 1;
    }
    Ok(posted)
}

pub(crate) fn void_canceled_matter(
    store: &Store,
    project_id: &str,
    matter_id: &str,
) -> Result<usize> {
    if !project_active(store, project_id)? {
        return Ok(0);
    }
    let intents = store.list_task_settlement_intents_for_matter(project_id, matter_id)?;
    let mut voided = 0;
    for intent in intents
        .iter()
        .filter(|intent| intent.status == INTENT_PENDING)
    {
        store.void_task_settlement_intent(
            project_id,
            &intent.id,
            "matter canceled before shadow settlement posting",
        )?;
        voided += 1;
    }
    Ok(voided)
}

fn project_active(store: &Store, project_id: &str) -> Result<bool> {
    Ok(runtime_enabled() && store.task_economy_project_setting(project_id)?.enabled)
}

fn fen_to_micros(fen: i64) -> Result<i64> {
    if fen < 0 {
        bail!("真实结算金额不能为负数");
    }
    fen.checked_mul(MICROS_PER_FEN)
        .ok_or_else(|| anyhow!("人民币微元换算溢出"))
}

fn checked_sum(mut values: impl Iterator<Item = i64>, label: &str) -> Result<i64> {
    values
        .try_fold(0_i64, i64::checked_add)
        .ok_or_else(|| anyhow!("{label}汇总溢出"))
}

fn digest_json(value: &serde_json::Value) -> Result<String> {
    Ok(digest_text(&serde_json::to_string(value)?))
}

fn digest_text(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        group_ai::types::{CreateMatterAssignmentRecord, CreateMatterRecord},
        store::{NodeComputeRunFinish, NodeComputeRunStart},
    };
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon-task-shadow-settlement-{}.sqlite",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("task settlement test store should open")
    }

    #[test]
    fn runtime_flag_defaults_to_off() {
        if std::env::var(RUNTIME_FLAG).is_err() {
            assert!(!runtime_enabled());
        }
    }

    #[test]
    fn fen_conversion_is_integer_only() {
        assert_eq!(fen_to_micros(123).unwrap(), 1_230_000);
    }

    #[test]
    fn accepted_matter_posts_one_balanced_shadow_receipt() {
        let store = temp_store();
        let consumer = store
            .create_user(
                "shadow-consumer@example.com",
                "secret1",
                Some("Shadow Consumer"),
                None,
            )
            .unwrap();
        let provider = store
            .create_user(
                "shadow-provider@example.com",
                "secret1",
                Some("Shadow Provider"),
                None,
            )
            .unwrap();
        let project = store
            .create_project(&consumer.id, "Shadow Settlement", None, None)
            .unwrap()
            .project;
        let channel = store
            .list_project_space_channels(&consumer.id, &project.id)
            .unwrap()
            .into_iter()
            .find(|channel| channel.kind == "ai_development")
            .unwrap();
        let matter = store
            .create_project_ai_matter(CreateMatterRecord {
                project_id: project.id.clone(),
                channel_id: channel.id,
                requester_user_id: consumer.id.clone(),
                source_message_id: None,
                title: "验证影子结算".to_string(),
                brief: "只投影真实节点成本".to_string(),
                collaboration_mode: "solo".to_string(),
                participant_user_ids: vec![consumer.id.clone(), provider.id.clone()],
                node_policy_json: json!({"mode":"project_write"}),
                acceptance_criteria: vec!["人工验收后才能过账".to_string()],
                plan_json: json!({"roles":[]}),
            })
            .unwrap();
        let assignment = store
            .create_project_ai_matter_assignment(CreateMatterAssignmentRecord {
                matter_id: matter.id.clone(),
                bot_id: "bot:codex".to_string(),
                assignee_user_id: Some(provider.id.clone()),
                provider_user_id: provider.id.clone(),
                node_id: "node-shadow".to_string(),
                role: "implementer".to_string(),
                runtime_route: "pc_node_cli".to_string(),
                cli_name: "codex".to_string(),
                worktree_path: None,
                branch_name: None,
                status: "settled".to_string(),
            })
            .unwrap();
        store
            .start_node_compute_run(NodeComputeRunStart {
                compute_call_id: "shadow:call-1",
                consumer_user_id: &consumer.id,
                provider_user_id: Some(&provider.id),
                node_id: "node-shadow",
                model_id: Some("pc-cli/codex"),
                feature: "group_ai_assignment",
                usage_mode: "pc_agent_cli",
                route_reason: Some("group_ai_assignment"),
            })
            .unwrap();
        store
            .finish_node_compute_run(
                "shadow:call-1",
                NodeComputeRunFinish {
                    provider_user_id: Some(&provider.id),
                    status: "settled",
                    prompt_tokens: 120,
                    completion_tokens: 30,
                    billed_cost_rmb_fen: 20,
                    provider_earned_fen: 16,
                    settlement_status: Some("billed"),
                    error_message: None,
                },
            )
            .unwrap();

        capture_task_assignment_facts(
            &store,
            &project.id,
            &matter.id,
            &assignment.id,
            "shadow:call-1",
        )
        .unwrap();
        assert!(post_accepted_matter_facts(&store, &project.id, &matter.id).is_err());

        store
            .update_project_ai_matter_status(
                &project.id,
                &matter.id,
                ACCEPTED_MATTER_STATUS,
                Some(&consumer.id),
                Some("accepted"),
            )
            .unwrap();
        assert_eq!(
            post_accepted_matter_facts(&store, &project.id, &matter.id).unwrap(),
            1
        );
        assert_eq!(
            post_accepted_matter_facts(&store, &project.id, &matter.id).unwrap(),
            0
        );

        let receipt = store
            .list_task_settlement_receipts(&project.id, 10)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(receipt.compute_amount_micros, 200_000);
        assert_eq!(receipt.provider_amount_micros, 160_000);
        assert_eq!(receipt.platform_amount_micros, 40_000);
        let ledger = store
            .task_ledger_transaction_for_receipt(&receipt.id)
            .unwrap()
            .unwrap();
        let debits: i64 = ledger
            .entries
            .iter()
            .filter(|entry| entry.side == "debit")
            .map(|entry| entry.amount_micros)
            .sum();
        let credits: i64 = ledger
            .entries
            .iter()
            .filter(|entry| entry.side == "credit")
            .map(|entry| entry.amount_micros)
            .sum();
        assert_eq!(debits, credits);
        let envelope = sui_projection::envelope(&receipt).unwrap();
        assert_eq!(envelope.network_submission, "not_submitted");
    }
}
