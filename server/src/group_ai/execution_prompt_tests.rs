use super::*;
use crate::group_ai::types::{ProjectAiMatter, ProjectAiMatterAssignment};
use serde_json::json;

#[test]
fn prompt_contains_assignment_scope_and_human_merge_rules() {
    let matter = ProjectAiMatter {
        id: "m1".to_string(),
        project_id: "p1".to_string(),
        channel_id: "c1".to_string(),
        requester_user_id: "u1".to_string(),
        decision_user_id: None,
        source_message_id: None,
        title: "多 AI 协作".to_string(),
        brief: "实现 Assignment 执行闭环".to_string(),
        collaboration_mode: "split".to_string(),
        status: "running".to_string(),
        participant_user_ids: vec!["u1".to_string()],
        node_policy: json!({}),
        acceptance_criteria: vec!["记录 compute_call_id".to_string()],
        plan: json!({"execution_contract":{"schema":"test.contract.v1","required_artifact":{"artifact_kind":"test_evidence"}}}),
        final_summary: None,
        final_decision: None,
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    let assignment = ProjectAiMatterAssignment {
        id: "a1".to_string(),
        matter_id: "m1".to_string(),
        bot_id: "bot:codex".to_string(),
        assignee_user_id: Some("u2".to_string()),
        provider_user_id: "u2".to_string(),
        node_id: "node-a".to_string(),
        role: "executor".to_string(),
        runtime_route: "pc_node_cli".to_string(),
        cli_name: "codex".to_string(),
        worktree_path: None,
        branch_name: Some("group-ai/m1".to_string()),
        status: "planned".to_string(),
        result_summary: None,
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };

    let prompt =
        build_assignment_execution_prompt(&matter, &assignment, "D:/repo", "project_write");

    assert!(prompt.contains("实现 Assignment 执行闭环"));
    assert!(prompt.contains("group-ai/m1"));
    assert!(prompt.contains("不要 push"));
    assert!(prompt.contains("验证命令和结果"));
    assert!(prompt.contains("test.contract.v1"));
    assert!(prompt.contains("不得伪造测试、部署、支付或链上结果"));
}
