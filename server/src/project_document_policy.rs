//! Deterministic project-document authority classification.
//!
//! Path rules are the authority ceiling. Frontmatter may narrow lifecycle or
//! scope, but a note under drafts/archive can never promote itself to current.

use homecli_proto::ProjectDocumentMetadata;
use sha2::{Digest, Sha256};

/// Increment whenever deterministic path/frontmatter classification semantics
/// change. Persistent catalog entries are derived data and must be rebuilt
/// even when the Markdown file size and mtime are unchanged.
pub(crate) const CLASSIFIER_VERSION: &str = "4";

pub(crate) fn classify_project_document(
    path: &str,
    content: &str,
    full_char_count: usize,
) -> ProjectDocumentMetadata {
    let normalized = path.replace('\\', "/");
    let lower = normalized.to_lowercase();
    let file_name = lower.rsplit('/').next().unwrap_or(lower.as_str());
    let mut metadata = path_defaults(&lower, file_name);
    let frontmatter = frontmatter(content);

    if let Some(scope) = frontmatter_value(&frontmatter, "scope") {
        metadata.scope = scope;
    }
    if let Some(lifecycle) = frontmatter_value(&frontmatter, "lifecycle")
        .or_else(|| frontmatter_value(&frontmatter, "status"))
        .or_else(|| frontmatter_value(&frontmatter, "version_status"))
    {
        apply_lifecycle_narrowing(&mut metadata, &lifecycle);
    }

    metadata.token_estimate = full_char_count.div_ceil(4) as u64;
    metadata.content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    metadata.headings = markdown_headings(content, 16);
    metadata
}

fn path_defaults(path: &str, file_name: &str) -> ProjectDocumentMetadata {
    let (role, lifecycle, authority, default_retrieval, ambiguous, confidence, reason) =
        if path == ".github/copilot-instructions.md" {
            (
                "policy",
                "active",
                "repository_policy",
                true,
                false,
                "high",
                "仓库共享权威规则",
            )
        } else if path == "agents.md" {
            (
                "router",
                "active",
                "repository_routing",
                true,
                false,
                "high",
                "跨供应商 AI 工作入口",
            )
        } else if path.starts_with(".github/instructions/") {
            (
                "instruction",
                "active",
                "domain_policy",
                false,
                false,
                "high",
                "按任务加载的领域规则",
            )
        } else if path.starts_with(".github/agents/") {
            (
                "agent_definition",
                "active",
                "customization",
                false,
                false,
                "high",
                "Agent 定义仅在对应任务中按需加载",
            )
        } else if path.starts_with(".github/prompts/") {
            (
                "prompt_template",
                "active",
                "customization",
                false,
                false,
                "high",
                "Prompt 模板不属于项目事实或当前规范",
            )
        } else if path.starts_with(".github/skills/") {
            (
                "skill",
                "active",
                "customization",
                false,
                false,
                "high",
                "Skill 只在能力命中时按需加载",
            )
        } else if matches!(path, "codex.md" | "claude.md" | "gemini.md") {
            (
                "provider_adapter",
                "active",
                "provider_routing",
                false,
                false,
                "high",
                "供应商桥接文档",
            )
        } else if path == "ai_current.md" {
            (
                "current_status",
                "active",
                "project_status",
                true,
                false,
                "high",
                "项目当前事实入口",
            )
        } else if matches!(
            path,
            "ai_project.md"
                | "ai_architecture.md"
                | "ai_index.md"
                | "ai_rules.md"
                | "ai_task_template.md"
        ) {
            (
                "project_guide",
                "active",
                "project_guidance",
                false,
                false,
                "high",
                "项目 AI 导航文档",
            )
        } else if path.starts_with("default-project-docs/")
            || path.starts_with("group-chat-project-docs/")
        {
            (
                "project_template",
                "active",
                "customization",
                false,
                false,
                "high",
                "新项目或群聊项目的初始化模板，不代表当前仓库事实",
            )
        } else if is_historical_path(path) {
            (
                "archive",
                "archived",
                "historical",
                false,
                false,
                "high",
                "历史目录默认不参与当前实现检索",
            )
        } else if path.starts_with("docs/current/specs/") {
            (
                "spec",
                "active",
                "normative",
                false,
                false,
                "high",
                "当前规范目录",
            )
        } else if path.starts_with("docs/current/architecture/") {
            (
                "architecture",
                "active",
                "normative",
                false,
                false,
                "high",
                "当前架构目录",
            )
        } else if path.starts_with("docs/current/requirements/") {
            (
                "requirement",
                "active",
                "approved",
                false,
                false,
                "high",
                "已批准需求目录",
            )
        } else if path.starts_with("docs/current/runbooks/") {
            (
                "runbook",
                "active",
                "operational",
                false,
                false,
                "high",
                "当前操作手册目录",
            )
        } else if path.starts_with("docs/decisions/") {
            (
                "decision",
                "accepted",
                "decision_record",
                false,
                false,
                "high",
                "架构决策记录",
            )
        } else if path.starts_with("docs/evidence/status/") {
            (
                "status",
                "active",
                "evidence",
                false,
                false,
                "high",
                "状态证据不能覆盖规范",
            )
        } else if path.starts_with("docs/evidence/reports/") || path.starts_with("docs/reports/") {
            (
                "report",
                "active",
                "evidence",
                false,
                false,
                "high",
                "报告证据不能定义需求",
            )
        } else if path.starts_with("docs/drafts/requirements/") {
            (
                "requirement",
                "draft",
                "proposal",
                false,
                false,
                "high",
                "未批准需求",
            )
        } else if path.starts_with("docs/drafts/") || path.starts_with("docs/discussions/") {
            (
                "discussion",
                "draft",
                "proposal",
                false,
                false,
                "high",
                "讨论和草稿默认不参与实现检索",
            )
        } else if path.starts_with("docs/inbox/conversations/") {
            (
                "discussion",
                "source_material",
                "none",
                false,
                false,
                "high",
                "导入聊天只作为讨论来源，不参与当前事实检索",
            )
        } else if path.starts_with("docs/inbox/") {
            (
                "note",
                "unclassified",
                "unknown",
                false,
                true,
                "high",
                "Inbox 笔记尚未确认权威性和长期分区",
            )
        } else if looks_like_discussion(file_name) {
            (
                "discussion",
                "draft",
                "proposal",
                false,
                false,
                "medium",
                "文件名表明它是讨论或观点",
            )
        } else if looks_like_report(file_name) {
            (
                "report",
                "active",
                "evidence",
                false,
                false,
                "medium",
                "文件名表明它是报告或测试证据",
            )
        } else if path == "readme.md" || path.starts_with("docs/") && file_name == "readme.md" {
            (
                "guide",
                "active",
                "informative",
                false,
                false,
                "medium",
                "说明文档不自动定义现行规则",
            )
        } else {
            (
                "note",
                "unclassified",
                "unknown",
                false,
                true,
                "low",
                "路径无法确定权威性，需要整理",
            )
        };

    ProjectDocumentMetadata {
        role: role.to_string(),
        lifecycle: lifecycle.to_string(),
        authority: authority.to_string(),
        scope: infer_scope(path),
        default_retrieval,
        ambiguous,
        confidence: confidence.to_string(),
        reason: reason.to_string(),
        ..ProjectDocumentMetadata::default()
    }
}

fn apply_lifecycle_narrowing(metadata: &mut ProjectDocumentMetadata, value: &str) {
    let normalized = value.trim().to_lowercase().replace('-', "_");
    if matches!(
        normalized.as_str(),
        "draft" | "deprecated" | "superseded" | "archived"
    ) {
        metadata.lifecycle = normalized;
        metadata.default_retrieval = false;
        metadata.reason = format!("{}；frontmatter 将生命周期收窄", metadata.reason);
    }
}

fn frontmatter(content: &str) -> Vec<(String, String)> {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Vec::new();
    }
    lines
        .take(40)
        .take_while(|line| line.trim() != "---")
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            let key = key.trim().to_lowercase();
            let value = value.trim().trim_matches(['"', '\'']).to_string();
            (!key.is_empty() && !value.is_empty()).then_some((key, value))
        })
        .collect()
}

fn frontmatter_value(values: &[(String, String)], key: &str) -> Option<String> {
    values
        .iter()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, value)| value.clone())
}

fn markdown_headings(content: &str, limit: usize) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let title = trimmed.trim_start_matches('#');
            (title.len() < trimmed.len() && title.starts_with(' '))
                .then(|| title.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .take(limit)
        .collect()
}

fn infer_scope(path: &str) -> String {
    let parts = path.split('/').collect::<Vec<_>>();
    if path.starts_with(".github/instructions/") {
        return parts
            .last()
            .unwrap_or(&"repository")
            .split('.')
            .next()
            .unwrap_or("repository")
            .to_string();
    }
    if path.starts_with("docs/current/") && parts.len() > 3 {
        return parts[3].split('.').next().unwrap_or("project").to_string();
    }
    "project".to_string()
}

fn looks_like_discussion(file_name: &str) -> bool {
    [
        "讨论",
        "观点",
        "建议",
        "问题",
        "想法",
        "愿景",
        "proposal",
        "discussion",
    ]
    .iter()
    .any(|marker| file_name.contains(marker))
}

fn is_historical_path(path: &str) -> bool {
    let padded = format!("/{}/", path.trim_matches('/'));
    [
        "archive",
        "archives",
        "archived",
        "historical",
        "history",
        "legacy",
    ]
    .iter()
    .any(|directory| padded.contains(&format!("/{directory}/")))
}

fn looks_like_report(file_name: &str) -> bool {
    ["report", "报告", "验收", "e2e", "test-result", "测试结果"]
        .iter()
        .any(|marker| file_name.contains(marker))
}

#[cfg(test)]
#[path = "project_document_policy_tests.rs"]
mod tests;
