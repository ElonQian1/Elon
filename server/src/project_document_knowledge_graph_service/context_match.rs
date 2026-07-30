use std::collections::HashSet;

use serde_json::{json, Value};

use crate::{
    project_document_governance::DocumentSectionManifest,
    project_document_governance_facets::DocumentGovernanceFacets,
};

use super::normalize;

pub(super) fn is_historical_noise(document: &homecli_proto::ProjectDocumentEntry) -> bool {
    let path = normalize(&document.path);
    matches!(
        document.metadata.lifecycle.as_str(),
        "deprecated" | "superseded" | "archived"
    ) || matches!(
        document.metadata.role.as_str(),
        "report" | "discussion" | "status" | "archive"
    ) || ["/reports/", "e2e", "trace", "archive", "history"]
        .iter()
        .any(|part| path.contains(part))
}

pub(super) fn is_task_specific_customization(
    document: &homecli_proto::ProjectDocumentEntry,
) -> bool {
    let path = normalize(&document.path);
    matches!(
        document.metadata.role.as_str(),
        "instruction"
            | "agent_definition"
            | "prompt_template"
            | "skill"
            | "provider_adapter"
            | "project_template"
    ) || matches!(path.as_str(), "ai_rules.md" | "ai_task_template.md")
}

pub(super) fn manifest_entrypoint_score(
    path: &str,
    query_terms: &[String],
    manifest: &DocumentSectionManifest,
) -> usize {
    let home_text = format!("{} {}", manifest.home.title, manifest.home.summary).to_lowercase();
    let home_matches = query_terms
        .iter()
        .filter(|term| home_text.contains(term.as_str()))
        .count();
    if home_matches > 0 && normalize(&manifest.home.entrypoint) == path {
        return 60 + home_matches * 30;
    }
    if home_matches > 0
        && manifest
            .home
            .start_here
            .iter()
            .any(|candidate| normalize(candidate) == path)
    {
        return 40 + home_matches * 20;
    }
    manifest
        .sections
        .iter()
        .filter(|section| normalize(&section.entrypoint) == path)
        .map(|section| {
            let text = format!("{} {}", section.label, section.detail).to_lowercase();
            let matches = query_terms
                .iter()
                .filter(|term| text.contains(term.as_str()))
                .count();
            if matches == 0 {
                0
            } else {
                60 + matches * 30
            }
        })
        .max()
        .unwrap_or_default()
}

pub(super) fn context_reason(
    score: usize,
    explicitly_requested: bool,
    linked: bool,
    knowledge_entrypoint: bool,
    authoritative: bool,
) -> Value {
    json!({
        "explicit_document_match":explicitly_requested,
        "graph_linked":linked,
        "knowledge_entrypoint":knowledge_entrypoint,
        "authoritative_entrypoint":knowledge_entrypoint && authoritative,
        "ranking_score":score,
    })
}

pub(super) fn governance_intent_score(
    query: &str,
    term_score: usize,
    facets: &DocumentGovernanceFacets,
) -> usize {
    let current_status_requested = [
        "当前",
        "现在",
        "现状",
        "已经实现",
        "已实现",
        "正在建设",
        "当前事实",
        "current status",
        "implemented",
    ]
    .iter()
    .any(|term| query.contains(term));
    if current_status_requested && facets.document_type == "current_status" {
        return 2_500 + term_score;
    }

    let direct_decision_requested = ["是否采用", "是否使用", "主架构", "不采用"]
        .iter()
        .any(|term| query.contains(term));
    let decision_requested = direct_decision_requested
        || ["否决", "拒绝", "弃用", "决定", "decision", "reject"]
            .iter()
            .any(|term| query.contains(term));
    if decision_requested
        && matches!(
            facets.document_type.as_str(),
            "decision" | "architecture_decision"
        )
    {
        let intent_score = if direct_decision_requested {
            4_000
        } else {
            1_600
        };
        return intent_score + term_score.saturating_mul(4);
    }

    0
}

pub(super) fn explicit_document_matches(
    documents: &[homecli_proto::ProjectDocumentEntry],
    query: &str,
) -> HashSet<String> {
    let normalized_query = normalize(query);
    documents
        .iter()
        .filter_map(|document| {
            let path = normalize(&document.path);
            let file_name = path.rsplit('/').next().unwrap_or(path.as_str());
            let title = document.title.trim().to_lowercase();
            let path_match = normalized_query.contains(&path);
            let file_match = file_name.chars().count() >= 5 && normalized_query.contains(file_name);
            let title_match = title.chars().count() >= 4 && query.contains(&title);
            (path_match || file_match || title_match).then_some(path)
        })
        .collect()
}

pub(super) fn context_query_terms(query: &str) -> Vec<String> {
    let mut terms = query
        .split(|character: char| {
            character.is_whitespace()
                || matches!(character, ',' | '/' | ':' | '，' | '、' | '；' | ';')
        })
        .flat_map(split_ascii_cjk_boundaries)
        .filter(|term| term.chars().count() >= 2)
        .map(str::to_string)
        .collect::<Vec<_>>();
    for keyword in [
        "当前",
        "现状",
        "已实现",
        "正在建设",
        "否决",
        "拒绝",
        "文档",
        "知识",
        "治理",
        "架构",
        "系统",
        "监督",
        "项目",
        "权限",
        "发布",
        "节点",
        "检索",
        "健康",
        "能力",
        "商户",
        "消费者",
        "开放商业",
        "融合",
        "共享",
        "算力",
        "结算",
        "调用",
        "提案",
        "讨论",
        "来源",
        "sui",
        "mcp",
        "token",
        "android",
        "pc",
    ] {
        if query.contains(keyword) && !terms.iter().any(|term| term == keyword) {
            terms.push(keyword.to_string());
        }
    }
    terms.sort();
    terms.dedup();
    terms
}

fn split_ascii_cjk_boundaries(value: &str) -> Vec<&str> {
    let mut boundaries = vec![0usize];
    let mut previous_ascii = None;
    for (index, character) in value.char_indices() {
        let ascii = character.is_ascii_alphanumeric();
        if previous_ascii.is_some_and(|previous| previous != ascii) {
            boundaries.push(index);
        }
        previous_ascii = Some(ascii);
    }
    boundaries.push(value.len());
    boundaries
        .windows(2)
        .filter_map(|range| value.get(range[0]..range[1]))
        .filter(|part| !part.is_empty())
        .collect()
}
