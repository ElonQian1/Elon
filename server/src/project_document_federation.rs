//! Hierarchical knowledge nodes for large repositories and multi-module projects.

use anyhow::{bail, Context, Result};
use homecli_proto::ProjectDocumentEntry;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

use crate::{
    project_document_architecture::analyze_knowledge_architecture,
    project_document_governance::{DocumentKnowledgeHome, DocumentSectionManifest},
};

pub(crate) const FEDERATION_CONFIG_PATH: &str = ".elon/knowledge-federation.json";
const MAX_NODES: usize = 256;
const MAX_DEPTH: usize = 6;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct KnowledgeNode {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub parent_id: String,
    #[serde(default)]
    pub scope_path: String,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub home: DocumentKnowledgeHome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct KnowledgeFederationManifest {
    #[serde(default = "schema_version")]
    pub version: u8,
    #[serde(default)]
    pub nodes: Vec<KnowledgeNode>,
}

impl Default for KnowledgeFederationManifest {
    fn default() -> Self {
        Self {
            version: schema_version(),
            nodes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct KnowledgeNodeHealth {
    pub id: String,
    pub label: String,
    pub parent_id: String,
    pub scope_path: String,
    pub profile: String,
    pub owner: String,
    pub document_count: usize,
    pub direct_children: usize,
    pub score: u8,
    pub status: &'static str,
    pub home_configured: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct KnowledgeFederationHealth {
    pub enabled: bool,
    pub source: &'static str,
    pub root_id: String,
    pub node_count: usize,
    pub aggregated_score: u8,
    pub unhealthy_nodes: usize,
    pub max_depth: usize,
    pub nodes: Vec<KnowledgeNodeHealth>,
}

pub(crate) fn analyze_federation(
    workspace: &Path,
    documents: &[ProjectDocumentEntry],
    root_manifest: &DocumentSectionManifest,
) -> Result<KnowledgeFederationHealth> {
    let (manifest, source) = load_or_infer_manifest(workspace, documents)?;
    let nodes = normalized_nodes(manifest.nodes)?;
    let child_counts = nodes
        .iter()
        .fold(HashMap::<String, usize>::new(), |mut counts, node| {
            if !node.parent_id.is_empty() {
                *counts.entry(node.parent_id.clone()).or_default() += 1;
            }
            counts
        });
    let mut health = Vec::new();
    for node in &nodes {
        let scoped = documents_for_scope(documents, &node.scope_path);
        let mut node_manifest = root_manifest.clone();
        if !node.profile.is_empty() && node.profile != "auto" {
            node_manifest.profile = node.profile.clone();
        }
        if !node.home.title.is_empty() || !node.home.entrypoint.is_empty() {
            node_manifest.home = node.home.clone();
        }
        let architecture = analyze_knowledge_architecture(&scoped, &node_manifest);
        health.push(KnowledgeNodeHealth {
            id: node.id.clone(),
            label: node.label.clone(),
            parent_id: node.parent_id.clone(),
            scope_path: node.scope_path.clone(),
            profile: architecture.profile,
            owner: node.owner.clone(),
            document_count: scoped.len(),
            direct_children: child_counts.get(&node.id).copied().unwrap_or_default(),
            score: architecture.score,
            status: architecture.status,
            home_configured: architecture.home_configured,
        });
    }
    let total_documents = health.iter().map(|node| node.document_count).sum::<usize>();
    let aggregated_score = if total_documents == 0 {
        100
    } else {
        (health
            .iter()
            .map(|node| usize::from(node.score) * node.document_count)
            .sum::<usize>()
            / total_documents) as u8
    };
    let unhealthy_nodes = health.iter().filter(|node| node.score < 60).count();
    let max_depth = nodes
        .iter()
        .map(|node| node_depth(node, &nodes))
        .max()
        .unwrap_or_default();
    let root_id = nodes
        .iter()
        .find(|node| node.parent_id.is_empty())
        .map(|node| node.id.clone())
        .unwrap_or_else(|| "root".to_string());
    Ok(KnowledgeFederationHealth {
        enabled: health.len() > 1,
        source,
        root_id,
        node_count: health.len(),
        aggregated_score,
        unhealthy_nodes,
        max_depth,
        nodes: health,
    })
}

pub(crate) fn documents_for_node<'a>(
    documents: &'a [ProjectDocumentEntry],
    federation: &KnowledgeFederationHealth,
    scope_id: Option<&str>,
) -> Result<Vec<&'a ProjectDocumentEntry>> {
    let Some(scope_id) = scope_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(documents.iter().collect());
    };
    let node = federation
        .nodes
        .iter()
        .find(|node| node.id == scope_id)
        .ok_or_else(|| anyhow::anyhow!("未知知识节点：{scope_id}"))?;
    Ok(documents
        .iter()
        .filter(|document| in_scope(&document.path, &node.scope_path))
        .collect())
}

fn load_or_infer_manifest(
    workspace: &Path,
    documents: &[ProjectDocumentEntry],
) -> Result<(KnowledgeFederationManifest, &'static str)> {
    let path = workspace.join(FEDERATION_CONFIG_PATH);
    if path.is_file() {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("读取联邦知识清单失败：{}", path.display()))?;
        let manifest: KnowledgeFederationManifest = serde_json::from_str(&content)?;
        if manifest.version != schema_version() {
            bail!("knowledge-federation.json 仅支持 version=1");
        }
        return Ok((manifest, "manifest"));
    }
    Ok((infer_manifest(workspace, documents), "metadata"))
}

fn infer_manifest(
    workspace: &Path,
    documents: &[ProjectDocumentEntry],
) -> KnowledgeFederationManifest {
    let mut counts = HashMap::<String, usize>::new();
    for document in documents {
        if let Some(top) = normalize(&document.path)
            .split('/')
            .next()
            .filter(|value| !value.is_empty())
        {
            *counts.entry(top.to_string()).or_default() += 1;
        }
    }
    let mut nodes = vec![KnowledgeNode {
        id: "root".to_string(),
        label: "项目知识根".to_string(),
        ..KnowledgeNode::default()
    }];
    let mut candidates = counts
        .into_iter()
        .filter(|(scope, count)| *count >= 3 && module_marker(workspace, scope))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.0.cmp(&right.0));
    for (scope, _) in candidates.into_iter().take(MAX_NODES - 1) {
        nodes.push(KnowledgeNode {
            id: sanitize_id(&scope),
            label: scope.clone(),
            parent_id: "root".to_string(),
            scope_path: scope,
            profile: "auto".to_string(),
            ..KnowledgeNode::default()
        });
    }
    KnowledgeFederationManifest { version: 1, nodes }
}

fn normalized_nodes(mut nodes: Vec<KnowledgeNode>) -> Result<Vec<KnowledgeNode>> {
    if nodes.is_empty() {
        nodes.push(KnowledgeNode {
            id: "root".to_string(),
            label: "项目知识根".to_string(),
            ..KnowledgeNode::default()
        });
    }
    if nodes.len() > MAX_NODES {
        bail!("联邦知识节点超过 {MAX_NODES} 个安全上限");
    }
    let ids = nodes
        .iter()
        .map(|node| sanitize_id(&node.id))
        .collect::<Vec<_>>();
    if ids.iter().any(String::is_empty) {
        bail!("联邦知识节点必须包含有效 id");
    }
    let mut seen = HashMap::new();
    for (index, id) in ids.iter().enumerate() {
        if seen.insert(id.clone(), index).is_some() {
            bail!("联邦知识节点 id 重复：{id}");
        }
    }
    for (index, node) in nodes.iter_mut().enumerate() {
        node.id = ids[index].clone();
        node.parent_id = sanitize_id(&node.parent_id);
        node.scope_path = normalize(&node.scope_path).trim_matches('/').to_string();
        node.label = node.label.trim().chars().take(80).collect();
        node.owner = node.owner.trim().chars().take(80).collect();
        if node.label.is_empty() {
            bail!("联邦知识节点必须包含 label");
        }
        if !node.parent_id.is_empty() && !seen.contains_key(&node.parent_id) {
            bail!("联邦知识节点引用了不存在的父节点：{}", node.parent_id);
        }
    }
    for node in &nodes {
        if node_depth(node, &nodes) > MAX_DEPTH {
            bail!("联邦知识节点最多支持 {MAX_DEPTH} 层：{}", node.id);
        }
    }
    Ok(nodes)
}

fn node_depth(node: &KnowledgeNode, nodes: &[KnowledgeNode]) -> usize {
    let parents = nodes
        .iter()
        .map(|item| (item.id.as_str(), item.parent_id.as_str()))
        .collect::<HashMap<_, _>>();
    let mut cursor = node.id.as_str();
    let mut visited = std::collections::HashSet::new();
    let mut depth = 0;
    loop {
        if !visited.insert(cursor) {
            return MAX_DEPTH + 1;
        }
        let Some(parent) = parents
            .get(cursor)
            .copied()
            .filter(|value| !value.is_empty())
        else {
            return depth;
        };
        depth += 1;
        cursor = parent;
    }
}

fn documents_for_scope(
    documents: &[ProjectDocumentEntry],
    scope: &str,
) -> Vec<ProjectDocumentEntry> {
    documents
        .iter()
        .filter(|document| in_scope(&document.path, scope))
        .cloned()
        .collect()
}

fn in_scope(path: &str, scope: &str) -> bool {
    let path = normalize(path).to_ascii_lowercase();
    let scope = normalize(scope).trim_matches('/').to_ascii_lowercase();
    scope.is_empty() || path == scope || path.starts_with(&format!("{scope}/"))
}

fn module_marker(workspace: &Path, scope: &str) -> bool {
    [
        "Cargo.toml",
        "package.json",
        "build.gradle",
        "build.gradle.kts",
        "go.mod",
        "pyproject.toml",
    ]
    .iter()
    .any(|marker| workspace.join(scope).join(marker).is_file())
}

fn sanitize_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(64)
        .collect()
}

fn normalize(value: &str) -> String {
    value.replace('\\', "/")
}

fn schema_version() -> u8 {
    1
}

#[cfg(test)]
#[path = "project_document_federation_tests.rs"]
mod tests;
