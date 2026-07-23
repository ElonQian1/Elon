//! Deterministic metadata-only knowledge maps shared by MCP and the PC workbench.

use homecli_proto::ProjectDocumentEntry;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};

use crate::{
    project_document_governance::{CustomDocumentSection, DocumentSectionManifest},
    project_document_knowledge_graph_health::{
        aggregate_child_knowledge, append_status_findings, score_map, view_needs_implementation,
    },
    project_document_knowledge_graph_model::{
        KnowledgeMapBudget, KnowledgeMapCoverage, KnowledgeMapDiagnostics, KnowledgeMapEvidence,
        KnowledgeMapFinding, KnowledgeMapStats, ProjectKnowledgeEdgeConfig,
        ProjectKnowledgeGraphConfig, ProjectKnowledgeMap, ProjectKnowledgeMapEdge,
        ProjectKnowledgeMapNode, ProjectKnowledgeMaps, ProjectKnowledgeNodeConfig,
    },
    project_document_knowledge_graph_templates::template_graph,
};

pub(crate) const KNOWLEDGE_MAP_VIEWS: [&str; 3] = ["capabilities", "architecture", "topics"];

pub(crate) fn build_knowledge_maps(
    workspace: &Path,
    documents: &[ProjectDocumentEntry],
    manifest: &DocumentSectionManifest,
) -> ProjectKnowledgeMaps {
    ProjectKnowledgeMaps {
        capabilities: build_map(workspace, documents, manifest, "capabilities"),
        architecture: build_map(workspace, documents, manifest, "architecture"),
        topics: build_map(workspace, documents, manifest, "topics"),
    }
}

pub(crate) fn build_map(
    workspace: &Path,
    documents: &[ProjectDocumentEntry],
    manifest: &DocumentSectionManifest,
    view: &str,
) -> ProjectKnowledgeMap {
    let view = if KNOWLEDGE_MAP_VIEWS.contains(&view) {
        view
    } else {
        "capabilities"
    };
    let configured = manifest
        .knowledge_graph
        .nodes
        .iter()
        .any(|node| node.view == view);
    let (config, source) = if view == "topics" {
        (topic_graph(manifest, documents), "manifest")
    } else if configured {
        (manifest.knowledge_graph.clone(), "manifest")
    } else {
        (template_graph(&manifest.profile), "profile_template")
    };
    build_from_config(workspace, documents, manifest, view, config, source)
}

fn build_from_config(
    workspace: &Path,
    documents: &[ProjectDocumentEntry],
    manifest: &DocumentSectionManifest,
    view: &str,
    config: ProjectKnowledgeGraphConfig,
    source: &str,
) -> ProjectKnowledgeMap {
    let document_by_path = documents
        .iter()
        .map(|document| (normalize(&document.path), document))
        .collect::<HashMap<_, _>>();
    let root_id = format!("map-{view}-root");
    let mut findings = Vec::new();
    let mut nodes = config
        .nodes
        .iter()
        .filter(|node| node.view == view)
        .map(|node| {
            hydrate_node(
                workspace,
                node,
                documents,
                &document_by_path,
                source,
                &mut findings,
            )
        })
        .collect::<Vec<_>>();
    let node_ids = nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    for node in &mut nodes {
        if node.parent_id.is_empty() || !node_ids.contains(&node.parent_id) {
            node.parent_id = root_id.clone();
        }
    }
    let depths = resolve_depths(&nodes, &root_id);
    let child_counts = nodes
        .iter()
        .fold(HashMap::<String, usize>::new(), |mut counts, node| {
            *counts.entry(node.parent_id.clone()).or_default() += 1;
            counts
        });
    for node in &mut nodes {
        node.depth = depths.get(&node.id).copied().unwrap_or(1);
        node.child_count = child_counts.get(&node.id).copied().unwrap_or_default();
    }
    aggregate_child_knowledge(&mut nodes, &document_by_path, view);
    findings.retain(|item| {
        !matches!(
            item.code,
            "undocumented_node"
                | "missing_entrypoint"
                | "missing_implementation_evidence"
                | "stale_implementation_evidence"
        )
    });
    append_status_findings(&nodes, &mut findings);
    nodes.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then(left.order.cmp(&right.order))
            .then(left.label.cmp(&right.label))
    });
    let root_documents = prioritized_documents(documents)
        .into_iter()
        .take(24)
        .map(|doc| doc.path.clone())
        .collect::<Vec<_>>();
    let root_entrypoint = known_entrypoint(&manifest.home.entrypoint, &document_by_path)
        .or_else(|| root_documents.first().cloned())
        .unwrap_or_default();
    let root_coverage = coverage_for_paths(
        &root_documents,
        &document_by_path,
        !root_entrypoint.is_empty(),
    );
    let root = ProjectKnowledgeMapNode {
        id: root_id.clone(),
        view: view.to_string(),
        kind: "project".to_string(),
        label: root_label(manifest),
        detail: view_description(view).to_string(),
        color: "#9B73ED".to_string(),
        parent_id: String::new(),
        section_id: String::new(),
        depth: 0,
        child_count: child_counts.get(&root_id).copied().unwrap_or_default(),
        order: 0,
        document_count: documents.len(),
        document_paths: root_documents,
        entrypoint: root_entrypoint,
        entrypoint_source: if manifest.home.entrypoint.is_empty() {
            "inferred"
        } else {
            "configured"
        }
        .to_string(),
        missing_coverage: root_coverage
            .iter()
            .filter(|item| !item.covered)
            .map(|item| item.label.to_string())
            .collect(),
        documentation_status: "documented".to_string(),
        coverage: root_coverage,
        implementation_refs: Vec::new(),
        implementation_status: "not_applicable".to_string(),
        source: source.to_string(),
        tags: Vec::new(),
    };
    nodes.insert(0, root);
    let mut edges = nodes
        .iter()
        .filter(|node| node.id != root_id)
        .map(|node| ProjectKnowledgeMapEdge {
            id: format!("contains-{}", node.id),
            source: node.parent_id.clone(),
            target: node.id.clone(),
            relation: "contains".to_string(),
            label: String::new(),
            configured: node.parent_id != root_id || source == "manifest",
        })
        .collect::<Vec<_>>();
    edges.extend(
        config
            .edges
            .iter()
            .filter(|edge| node_ids.contains(&edge.source) && node_ids.contains(&edge.target))
            .map(map_edge),
    );
    let stats = collect_stats(&nodes);
    if nodes.len() == 1 {
        findings.push(finding(
            "empty_view",
            "warning",
            &root_id,
            "当前视图还没有任何节点。",
            "让 AI 提出最小节点树，再由用户审核应用。",
        ));
    }
    if source == "profile_template" {
        findings.push(finding(
            "derived_template",
            "info",
            &root_id,
            "当前视图来自项目类型模板，尚未固化为项目事实。",
            "与 AI 讨论后把确认的节点和证据写入共享清单。",
        ));
    }
    let score = score_map(&stats, &findings, view);
    ProjectKnowledgeMap {
        version: 1,
        view: view.to_string(),
        title: view_title(view).to_string(),
        source: source.to_string(),
        root_id,
        nodes,
        edges,
        stats,
        diagnostics: KnowledgeMapDiagnostics {
            structural_score: score.structural,
            status: if score.structural >= 85 {
                "healthy"
            } else if score.structural >= 60 {
                "review"
            } else {
                "needs_structure"
            },
            finding_score: score.finding,
            documentation_score: score.documentation,
            implementation_score: score.implementation,
            score_formula: score.formula,
            findings,
        },
        budget: KnowledgeMapBudget {
            classification_model_tokens: 0,
            markdown_bodies_read: 0,
            metadata_only: true,
        },
    }
}

fn hydrate_node(
    workspace: &Path,
    node: &ProjectKnowledgeNodeConfig,
    documents: &[ProjectDocumentEntry],
    by_path: &HashMap<String, &ProjectDocumentEntry>,
    source: &str,
    findings: &mut Vec<KnowledgeMapFinding>,
) -> ProjectKnowledgeMapNode {
    let mut paths = node
        .document_paths
        .iter()
        .filter_map(|path| {
            if by_path.contains_key(&normalize(path)) {
                Some(path.clone())
            } else {
                findings.push(finding(
                    "missing_document",
                    "warning",
                    &node.id,
                    &format!("关联文档不存在：{path}"),
                    "修正路径或移除过期关联。",
                ));
                None
            }
        })
        .collect::<Vec<_>>();
    if paths.is_empty() && source == "profile_template" {
        paths = infer_documents(node, documents);
    }
    if !node.entrypoint.is_empty()
        && by_path.contains_key(&normalize(&node.entrypoint))
        && !paths
            .iter()
            .any(|path| normalize(path) == normalize(&node.entrypoint))
    {
        paths.insert(0, node.entrypoint.clone());
    }
    paths.sort();
    paths.dedup();
    let entrypoint = known_entrypoint(&node.entrypoint, by_path)
        .or_else(|| paths.first().cloned())
        .unwrap_or_default();
    let entrypoint_source = if !node.entrypoint.is_empty() && entrypoint == node.entrypoint {
        "configured"
    } else if entrypoint.is_empty() {
        "missing"
    } else {
        "inferred"
    };
    let coverage = coverage_for_paths(&paths, by_path, !entrypoint.is_empty());
    let covered = coverage.iter().filter(|item| item.covered).count();
    let documentation_status = if paths.is_empty() {
        "undocumented"
    } else if entrypoint_source == "configured" || covered >= 3 {
        "documented"
    } else {
        "partial"
    };
    if paths.is_empty() {
        findings.push(finding(
            "undocumented_node",
            "warning",
            &node.id,
            "节点没有关联任何当前 Markdown。",
            "关联现有权威文档，确有缺口时再新增文档。",
        ));
    }
    if entrypoint_source == "missing" {
        findings.push(finding(
            "missing_entrypoint",
            "info",
            &node.id,
            "节点缺少推荐入口文档。",
            "从关联文档中指定唯一入口。",
        ));
    }
    let evidence = node
        .implementation_refs
        .iter()
        .map(|reference| verify_evidence(workspace, reference))
        .collect::<Vec<_>>();
    let implementation_status = if view_needs_implementation(&node.view) && evidence.is_empty() {
        findings.push(finding(
            "missing_implementation_evidence",
            "warning",
            &node.id,
            "节点没有代码、API 或测试证据。",
            "添加 file:/route:/symbol:/test: 引用，让 AI 可按需核对实现。",
        ));
        "missing"
    } else if evidence.iter().any(|item| item.verification == "missing") {
        findings.push(finding(
            "stale_implementation_evidence",
            "warning",
            &node.id,
            "部分实现引用已经失效。",
            "修正或移除过期实现引用。",
        ));
        "missing"
    } else if evidence.iter().any(|item| item.verification == "declared") {
        "declared"
    } else if evidence.is_empty() {
        "not_applicable"
    } else {
        "verified"
    };
    ProjectKnowledgeMapNode {
        id: node.id.clone(),
        view: node.view.clone(),
        kind: node.kind.clone(),
        label: node.label.clone(),
        detail: node.detail.clone(),
        color: node.color.clone(),
        parent_id: node.parent_id.clone(),
        section_id: node
            .id
            .strip_prefix("topic-")
            .map(|id| format!("custom:{id}"))
            .unwrap_or_default(),
        depth: 1,
        child_count: 0,
        order: node.order,
        document_count: paths.len(),
        document_paths: paths,
        entrypoint,
        entrypoint_source: entrypoint_source.to_string(),
        missing_coverage: coverage
            .iter()
            .filter(|item| !item.covered)
            .map(|item| item.label.to_string())
            .collect(),
        coverage,
        documentation_status: documentation_status.to_string(),
        implementation_refs: evidence,
        implementation_status: implementation_status.to_string(),
        source: source.to_string(),
        tags: node.tags.clone(),
    }
}

fn topic_graph(
    manifest: &DocumentSectionManifest,
    documents: &[ProjectDocumentEntry],
) -> ProjectKnowledgeGraphConfig {
    let assignments = manifest.assignments.iter().fold(
        HashMap::<String, Vec<String>>::new(),
        |mut map, (path, section)| {
            if let Some(id) = section.strip_prefix("custom:") {
                map.entry(id.to_string()).or_default().push(path.clone());
            }
            map
        },
    );
    ProjectKnowledgeGraphConfig {
        nodes: manifest
            .sections
            .iter()
            .map(|section| {
                topic_node(
                    section,
                    assignments.get(&section.id).cloned().unwrap_or_default(),
                    documents,
                )
            })
            .collect(),
        edges: Vec::new(),
    }
}

fn topic_node(
    section: &CustomDocumentSection,
    paths: Vec<String>,
    documents: &[ProjectDocumentEntry],
) -> ProjectKnowledgeNodeConfig {
    let known = documents
        .iter()
        .map(|document| normalize(&document.path))
        .collect::<HashSet<_>>();
    ProjectKnowledgeNodeConfig {
        id: format!("topic-{}", section.id),
        view: "topics".to_string(),
        kind: "topic".to_string(),
        label: section.label.clone(),
        detail: section.detail.clone(),
        parent_id: if section.parent_id.is_empty() {
            String::new()
        } else {
            format!("topic-{}", section.parent_id)
        },
        order: section.order,
        color: section.color.clone(),
        entrypoint: section.entrypoint.clone(),
        document_paths: paths
            .into_iter()
            .filter(|path| known.contains(&normalize(path)))
            .collect(),
        implementation_refs: Vec::new(),
        tags: Vec::new(),
    }
}

fn infer_documents(
    node: &ProjectKnowledgeNodeConfig,
    documents: &[ProjectDocumentEntry],
) -> Vec<String> {
    let terms = node
        .tags
        .iter()
        .chain(std::iter::once(&node.label))
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    prioritized_documents(documents)
        .into_iter()
        .filter(|document| {
            let text = format!(
                "{} {} {}",
                document.path,
                document.title,
                document.metadata.headings.join(" ")
            )
            .to_ascii_lowercase();
            terms.iter().any(|term| text.contains(term))
        })
        .take(12)
        .map(|document| document.path.clone())
        .collect()
}

pub(crate) fn coverage_for_paths(
    paths: &[String],
    by_path: &HashMap<String, &ProjectDocumentEntry>,
    has_entrypoint: bool,
) -> Vec<KnowledgeMapCoverage> {
    const SPECS: [(&str, &str); 6] = [
        ("overview", "入口"),
        ("requirements", "需求"),
        ("architecture", "设计"),
        ("reference", "参考"),
        ("operations", "操作"),
        ("evidence", "证据"),
    ];
    let mut counts = BTreeMap::<&str, usize>::new();
    for document in paths
        .iter()
        .filter_map(|path| by_path.get(&normalize(path)).copied())
    {
        for key in coverage_keys(document) {
            *counts.entry(key).or_default() += 1;
        }
    }
    if has_entrypoint {
        *counts.entry("overview").or_default() =
            counts.get("overview").copied().unwrap_or_default().max(1);
    }
    SPECS
        .into_iter()
        .map(|(key, label)| KnowledgeMapCoverage {
            key,
            label,
            count: counts.get(key).copied().unwrap_or_default(),
            covered: counts.get(key).copied().unwrap_or_default() > 0,
        })
        .collect()
}

fn coverage_keys(document: &ProjectDocumentEntry) -> Vec<&'static str> {
    let role = document.metadata.role.as_str();
    let text = format!(
        "{} {} {}",
        document.path,
        document.title,
        document.metadata.headings.join(" ")
    )
    .to_ascii_lowercase();
    let mut keys = Vec::new();
    if matches!(role, "policy" | "router" | "project_guide")
        || contains(&text, &["readme", "overview", "总览", "入口"])
    {
        keys.push("overview");
    }
    if role == "requirement" || contains(&text, &["requirement", "需求", "验收"]) {
        keys.push("requirements");
    }
    if role == "architecture" || contains(&text, &["architecture", "设计", "架构", "data-flow"])
    {
        keys.push("architecture");
    }
    if matches!(role, "spec" | "instruction" | "provider_adapter")
        || contains(&text, &["api", "reference", "schema", "接口", "规范"])
    {
        keys.push("reference");
    }
    if matches!(role, "runbook" | "guide")
        || contains(
            &text,
            &[
                "runbook", "deploy", "release", "setup", "操作", "发布", "运维",
            ],
        )
    {
        keys.push("operations");
    }
    if matches!(role, "status" | "report" | "decision")
        || contains(
            &text,
            &[
                "test", "report", "evidence", "status", "测试", "证据", "决策",
            ],
        )
    {
        keys.push("evidence");
    }
    keys
}

fn verify_evidence(workspace: &Path, reference: &str) -> KnowledgeMapEvidence {
    let verification = reference
        .split_once(':')
        .map(|(kind, value)| (kind, value.trim()))
        .map_or("declared", |(kind, value)| {
            if matches!(kind, "file" | "test") {
                if workspace.join(value).exists() {
                    "exists"
                } else {
                    "missing"
                }
            } else {
                "declared"
            }
        });
    KnowledgeMapEvidence {
        reference: reference.to_string(),
        verification,
    }
}

fn resolve_depths(nodes: &[ProjectKnowledgeMapNode], root_id: &str) -> HashMap<String, usize> {
    let parents = nodes
        .iter()
        .map(|node| (node.id.as_str(), node.parent_id.as_str()))
        .collect::<HashMap<_, _>>();
    nodes
        .iter()
        .map(|node| {
            let mut cursor = node.parent_id.as_str();
            let mut depth = 1;
            while cursor != root_id && depth < 8 {
                cursor = parents.get(cursor).copied().unwrap_or(root_id);
                depth += 1;
            }
            (node.id.clone(), depth)
        })
        .collect()
}

fn collect_stats(nodes: &[ProjectKnowledgeMapNode]) -> KnowledgeMapStats {
    let mut stats = KnowledgeMapStats::default();
    for node in nodes.iter().filter(|node| node.kind != "project") {
        stats.nodes += 1;
        stats.configured_nodes += usize::from(node.source == "manifest");
        match node.documentation_status.as_str() {
            "documented" => stats.documented += 1,
            "partial" => stats.partial += 1,
            _ => stats.undocumented += 1,
        }
        match node.implementation_status.as_str() {
            "verified" => stats.implementation_verified += 1,
            "declared" => stats.implementation_declared += 1,
            "missing" => stats.implementation_missing += 1,
            _ => {}
        }
    }
    stats
}

fn prioritized_documents(documents: &[ProjectDocumentEntry]) -> Vec<&ProjectDocumentEntry> {
    let mut output = documents.iter().collect::<Vec<_>>();
    output.sort_by_key(|document| {
        (
            std::cmp::Reverse(document.metadata.default_retrieval),
            document.metadata.ambiguous,
            document.path.clone(),
        )
    });
    output
}

fn known_entrypoint(
    value: &str,
    by_path: &HashMap<String, &ProjectDocumentEntry>,
) -> Option<String> {
    (!value.is_empty() && by_path.contains_key(&normalize(value))).then(|| value.to_string())
}

fn map_edge(edge: &ProjectKnowledgeEdgeConfig) -> ProjectKnowledgeMapEdge {
    ProjectKnowledgeMapEdge {
        id: edge.id.clone(),
        source: edge.source.clone(),
        target: edge.target.clone(),
        relation: edge.relation.clone(),
        label: edge.label.clone(),
        configured: true,
    }
}

fn finding(
    code: &'static str,
    severity: &'static str,
    node_id: &str,
    message: &str,
    action: &'static str,
) -> KnowledgeMapFinding {
    KnowledgeMapFinding {
        code,
        severity,
        node_id: node_id.to_string(),
        message: message.to_string(),
        suggested_action: action,
    }
}

fn contains(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}
fn normalize(value: &str) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}
fn root_label(manifest: &DocumentSectionManifest) -> String {
    if manifest.home.title.is_empty() {
        "项目知识图谱".to_string()
    } else {
        manifest.home.title.clone()
    }
}
fn view_title(view: &str) -> &'static str {
    match view {
        "architecture" => "技术架构图",
        "topics" => "文档主题图",
        _ => "产品功能图",
    }
}
fn view_description(view: &str) -> &'static str {
    match view {
        "architecture" => "真实技术组件、依赖与实现证据",
        "topics" => "文档讲什么以及推荐阅读入口",
        _ => "产品能力、子能力与对应文档和实现证据",
    }
}

#[cfg(test)]
#[path = "project_document_knowledge_graph_tests.rs"]
mod tests;
