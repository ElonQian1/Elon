use crate::group_ai::{
    context_policy::assignment_context_policy,
    types::{ProjectAiMatter, ProjectAiMatterAssignment},
};

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
    let context_policy = assignment_context_policy(matter, assignment);
    let context_policy_json =
        serde_json::to_string_pretty(&context_policy).unwrap_or_else(|_| "{}".to_string());
    let execution_contract = matter
        .plan
        .get("execution_contract")
        .map(|value| serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string()))
        .unwrap_or_else(|| "无专用执行合同，按 Matter brief 和验收标准执行。".to_string());

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

上下文与文件所有权:
{context_policy_json}

机器可读执行合同:
{execution_contract}

执行要求:
1. 只围绕本 Assignment 的角色和 Matter 需求工作，避免无关重构。
2. 只修改 owned_paths 覆盖的模块；确需跨区修改时在结果中声明原因，不要直接扩大范围。
3. 可以修改代码、补测试、运行必要验证；不要 push、不要部署、不要发布 APK。
4. 如果运行时已经给你隔离 worktree/branch，在其中完成改动；否则保留在当前工作区，并在结果里说明。
5. 结束时必须输出：改动摘要、关键文件、验证命令和结果、风险、需要人工合并/审核的点。
6. 如果无法执行，明确说明阻塞原因和下一步需要谁处理。
7. 执行合同声明 required_artifact 时，必须按其 artifact_kind 和 evidence_schema 登记证据；不得伪造测试、部署、支付或链上结果。
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
        execution_contract = execution_contract,
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
#[path = "execution_prompt_tests.rs"]
mod tests;
