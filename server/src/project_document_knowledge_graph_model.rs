//! Shared manifest schema for product-capability and technical-architecture maps.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::project_document_file_operation_model::normalize_document_path;

pub(crate) const MAX_KNOWLEDGE_GRAPH_NODES: usize = 256;
pub(crate) const MAX_KNOWLEDGE_GRAPH_EDGES: usize = 512;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectKnowledgeMaps {
    pub capabilities: ProjectKnowledgeMap,
    pub architecture: ProjectKnowledgeMap,
    pub topics: ProjectKnowledgeMap,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectKnowledgeMap {
    pub version: u8,
    pub view: String,
    pub title: String,
    pub source: String,
    pub root_id: String,
    pub nodes: Vec<ProjectKnowledgeMapNode>,
    pub edges: Vec<ProjectKnowledgeMapEdge>,
    pub stats: KnowledgeMapStats,
    pub diagnostics: KnowledgeMapDiagnostics,
    pub budget: KnowledgeMapBudget,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectKnowledgeMapNode {
    pub id: String,
    pub view: String,
    pub kind: String,
    pub label: String,
    pub detail: String,
    pub color: String,
    pub parent_id: String,
    pub section_id: String,
    pub depth: usize,
    pub child_count: usize,
    pub order: i32,
    pub document_count: usize,
    pub document_paths: Vec<String>,
    pub entrypoint: String,
    pub entrypoint_source: String,
    pub coverage: Vec<KnowledgeMapCoverage>,
    pub missing_coverage: Vec<String>,
    pub documentation_status: String,
    pub implementation_refs: Vec<KnowledgeMapEvidence>,
    pub implementation_status: String,
    pub source: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectKnowledgeMapEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: String,
    pub label: String,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct KnowledgeMapCoverage {
    pub key: &'static str,
    pub label: &'static str,
    pub covered: bool,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct KnowledgeMapEvidence {
    pub reference: String,
    pub verification: &'static str,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct KnowledgeMapStats {
    pub nodes: usize,
    pub configured_nodes: usize,
    pub documented: usize,
    pub partial: usize,
    pub undocumented: usize,
    pub implementation_verified: usize,
    pub implementation_declared: usize,
    pub implementation_missing: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct KnowledgeMapDiagnostics {
    pub structural_score: u8,
    pub status: &'static str,
    pub findings: Vec<KnowledgeMapFinding>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct KnowledgeMapFinding {
    pub code: &'static str,
    pub severity: &'static str,
    pub node_id: String,
    pub message: String,
    pub suggested_action: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct KnowledgeMapBudget {
    pub classification_model_tokens: u8,
    pub markdown_bodies_read: u8,
    pub metadata_only: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectKnowledgeGraphConfig {
    #[serde(default)]
    pub nodes: Vec<ProjectKnowledgeNodeConfig>,
    #[serde(default)]
    pub edges: Vec<ProjectKnowledgeEdgeConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectKnowledgeNodeConfig {
    pub id: String,
    pub view: String,
    #[serde(default)]
    pub kind: String,
    pub label: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub parent_id: String,
    #[serde(default)]
    pub order: i32,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub entrypoint: String,
    #[serde(default)]
    pub document_paths: Vec<String>,
    #[serde(default)]
    pub implementation_refs: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectKnowledgeEdgeConfig {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub relation: String,
    #[serde(default)]
    pub label: String,
}

pub(crate) fn normalize_graph_config(
    mut graph: ProjectKnowledgeGraphConfig,
) -> Result<ProjectKnowledgeGraphConfig> {
    if graph.nodes.len() > MAX_KNOWLEDGE_GRAPH_NODES
        || graph.edges.len() > MAX_KNOWLEDGE_GRAPH_EDGES
    {
        bail!("项目知识图谱超过安全上限");
    }
    graph.nodes = graph
        .nodes
        .into_iter()
        .map(normalize_node)
        .collect::<Result<Vec<_>>>()?;
    let node_ids = graph
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    let node_views = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.view.clone()))
        .collect::<HashMap<_, _>>();
    if node_ids.len() != graph.nodes.len() {
        bail!("项目知识图谱节点 id 必须全局唯一");
    }
    validate_hierarchy(&graph.nodes, &node_ids)?;
    graph.edges = graph
        .edges
        .into_iter()
        .map(|edge| normalize_edge(edge, &node_ids, &node_views))
        .collect::<Result<Vec<_>>>()?;
    let edge_ids = graph
        .edges
        .iter()
        .map(|edge| &edge.id)
        .collect::<HashSet<_>>();
    if edge_ids.len() != graph.edges.len() {
        bail!("项目知识图谱关系 id 必须唯一");
    }
    Ok(graph)
}

pub(crate) fn merge_graph_config(
    current: ProjectKnowledgeGraphConfig,
    proposed: ProjectKnowledgeGraphConfig,
) -> Result<ProjectKnowledgeGraphConfig> {
    let mut nodes = current
        .nodes
        .into_iter()
        .map(|node| (node.id.clone(), node))
        .collect::<HashMap<_, _>>();
    nodes.extend(
        proposed
            .nodes
            .into_iter()
            .map(|node| (node.id.clone(), node)),
    );
    let mut edges = current
        .edges
        .into_iter()
        .map(|edge| (edge.id.clone(), edge))
        .collect::<HashMap<_, _>>();
    edges.extend(
        proposed
            .edges
            .into_iter()
            .map(|edge| (edge.id.clone(), edge)),
    );
    normalize_graph_config(ProjectKnowledgeGraphConfig {
        nodes: nodes.into_values().collect(),
        edges: edges.into_values().collect(),
    })
}

pub(crate) fn validate_graph_document_paths(
    graph: &ProjectKnowledgeGraphConfig,
    known_paths: &HashSet<String>,
) -> Result<()> {
    for node in &graph.nodes {
        for path in node
            .document_paths
            .iter()
            .chain((!node.entrypoint.is_empty()).then_some(&node.entrypoint))
        {
            if !known_paths.contains(&path.to_ascii_lowercase()) {
                bail!("知识图谱节点 {} 引用了目录中不存在的文档：{path}", node.id);
            }
        }
    }
    Ok(())
}

fn normalize_node(mut node: ProjectKnowledgeNodeConfig) -> Result<ProjectKnowledgeNodeConfig> {
    node.id = stable_id(&node.id, 80);
    node.view = match node.view.trim().to_ascii_lowercase().as_str() {
        "capabilities" | "architecture" => node.view.trim().to_ascii_lowercase(),
        _ => bail!("知识图谱节点 view 只支持 capabilities 或 architecture"),
    };
    node.kind = stable_id(&node.kind, 40);
    if node.kind.is_empty() {
        node.kind = if node.view == "capabilities" {
            "capability".to_string()
        } else {
            "component".to_string()
        };
    }
    node.label = truncate(node.label.trim(), 60);
    node.detail = truncate(node.detail.trim(), 240);
    node.parent_id = stable_id(&node.parent_id, 80);
    node.order = node.order.clamp(0, 9_999);
    if !is_hex_color(&node.color) {
        node.color = default_color();
    }
    node.entrypoint = optional_document_path(&node.entrypoint)?;
    node.document_paths = node
        .document_paths
        .into_iter()
        .take(48)
        .map(|path| normalize_document_path(&path))
        .collect::<Result<Vec<_>>>()?;
    node.document_paths.sort();
    node.document_paths.dedup();
    node.implementation_refs = bounded_strings(node.implementation_refs, 48, 500);
    node.tags = bounded_strings(node.tags, 24, 80);
    if node.id.is_empty() || node.label.is_empty() {
        bail!("知识图谱节点必须包含有效 id 和 label");
    }
    Ok(node)
}

fn normalize_edge(
    mut edge: ProjectKnowledgeEdgeConfig,
    node_ids: &HashSet<String>,
    node_views: &HashMap<String, String>,
) -> Result<ProjectKnowledgeEdgeConfig> {
    edge.id = stable_id(&edge.id, 100);
    edge.source = stable_id(&edge.source, 80);
    edge.target = stable_id(&edge.target, 80);
    edge.relation = stable_id(&edge.relation, 40);
    edge.label = truncate(edge.label.trim(), 80);
    if edge.relation.is_empty() {
        edge.relation = "related_to".to_string();
    }
    if edge.id.is_empty()
        || edge.source == edge.target
        || !node_ids.contains(&edge.source)
        || !node_ids.contains(&edge.target)
    {
        bail!("知识图谱关系引用了无效节点");
    }
    if node_views.get(&edge.source) != node_views.get(&edge.target) {
        bail!("知识图谱关系不能跨越产品功能图与技术架构图");
    }
    Ok(edge)
}

fn validate_hierarchy(
    nodes: &[ProjectKnowledgeNodeConfig],
    node_ids: &HashSet<String>,
) -> Result<()> {
    let parents = nodes
        .iter()
        .map(|node| (node.id.as_str(), node.parent_id.as_str()))
        .collect::<HashMap<_, _>>();
    let views = nodes
        .iter()
        .map(|node| (node.id.as_str(), node.view.as_str()))
        .collect::<HashMap<_, _>>();
    for node in nodes {
        if !node.parent_id.is_empty()
            && (!node_ids.contains(&node.parent_id)
                || views.get(node.parent_id.as_str()) != Some(&node.view.as_str()))
        {
            bail!("知识图谱节点 {} 引用了无效或跨视图父节点", node.id);
        }
        let mut cursor = node.id.as_str();
        let mut visited = HashSet::new();
        for depth in 0..=5 {
            if !visited.insert(cursor) {
                bail!("知识图谱层级存在循环：{}", node.id);
            }
            let parent = parents.get(cursor).copied().unwrap_or_default();
            if parent.is_empty() {
                break;
            }
            if depth == 5 {
                bail!("知识图谱层级最多支持 6 层：{}", node.id);
            }
            cursor = parent;
        }
    }
    Ok(())
}

fn optional_document_path(value: &str) -> Result<String> {
    if value.trim().is_empty() {
        Ok(String::new())
    } else {
        normalize_document_path(value)
    }
}

fn stable_id(value: &str, max: usize) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        .take(max)
        .collect()
}

fn bounded_strings(values: Vec<String>, count: usize, chars: usize) -> Vec<String> {
    let mut output = values
        .into_iter()
        .take(count)
        .map(|value| truncate(value.trim(), chars))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    output.sort();
    output.dedup();
    output
}

fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

fn default_color() -> String {
    "#7f8fb3".to_string()
}
