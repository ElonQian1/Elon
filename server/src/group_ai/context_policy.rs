use serde::Serialize;
use serde_json::{json, Value};

use crate::group_ai::types::{ProjectAiBot, ProjectAiMatter, ProjectAiMatterAssignment};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RoleContextPolicy {
    pub owned_paths: Vec<String>,
    pub context_sources: Vec<String>,
    pub verification_commands: Vec<String>,
    pub handoff_contract: String,
}

pub(crate) fn plan_ownership(
    collaboration_mode: &str,
    brief: &str,
    bots: &[&ProjectAiBot],
) -> Value {
    let domains = inferred_domains(brief);
    let roles = bots
        .iter()
        .enumerate()
        .map(|(index, bot)| {
            let owned_paths = owned_paths_for(index, &domains);
            json!({
                "bot_id": bot.bot_id,
                "role_index": index,
                "owned_paths": owned_paths,
                "conflict_policy": "只修改 owned_paths 覆盖的模块；需要跨区修改时先在结果中声明，不直接扩大范围。",
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "project_ai.ownership.v1",
        "collaboration_mode": collaboration_mode,
        "domains": domains,
        "roles": roles,
    })
}

pub(crate) fn verification_commands(brief: &str) -> Vec<String> {
    let lower = brief.to_ascii_lowercase();
    let mut commands = Vec::new();
    if lower.contains("pc") || lower.contains("frontend") || brief.contains("前端") {
        commands.push("cd pc-frontend && npm run build".to_string());
    }
    if lower.contains("server") || lower.contains("api") || brief.contains("后端") {
        commands
            .push("cargo check --manifest-path server/Cargo.toml --bin elon-server".to_string());
    }
    if commands.is_empty() {
        commands.push(
            "cargo test --manifest-path server/Cargo.toml group_ai --bin elon-server".to_string(),
        );
    }
    commands.truncate(3);
    commands
}

pub(crate) fn assignment_context_policy(
    matter: &ProjectAiMatter,
    assignment: &ProjectAiMatterAssignment,
) -> RoleContextPolicy {
    let ownership = matter
        .plan
        .get("ownership")
        .and_then(|value| value.get("roles"))
        .and_then(Value::as_array)
        .and_then(|roles| {
            roles.iter().find(|role| {
                role.get("bot_id").and_then(Value::as_str) == Some(assignment.bot_id.as_str())
            })
        });
    let owned_paths = ownership
        .and_then(|role| role.get("owned_paths"))
        .and_then(Value::as_array)
        .map(|values| string_array(values))
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| owned_paths_for(0, &inferred_domains(&matter.brief)));
    let verification_commands = matter
        .plan
        .get("verification_commands")
        .and_then(Value::as_array)
        .map(|values| string_array(values))
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| verification_commands(&matter.brief));
    let context_sources = if is_review_role(&assignment.role) {
        vec![
            "Matter brief 和 acceptance_criteria".to_string(),
            "实现 Assignment 的 artifact、diff_stat、test_results".to_string(),
            "project_ai.review_result.v1 结构化输出".to_string(),
        ]
    } else {
        vec![
            "Matter brief 和 acceptance_criteria".to_string(),
            "owned_paths 对应的 repo map / symbol task pack".to_string(),
            "项目文档、AGENTS.md、模块化规则".to_string(),
        ]
    };
    RoleContextPolicy {
        owned_paths,
        context_sources,
        verification_commands,
        handoff_contract: if is_review_role(&assignment.role) {
            "输出必须绑定 target_assignment_id、failed_criteria、required_fixes 和 merge_recommendation。".to_string()
        } else {
            "输出必须列出关键文件、验证命令、diff 摘要和人工合并建议。".to_string()
        },
    }
}

fn inferred_domains(brief: &str) -> Vec<String> {
    let lower = brief.to_ascii_lowercase();
    let mut domains = Vec::new();
    if lower.contains("frontend")
        || lower.contains("pc")
        || brief.contains("前端")
        || brief.contains("页面")
    {
        domains.push("pc_frontend".to_string());
    }
    if lower.contains("api")
        || lower.contains("server")
        || brief.contains("后端")
        || brief.contains("接口")
    {
        domains.push("server_api".to_string());
    }
    if lower.contains("db")
        || lower.contains("sqlite")
        || brief.contains("数据库")
        || brief.contains("持久化")
    {
        domains.push("storage".to_string());
    }
    if lower.contains("test") || brief.contains("测试") || brief.contains("验证") {
        domains.push("verification".to_string());
    }
    if domains.is_empty() {
        domains.push("project_scope".to_string());
    }
    domains
}

fn owned_paths_for(index: usize, domains: &[String]) -> Vec<String> {
    let domain = domains
        .get(index % domains.len().max(1))
        .map(String::as_str)
        .unwrap_or("project_scope");
    match domain {
        "pc_frontend" => vec!["pc-frontend/src/features/**".to_string()],
        "server_api" => vec![
            "server/src/group_ai/**".to_string(),
            "server/src/router.rs".to_string(),
        ],
        "storage" => vec![
            "server/src/store/**".to_string(),
            "server/src/store_migrations.rs".to_string(),
        ],
        "verification" => vec![
            "server/src/**".to_string(),
            "pc-frontend/src/**".to_string(),
        ],
        _ => vec!["按 Matter brief 和现有模块边界选择最小必要文件".to_string()],
    }
}

fn string_array(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .take(8)
        .map(ToOwned::to_owned)
        .collect()
}

fn is_review_role(role: &str) -> bool {
    let role = role.trim().to_ascii_lowercase();
    role.contains("review") || role.contains("critic")
}
