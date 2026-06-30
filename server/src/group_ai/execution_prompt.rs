use crate::group_ai::types::{ProjectAiMatter, ProjectAiMatterAssignment};

pub(crate) fn build_assignment_execution_prompt(
    matter: &ProjectAiMatter,
    assignment: &ProjectAiMatterAssignment,
    workspace_path: &str,
    runtime_permission: &str,
) -> String {
    let criteria = if matter.acceptance_criteria.is_empty() {
        "- 无额外验收标准，按 Matter brief 自行判断完成度。".to_string()
    } else {
        matter
            .acceptance_criteria
            .iter()
            .enumerate()
            .map(|(index, item)| format!("{}. {}", index + 1, item))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let branch = assignment
        .branch_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("由节点运行时创建或复用当前工作区");
    let role_guidance = role_guidance(&assignment.role);

    format!(
        r#"你正在执行「一龙」群体 AI 开发中的一个 Assignment。你是多个用户、多个 PC 节点、多种 AI 共同开发流程里的执行节点之一。

Matter:
- 标题：{title}
- 协作模式：{mode}
- 需求：{brief}

Assignment:
- 角色：{role}
- Bot：{bot_id}
- CLI：{cli}
- 节点：{node_id}
- 工作区：{workspace_path}
- 目标分支/产物登记：{branch}
- 运行权限：{runtime_permission}

验收标准:
{criteria}

角色重点:
{role_guidance}

执行要求:
1. 只围绕本 Assignment 的角色和 Matter 需求工作，避免无关重构。
2. 可以修改代码、补测试、运行必要验证；不要 push、不要部署、不要发布 APK。
3. 如果运行时已经给你隔离 worktree/branch，在其中完成改动；否则保留在当前工作区，并在结果里说明。
4. 结束时必须输出：改动摘要、关键文件、验证命令和结果、风险、需要人工合并/审核的点。
5. 如果无法执行，明确说明阻塞原因和下一步需要谁处理。
"#,
        title = matter.title,
        mode = matter.collaboration_mode,
        brief = matter.brief,
        role = assignment.role,
        bot_id = assignment.bot_id,
        cli = assignment.cli_name,
        node_id = assignment.node_id,
        workspace_path = workspace_path,
        branch = branch,
        runtime_permission = runtime_permission,
        criteria = criteria,
        role_guidance = role_guidance,
    )
}

fn role_guidance(role: &str) -> &'static str {
    let role = role.trim().to_ascii_lowercase();
    if role.contains("review") || role.contains("critic") {
        return r#"- 以独立审核为主：检查已完成 Assignment 的结果、风险、遗漏、测试证据和人工合并建议。
- 除非必须修复小问题，否则不要做大范围实现改动。
- 输出末尾必须附带一个 JSON 对象，schema 固定为 project_ai.review_result.v1，例如：
{
  "schema": "project_ai.review_result.v1",
  "status": "passed|needs_changes|blocked",
  "risk_level": "low|medium|high",
  "summary": "审核结论",
  "failed_criteria": [],
  "required_fixes": [],
  "target_assignment_id": null,
  "merge_recommendation": "manual_merge|request_changes|reject"
}"#;
    }
    "- 以实现交付为主：完成本角色负责的代码、测试、文档或诊断产物，并明确交给 reviewer 审核的证据。\n- 结果中列出关键文件、diff 摘要、测试命令和人工合并建议，便于系统登记 artifact 和 merge queue。"
}

#[cfg(test)]
mod tests {
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
            plan: json!({}),
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
    }
}
