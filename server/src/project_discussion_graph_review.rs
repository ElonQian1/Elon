//! Deterministic quality review and safe repair preparation for discussion graphs.

use anyhow::Result;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{
    project_discussion_graph::{load_graph, load_proposal},
    project_discussion_graph_model::{
        DiscussionGraph, DiscussionGraphProposal, DiscussionNode,
    },
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DiscussionReviewIssue {
    pub id: String,
    pub rule: &'static str,
    pub severity: &'static str,
    pub title: String,
    pub detail: String,
    pub node_ids: Vec<String>,
    pub suggested_action: String,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DiscussionGraphReview {
    pub graph_revision: Option<String>,
    pub health_score: u8,
    pub severity_counts: Value,
    pub issues: Vec<DiscussionReviewIssue>,
    pub safe_repair_count: usize,
    pub budget: Value,
}

pub(crate) fn review_discussion_graph(workspace: &Path) -> Result<DiscussionGraphReview> {
    let versioned = load_graph(workspace)?;
    let graph = &versioned.value;
    let mut issues = Vec::new();
    if graph.nodes.is_empty() {
        issue(
            &mut issues,
            "graph-empty",
            "graph.empty",
            "advice",
            "讨论图尚未建立",
            "当前没有可供回看或继续分叉的讨论节点。",
            &[],
            "导入一段聊天，先建立主题、来源和第一层分支。",
            false,
        );
    }
    review_sources(workspace, graph, &mut issues);
    review_nodes(workspace, graph, &mut issues);
    review_duplicates(graph, &mut issues);
    review_unresolved_decisions(graph, &mut issues);
    review_superseded_relations(graph, &mut issues);
    let errors = issues.iter().filter(|item| item.severity == "error").count();
    let warnings = issues
        .iter()
        .filter(|item| item.severity == "warning")
        .count();
    let advice = issues
        .iter()
        .filter(|item| item.severity == "advice")
        .count();
    let penalty = errors * 15 + warnings * 6 + advice * 2;
    let health_score = 100usize.saturating_sub(penalty).min(100) as u8;
    let safe_repair_count = issues.iter().filter(|item| item.auto_fixable).count();
    Ok(DiscussionGraphReview {
        graph_revision: versioned.revision,
        health_score,
        severity_counts: json!({"error":errors,"warning":warnings,"advice":advice}),
        issues,
        safe_repair_count,
        budget: metadata_budget(),
    })
}

pub(crate) fn prepare_safe_discussion_repair(workspace: &Path) -> Result<Value> {
    let review = review_discussion_graph(workspace)?;
    let graph = load_graph(workspace)?;
    let proposal = load_proposal(workspace)?;
    let roots = actual_roots(&graph.value);
    let nodes = graph
        .value
        .nodes
        .iter()
        .filter_map(|node| {
            let expected = roots.get(node.id.as_str())?;
            (node.root_id != *expected).then(|| {
                let mut repaired = node.clone();
                repaired.root_id = expected.clone();
                repaired
            })
        })
        .collect::<Vec<_>>();
    let repair_count = nodes.len();
    let repair = DiscussionGraphProposal {
        status: "ready".to_string(),
        summary: format!("确定性修正 {repair_count} 个节点的根主题引用"),
        change_kind: "repair".to_string(),
        actor: "yilong-graph-review".to_string(),
        graph: DiscussionGraph {
            nodes,
            ..Default::default()
        },
        ..Default::default()
    };
    Ok(json!({
        "status": if repair_count == 0 { "no_safe_repairs" } else { "ready" },
        "proposal": repair,
        "expected_graph_revision": graph.revision,
        "expected_suggestions_revision": proposal.revision,
        "safe_repair_count": repair_count,
        "remaining_semantic_issues": review.issues.len().saturating_sub(repair_count),
        "instruction": if repair_count == 0 {
            "没有可由程序无歧义修正的问题；语义问题必须由 AI 按来源形成 proposal。"
        } else {
            "将 proposal 原样传给 project_discussions_save_proposal，再调用 apply；每次应用都会创建新的可回看版本。"
        },
        "budget": metadata_budget(),
    }))
}

fn review_sources(
    workspace: &Path,
    graph: &DiscussionGraph,
    issues: &mut Vec<DiscussionReviewIssue>,
) {
    for source in &graph.sources {
        let path = source.reference.split('#').next().unwrap_or_default();
        if is_project_path(path) && !workspace.join(path.replace('/', "\\")).is_file() {
            issue(
                issues,
                &format!("source-missing:{}", source.id),
                "source.reference_missing",
                "warning",
                "讨论来源文件已不存在",
                &format!("来源“{}”指向的 {} 无法找到。", source.title, path),
                &[],
                "恢复来源文件，或让 AI 将引用迁移到仍存在的原始记录。",
                false,
            );
        }
    }
}

fn review_nodes(
    workspace: &Path,
    graph: &DiscussionGraph,
    issues: &mut Vec<DiscussionReviewIssue>,
) {
    let roots = actual_roots(graph);
    let child_counts = graph.nodes.iter().fold(HashMap::new(), |mut counts, node| {
        *counts.entry(node.parent_id.as_str()).or_insert(0usize) += 1;
        counts
    });
    for node in &graph.nodes {
        if roots.get(node.id.as_str()).is_some_and(|root| node.root_id != *root) {
            issue(
                issues,
                &format!("root-mismatch:{}", node.id),
                "node.root_mismatch",
                "error",
                "节点归属的根主题不正确",
                &format!("节点“{}”的 root_id 与真实父子链不一致。", node.title),
                &[node.id.clone()],
                "程序可依据父子链无歧义修正 root_id。",
                true,
            );
        }
        if node.source_refs.is_empty() {
            issue(
                issues,
                &format!("source-empty:{}", node.id),
                "node.source_missing",
                "warning",
                "节点缺少来源锚点",
                &format!("节点“{}”无法追溯到原始聊天或证据。", node.title),
                &[node.id.clone()],
                "让 AI 只读取命中的来源，为节点补充 source_refs；不要猜测来源。",
                false,
            );
        }
        if node.summary.trim().is_empty() && node.kind != "topic" {
            issue(
                issues,
                &format!("summary-empty:{}", node.id),
                "node.summary_missing",
                "advice",
                "节点缺少可复用摘要",
                &format!("节点“{}”只有标题，后续 AI 难以低 Token 理解。", node.title),
                &[node.id.clone()],
                "补充一段只陈述该节点含义、边界和当前状态的短摘要。",
                false,
            );
        }
        if is_confirmed_business_node(node)
            && !matches!(node.authority.as_str(), "accepted" | "current" | "evidence")
        {
            issue(
                issues,
                &format!("authority-mismatch:{}", node.id),
                "node.authority_mismatch",
                "error",
                "确认状态与权威性冲突",
                &format!(
                    "节点“{}”状态为 {}，但 authority 仍是 {}。",
                    node.title, node.status, node.authority
                ),
                &[node.id.clone()],
                "核对来源：若确已确认则升级 authority；否则把状态退回讨论中。",
                false,
            );
        }
        for path in &node.document_paths {
            if !workspace.join(path.replace('/', "\\")).is_file() {
                issue(
                    issues,
                    &format!("document-missing:{}:{path}", node.id),
                    "node.document_missing",
                    if node.status == "implemented" {
                        "error"
                    } else {
                        "warning"
                    },
                    "节点关联文档已不存在",
                    &format!("节点“{}”关联的 {} 无法找到。", node.title, path),
                    &[node.id.clone()],
                    "查找重命名后的文档并更新路径，或明确移除失效关联。",
                    false,
                );
            }
        }
        if node.status == "implemented"
            && matches!(node.kind.as_str(), "feature" | "task" | "result")
            && node.document_paths.is_empty()
            && node.feature_node_ids.is_empty()
        {
            issue(
                issues,
                &format!("implementation-unlinked:{}", node.id),
                "node.implementation_unlinked",
                "warning",
                "已实现节点没有实现证据",
                &format!("节点“{}”未关联文档或功能节点。", node.title),
                &[node.id.clone()],
                "关联功能图节点、实现文档或可验证结果，避免把计划误写成已实现。",
                false,
            );
        }
        if node.parent_id.is_empty()
            && child_counts.get(node.id.as_str()).copied().unwrap_or(0) == 0
            && !graph
                .edges
                .iter()
                .any(|edge| edge.source == node.id || edge.target == node.id)
        {
            issue(
                issues,
                &format!("isolated-root:{}", node.id),
                "node.isolated_root",
                "advice",
                "主题尚未拆出讨论分支",
                &format!("根主题“{}”没有子节点或关系。", node.title),
                &[node.id.clone()],
                "继续讨论时至少拆分问题、方案、风险、决策或功能节点。",
                false,
            );
        }
    }
}

fn review_duplicates(graph: &DiscussionGraph, issues: &mut Vec<DiscussionReviewIssue>) {
    let mut groups = HashMap::<(String, String), Vec<&DiscussionNode>>::new();
    for node in &graph.nodes {
        groups
            .entry((node.root_id.clone(), normalized_title(&node.title)))
            .or_default()
            .push(node);
    }
    for nodes in groups.values().filter(|nodes| nodes.len() > 1) {
        issue(
            issues,
            &format!(
                "duplicate-title:{}",
                nodes.iter().map(|node| node.id.as_str()).collect::<Vec<_>>().join(",")
            ),
            "node.duplicate_title",
            "warning",
            "同一主题下存在近似重复节点",
            &format!(
                "{} 个节点使用标题“{}”。",
                nodes.len(),
                nodes[0].title
            ),
            &nodes.iter().map(|node| node.id.clone()).collect::<Vec<_>>(),
            "由 AI 比较摘要和来源：相同观点用 merged_into，相互演化用 supersedes。",
            false,
        );
    }
}

fn review_unresolved_decisions(
    graph: &DiscussionGraph,
    issues: &mut Vec<DiscussionReviewIssue>,
) {
    for decision in graph
        .nodes
        .iter()
        .filter(|node| node.kind == "decision" && node.status == "accepted")
    {
        let unresolved = graph.nodes.iter().filter(|candidate| {
            matches!(candidate.kind.as_str(), "risk" | "objection")
                && matches!(candidate.status.as_str(), "open" | "exploring")
                && (candidate.parent_id == decision.id
                    || graph.edges.iter().any(|edge| {
                        edge.relation == "opposes"
                            && edge.source == candidate.id
                            && edge.target == decision.id
                    }))
        });
        let ids = unresolved.map(|node| node.id.clone()).collect::<Vec<_>>();
        if !ids.is_empty() {
            issue(
                issues,
                &format!("decision-unresolved:{}", decision.id),
                "decision.unresolved_objection",
                "warning",
                "已采纳决策仍有未处理的风险或反对意见",
                &format!("决策“{}”仍连接 {} 个开放问题。", decision.title, ids.len()),
                &ids,
                "补充接受风险、缓解措施或否决理由，再更新相关节点状态。",
                false,
            );
        }
    }
}

fn review_superseded_relations(
    graph: &DiscussionGraph,
    issues: &mut Vec<DiscussionReviewIssue>,
) {
    for node in graph.nodes.iter().filter(|node| node.status == "superseded") {
        let linked = graph.edges.iter().any(|edge| {
            matches!(edge.relation.as_str(), "supersedes" | "merged_into")
                && (edge.source == node.id || edge.target == node.id)
        });
        if !linked {
            issue(
                issues,
                &format!("superseded-unlinked:{}", node.id),
                "node.superseded_unlinked",
                "advice",
                "已替代节点没有指向后继",
                &format!("节点“{}”标记为已替代，但看不到由什么替代。", node.title),
                &[node.id.clone()],
                "增加 supersedes 或 merged_into 关系，保留演化链。",
                false,
            );
        }
    }
}

fn actual_roots(graph: &DiscussionGraph) -> HashMap<&str, String> {
    let parents = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.parent_id.as_str()))
        .collect::<HashMap<_, _>>();
    graph
        .nodes
        .iter()
        .map(|node| {
            let mut root = node.id.as_str();
            let mut seen = HashSet::new();
            while seen.insert(root) {
                let parent = parents.get(root).copied().unwrap_or_default();
                if parent.is_empty() {
                    break;
                }
                root = parent;
            }
            (node.id.as_str(), root.to_string())
        })
        .collect()
}

fn is_confirmed_business_node(node: &DiscussionNode) -> bool {
    matches!(node.status.as_str(), "accepted" | "implemented")
        && matches!(
            node.kind.as_str(),
            "decision" | "requirement" | "feature" | "task" | "result"
        )
}

fn is_project_path(reference: &str) -> bool {
    let value = reference.trim().replace('\\', "/").to_ascii_lowercase();
    !value.contains("://")
        && !value.starts_with("codex:")
        && (value.starts_with("docs/")
            || value.starts_with(".github/")
            || value.ends_with(".md")
            || value.ends_with(".json"))
}

fn normalized_title(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn issue(
    issues: &mut Vec<DiscussionReviewIssue>,
    id: &str,
    rule: &'static str,
    severity: &'static str,
    title: &str,
    detail: &str,
    node_ids: &[String],
    suggested_action: &str,
    auto_fixable: bool,
) {
    issues.push(DiscussionReviewIssue {
        id: id.to_string(),
        rule,
        severity,
        title: title.to_string(),
        detail: detail.to_string(),
        node_ids: node_ids.to_vec(),
        suggested_action: suggested_action.to_string(),
        auto_fixable,
    });
}

fn metadata_budget() -> Value {
    json!({
        "classification_model_tokens": 0,
        "chat_bodies_read": 0,
        "document_bodies_read": 0,
        "metadata_only": true,
    })
}

#[cfg(test)]
#[path = "project_discussion_graph_review_tests.rs"]
mod tests;
