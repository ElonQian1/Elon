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
