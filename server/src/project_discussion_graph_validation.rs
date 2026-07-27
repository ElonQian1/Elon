//! Validation and merge rules for portable discussion graphs.

use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{
    project_discussion_graph_model::{
        version, DiscussionEdge, DiscussionGraph, DiscussionGraphProposal, DiscussionNode,
        DiscussionPromotion, DiscussionSource,
    },
    project_document_file_operation_model::normalize_document_path,
    project_document_files::{content_revision, read_project_document_file},
};

const MAX_SOURCES: usize = 512;
const MAX_NODES: usize = 4_096;
const MAX_EDGES: usize = 8_192;
const MAX_PROMOTIONS: usize = 256;

pub(crate) fn normalize_graph(mut graph: DiscussionGraph) -> Result<DiscussionGraph> {
    if graph.sources.len() > MAX_SOURCES
        || graph.nodes.len() > MAX_NODES
        || graph.edges.len() > MAX_EDGES
    {
        bail!("讨论图超过安全上限");
    }
    graph.version = version();
    graph.sources = graph
        .sources
        .into_iter()
        .map(normalize_source)
        .collect::<Result<Vec<_>>>()?;
    graph.nodes = graph
        .nodes
        .into_iter()
        .map(normalize_node)
        .collect::<Result<Vec<_>>>()?;
    let source_ids = unique_ids(graph.sources.iter().map(|item| item.id.as_str()), "来源")?;
    let node_ids = unique_ids(graph.nodes.iter().map(|item| item.id.as_str()), "节点")?;
    validate_nodes(&graph.nodes, &node_ids, &source_ids)?;
    graph.edges = graph
        .edges
        .into_iter()
        .map(|edge| normalize_edge(edge, &node_ids))
        .collect::<Result<Vec<_>>>()?;
    unique_ids(graph.edges.iter().map(|item| item.id.as_str()), "关系")?;
    Ok(graph)
}

pub(crate) fn normalize_proposal(
    mut proposal: DiscussionGraphProposal,
) -> Result<DiscussionGraphProposal> {
    proposal.version = version();
    proposal.status = match proposal.status.trim().to_ascii_lowercase().as_str() {
        "" | "ready" => "ready".to_string(),
        "applied" => "applied".to_string(),
        _ => bail!("讨论图建议状态只支持 ready 或 applied"),
    };
    proposal.summary = truncate(proposal.summary.trim(), 1_000);
    proposal.graph = normalize_graph(proposal.graph)?;
    if proposal.promotions.len() > MAX_PROMOTIONS {
        bail!("一次最多晋升 {MAX_PROMOTIONS} 份文档");
    }
    let node_ids = proposal
        .graph
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    proposal.promotions = proposal
        .promotions
        .into_iter()
        .map(|item| normalize_promotion(item, &node_ids))
        .collect::<Result<Vec<_>>>()?;
    unique_ids(
        proposal.promotions.iter().map(|item| item.id.as_str()),
        "晋升操作",
    )?;
    Ok(proposal)
}

pub(crate) fn merge_graph(
    current: DiscussionGraph,
    proposed: DiscussionGraph,
) -> Result<DiscussionGraph> {
    let mut sources = current
        .sources
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect::<HashMap<_, _>>();
    sources.extend(
        proposed
            .sources
            .into_iter()
            .map(|item| (item.id.clone(), item)),
    );
    let mut nodes = current
        .nodes
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect::<HashMap<_, _>>();
    nodes.extend(
        proposed
            .nodes
            .into_iter()
            .map(|item| (item.id.clone(), item)),
    );
    let mut edges = current
        .edges
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect::<HashMap<_, _>>();
    edges.extend(
        proposed
            .edges
            .into_iter()
            .map(|item| (item.id.clone(), item)),
    );
    normalize_graph(DiscussionGraph {
        version: version(),
        sources: sources.into_values().collect(),
        nodes: nodes.into_values().collect(),
        edges: edges.into_values().collect(),
    })
}

pub(crate) fn validate_promotions(
    workspace: &Path,
    proposal: &DiscussionGraphProposal,
) -> Result<()> {
    for promotion in &proposal.promotions {
        if let Ok(existing) = read_project_document_file(workspace, &promotion.path) {
            if content_revision(&promotion.content) != existing.revision {
                bail!("晋升目标已存在且内容不同，禁止覆盖：{}", promotion.path);
            }
        }
    }
    Ok(())
}

pub(crate) fn counts(graph: &DiscussionGraph, promotions: usize) -> Value {
    json!({
        "sources": graph.sources.len(),
        "nodes": graph.nodes.len(),
        "edges": graph.edges.len(),
        "roots": graph.nodes.iter().filter(|node| node.parent_id.is_empty()).count(),
        "open": graph.nodes.iter().filter(|node| matches!(node.status.as_str(), "open" | "exploring")).count(),
        "accepted": graph.nodes.iter().filter(|node| node.status == "accepted").count(),
        "promotions": promotions,
    })
}

fn normalize_source(mut source: DiscussionSource) -> Result<DiscussionSource> {
    source.id = stable_id(&source.id, 100);
    source.title = truncate(source.title.trim(), 160);
    source.kind = stable_id(&source.kind, 40);
    source.reference = truncate(source.reference.trim(), 1_000);
    source.imported_at = truncate(source.imported_at.trim(), 64);
    if source.id.is_empty() || source.title.is_empty() {
        bail!("讨论来源必须包含 id 和标题");
    }
    Ok(source)
}

fn normalize_node(mut node: DiscussionNode) -> Result<DiscussionNode> {
    node.id = stable_id(&node.id, 100);
    node.root_id = stable_id(&node.root_id, 100);
    node.parent_id = stable_id(&node.parent_id, 100);
    node.kind = normalized_choice(
        &node.kind,
        &[
            "topic",
            "question",
            "claim",
            "hypothesis",
            "option",
            "objection",
            "evidence",
            "risk",
            "decision",
            "requirement",
            "feature",
            "task",
            "result",
        ],
        "topic",
        "节点类型",
    )?;
    node.title = truncate(node.title.trim(), 120);
    node.summary = truncate(node.summary.trim(), 1_200);
    node.status = normalized_choice(
        &node.status,
        &[
            "open",
            "exploring",
            "accepted",
            "rejected",
            "superseded",
            "implemented",
        ],
        "open",
        "节点状态",
    )?;
    node.authority = normalized_choice(
        &node.authority,
        &[
            "source",
            "proposal",
            "accepted",
            "current",
            "evidence",
            "historical",
        ],
        "source",
        "节点权威性",
    )?;
    node.section_id = stable_id(&node.section_id, 100);
    node.order = node.order.clamp(0, 999_999);
    if !is_color(&node.color) {
        node.color = color_for_kind(&node.kind).to_string();
    }
    node.source_refs = strings(node.source_refs, 48, 300);
    node.conversation_refs = strings(node.conversation_refs, 24, 300);
    node.document_paths = node
        .document_paths
        .into_iter()
        .take(48)
        .map(|path| normalize_document_path(&path))
        .collect::<Result<Vec<_>>>()?;
    node.feature_node_ids = strings(node.feature_node_ids, 48, 100);
    node.tags = strings(node.tags, 24, 80);
    if node.id.is_empty() || node.title.is_empty() {
        bail!("讨论节点必须包含 id 和标题");
    }
    Ok(node)
}

fn validate_nodes(
    nodes: &[DiscussionNode],
    node_ids: &HashSet<String>,
    source_ids: &HashSet<String>,
) -> Result<()> {
    let parents = nodes
        .iter()
        .map(|node| (node.id.as_str(), node.parent_id.as_str()))
        .collect::<HashMap<_, _>>();
    for node in nodes {
        if !node.parent_id.is_empty() && !node_ids.contains(&node.parent_id) {
            bail!("讨论节点 {} 引用了不存在的父节点", node.id);
        }
        if !node.root_id.is_empty() && !node_ids.contains(&node.root_id) {
            bail!("讨论节点 {} 引用了不存在的根节点", node.id);
        }
        for reference in &node.source_refs {
            let source = reference.split(['#', ':']).next().unwrap_or_default();
            if !source.is_empty() && !source_ids.contains(source) {
                bail!("讨论节点 {} 引用了不存在的来源 {}", node.id, source);
            }
        }
        let mut cursor = node.id.as_str();
        let mut visited = HashSet::new();
        for depth in 0..=24 {
            if !visited.insert(cursor) {
                bail!("讨论图父子关系存在循环：{}", node.id);
            }
            let parent = parents.get(cursor).copied().unwrap_or_default();
            if parent.is_empty() {
                break;
            }
            if depth == 24 {
                bail!("讨论图父子层级最多支持 25 层");
            }
            cursor = parent;
        }
    }
    Ok(())
}

fn normalize_edge(mut edge: DiscussionEdge, node_ids: &HashSet<String>) -> Result<DiscussionEdge> {
    edge.id = stable_id(&edge.id, 120);
    edge.source = stable_id(&edge.source, 100);
    edge.target = stable_id(&edge.target, 100);
    edge.relation = normalized_choice(
        &edge.relation,
        &[
            "decomposes_to",
            "supports",
            "opposes",
            "alternative_to",
            "depends_on",
            "answers",
            "spawns",
            "leads_to",
            "resolves",
            "merged_into",
            "decides",
            "promotes_to",
            "implements",
            "validated_by",
            "supersedes",
            "related_to",
        ],
        "related_to",
        "关系类型",
    )?;
    edge.label = truncate(edge.label.trim(), 100);
    if edge.id.is_empty()
        || edge.source == edge.target
        || !node_ids.contains(&edge.source)
        || !node_ids.contains(&edge.target)
    {
        bail!("讨论图关系引用了无效节点");
    }
    Ok(edge)
}

fn normalize_promotion(
    mut item: DiscussionPromotion,
    node_ids: &HashSet<&str>,
) -> Result<DiscussionPromotion> {
    item.id = stable_id(&item.id, 120);
    item.node_id = stable_id(&item.node_id, 100);
    item.path = normalize_document_path(&item.path)?;
    item.title = truncate(item.title.trim(), 160);
    item.document_type = stable_id(&item.document_type, 40);
    item.section_id = stable_id(&item.section_id, 100);
    if item.id.is_empty()
        || item.title.is_empty()
        || !node_ids.contains(item.node_id.as_str())
        || !item.path.to_ascii_lowercase().starts_with("docs/")
        || !item.path.to_ascii_lowercase().ends_with(".md")
    {
        bail!("讨论节点晋升必须引用有效节点和 docs/ 下的 Markdown");
    }
    if item.content.len() > 2 * 1024 * 1024 {
        bail!("晋升文档超过 2 MiB");
    }
    Ok(item)
}

fn unique_ids<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<HashSet<String>> {
    let values = values.map(str::to_string).collect::<Vec<_>>();
    let unique = values.iter().cloned().collect::<HashSet<_>>();
    if values.iter().any(String::is_empty) || values.len() != unique.len() {
        bail!("{label} id 必须非空且唯一");
    }
    Ok(unique)
}

fn normalized_choice(value: &str, allowed: &[&str], default: &str, label: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    let value = if value.is_empty() { default } else { &value };
    if !allowed.contains(&value) {
        bail!("{label}无效：{value}");
    }
    Ok(value.to_string())
}

fn strings(values: Vec<String>, count: usize, chars: usize) -> Vec<String> {
    let mut output = values
        .into_iter()
        .take(count)
        .map(|item| truncate(item.trim(), chars))
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    output.sort();
    output.dedup();
    output
}

fn stable_id(value: &str, limit: usize) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        .take(limit)
        .collect()
}

fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn is_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

fn color_for_kind(kind: &str) -> &'static str {
    match kind {
        "decision" | "result" => "#55b989",
        "risk" | "objection" => "#d66f78",
        "hypothesis" | "question" => "#d8a950",
        "requirement" | "feature" | "task" => "#5f91dc",
        "evidence" => "#50aaa7",
        _ => "#9a73dc",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> DiscussionGraph {
        DiscussionGraph {
            sources: vec![DiscussionSource {
                id: "chat".into(),
                title: "聊天".into(),
                reference: "docs/inbox/conversations/chat.md".into(),
                ..Default::default()
            }],
            nodes: vec![
                DiscussionNode {
                    id: "root".into(),
                    title: "开放商业网络".into(),
                    ..Default::default()
                },
                DiscussionNode {
                    id: "merchant".into(),
                    root_id: "root".into(),
                    parent_id: "root".into(),
                    kind: "feature".into(),
                    title: "商户 AI".into(),
                    source_refs: vec!["chat#12".into()],
                    ..Default::default()
                },
            ],
            edges: vec![DiscussionEdge {
                id: "root-merchant".into(),
                source: "root".into(),
                target: "merchant".into(),
                relation: "decomposes_to".into(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn normalizes_portable_discussion_graph() {
        let normalized = normalize_graph(graph()).unwrap();
        assert_eq!(normalized.nodes[1].status, "open");
        assert_eq!(normalized.nodes[1].authority, "source");
        assert_eq!(normalized.edges[0].relation, "decomposes_to");
    }

    #[test]
    fn rejects_parent_cycles_and_unknown_sources() {
        let mut cyclic = graph();
        cyclic.nodes[0].parent_id = "merchant".into();
        assert!(normalize_graph(cyclic).is_err());
        let mut unknown = graph();
        unknown.nodes[1].source_refs = vec!["missing#12".into()];
        assert!(normalize_graph(unknown).is_err());
    }

    #[test]
    fn merges_a_branch_that_reuses_existing_parent_and_source() {
        let fragment = DiscussionGraph {
            nodes: vec![DiscussionNode {
                id: "merchant-risk".into(),
                root_id: "root".into(),
                parent_id: "merchant".into(),
                kind: "risk".into(),
                title: "平台复制风险".into(),
                source_refs: vec!["chat#20".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let merged = merge_graph(graph(), fragment).unwrap();
        assert_eq!(merged.sources.len(), 1);
        assert_eq!(merged.nodes.len(), 3);
        assert_eq!(
            merged
                .nodes
                .iter()
                .find(|node| node.id == "merchant-risk")
                .unwrap()
                .parent_id,
            "merchant"
        );
    }
}
