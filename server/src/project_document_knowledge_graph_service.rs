//! Bounded, metadata-only graph queries for MCP consumers.

use anyhow::{anyhow, bail, Result};
use homecli_proto::ProjectDocumentsSnapshot;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use crate::{
    project_docs_scan::{collect_project_documents_with_options, ProjectDocumentScanOptions},
    project_document_analysis_model::compact_document,
    project_document_files::content_revision,
    project_document_governance::{parse_manifest, DocumentSectionManifest, SECTION_CONFIG_PATH},
    project_document_governance_facets::effective_facets_with_metadata,
    project_document_knowledge_graph::build_knowledge_maps,
    project_document_knowledge_graph_model::{
        ProjectKnowledgeMap, ProjectKnowledgeMapEdge, ProjectKnowledgeMapNode, ProjectKnowledgeMaps,
    },
};

pub(crate) fn get_map(
    workspace: &Path,
    view: &str,
    root_id: Option<&str>,
    depth: usize,
    query: Option<&str>,
    max_nodes: usize,
) -> Result<Value> {
    let (snapshot, manifest, manifest_revision) = load(workspace)?;
    let maps = build_knowledge_maps(workspace, &snapshot.documents, &manifest);
    let identity = graph_identity(workspace, &snapshot, manifest_revision.as_deref(), &maps)?;
    if view == "overview" {
        let views = [&maps.capabilities, &maps.architecture, &maps.topics]
            .into_iter()
            .map(|map| {
                json!({
                    "view": map.view, "title": map.title, "source": map.source,
                    "stats": map.stats, "diagnostics": map.diagnostics,
                    "root_id": map.root_id,
                })
            })
            .collect::<Vec<_>>();
        return Ok(json!({
            "catalog_revision": snapshot.revision,
            "identity": identity,
            "views": views,
            "budget": {"classification_model_tokens":0,"markdown_bodies_read":0,"metadata_only":true}
        }));
    }
    ensure_view(view)?;
    let map = map_for_view(&maps, view);
    let (nodes, edges, truncated) = select_map(
        map,
        root_id,
        depth.clamp(1, 6),
        query,
        max_nodes.clamp(1, 200),
    )?;
    Ok(json!({
        "catalog_revision": snapshot.revision,
        "identity": identity,
        "view": map.view,
        "title": map.title,
        "source": map.source,
        "root_id": map.root_id,
        "nodes": nodes,
        "edges": edges,
        "stats": map.stats,
        "diagnostics": map.diagnostics,
        "selection": {"root_id":root_id,"depth":depth.clamp(1,6),"query":query,"max_nodes":max_nodes.clamp(1,200),"truncated":truncated},
        "budget": map.budget,
    }))
}

pub(crate) fn get_node(workspace: &Path, node_id: &str) -> Result<Value> {
    let (snapshot, manifest, manifest_revision) = load(workspace)?;
    let maps = build_knowledge_maps(workspace, &snapshot.documents, &manifest);
    let identity = graph_identity(workspace, &snapshot, manifest_revision.as_deref(), &maps)?;
    let map_views = [&maps.capabilities, &maps.architecture, &maps.topics];
    let (map, node) = map_views
        .into_iter()
        .find_map(|map| {
            map.nodes
                .iter()
                .find(|node| node.id == node_id)
                .map(|node| (map, node))
        })
        .ok_or_else(|| anyhow!("未知知识图谱节点：{node_id}"))?;
    let linked_paths = node
        .document_paths
        .iter()
        .map(|path| normalize(path))
        .collect::<HashSet<_>>();
    let documents = snapshot
        .documents
        .iter()
        .filter(|document| linked_paths.contains(&normalize(&document.path)))
        .take(24)
        .map(|document| compact_document(document, &manifest))
        .collect::<Vec<_>>();
    let relations = map
        .edges
        .iter()
        .filter(|edge| edge.source == node.id || edge.target == node.id)
        .collect::<Vec<_>>();
    let findings = map
        .diagnostics
        .findings
        .iter()
        .filter(|finding| finding.node_id == node.id)
        .collect::<Vec<_>>();
    Ok(json!({
        "catalog_revision": snapshot.revision,
        "identity": identity,
        "view": map.view,
        "node": node,
        "relations": relations,
        "documents": documents,
        "findings": findings,
        "recommended_next": [
            "先核对 entrypoint 和 implementation_refs；不要读取未关联文档正文。",
            "需要语义判断时，只用 project_docs_read 读取这里返回的少量 document path。",
            "结构改进写入 proposed_knowledge_graph，内容整理继续使用现有主题与治理建议。"
        ],
        "budget": {"classification_model_tokens":0,"markdown_bodies_read":0,"metadata_only":true}
    }))
}

pub(crate) fn review_map(workspace: &Path, view: &str) -> Result<Value> {
    ensure_view(view)?;
    let (snapshot, manifest, manifest_revision) = load(workspace)?;
    let maps = build_knowledge_maps(workspace, &snapshot.documents, &manifest);
    let identity = graph_identity(workspace, &snapshot, manifest_revision.as_deref(), &maps)?;
    let map = map_for_view(&maps, view);
    let review_questions = match view {
        "architecture" => vec![
            "组件边界是否与真实进程、部署单元和数据流一致？",
            "每个关键组件是否有源码、路由、数据或测试证据？",
            "配置的依赖关系是否遗漏关键调用或把主题误当成组件？",
        ],
        "topics" => vec![
            "主题是否只回答文档讲什么，而没有暗中改变权威性？",
            "每个核心主题是否有唯一入口和推荐阅读顺序？",
            "讨论、证据和历史材料是否仍由治理轴正确降权？",
        ],
        _ => vec![
            "节点是否代表用户可感知能力，而不是文档类别或代码目录？",
            "父子关系是否表达能力分解，每项是否有文档和实现证据？",
            "功能缺口与文档缺口是否被分开陈述，避免把有文档当成已实现？",
        ],
    };
    Ok(json!({
        "catalog_revision": snapshot.revision,
        "identity": identity,
        "view": view,
        "source": map.source,
        "stats": map.stats,
        "diagnostics": map.diagnostics,
        "review_questions": review_questions,
        "decision_rules": [
            "主题、功能、技术组件和治理状态是四个正交维度，不能互相替代。",
            "Markdown 是内容真源；图谱只保存稳定节点以及文档/实现引用。",
            "有文档只证明文档覆盖；实现状态必须由 file:/route:/symbol:/test: 证据单独说明。",
            "AI 只能提出 proposed_knowledge_graph，应用继续受 revision、权限模式和 Git 事务保护。"
        ],
        "suggestion_target": ".elon/document-organization-suggestions.json#proposed_knowledge_graph",
        "budget": {"classification_model_tokens":0,"markdown_bodies_read":0,"metadata_only":true}
    }))
}

pub(crate) fn plan_context(
    workspace: &Path,
    query: &str,
    node_id: Option<&str>,
    max_tokens: u64,
    max_documents: usize,
    max_rule_tokens: u64,
) -> Result<Value> {
    let query = query.trim();
    if query.is_empty() && node_id.is_none() {
        bail!("project_docs_plan_context 必须提供 query 或 node_id");
    }
    let (snapshot, manifest, manifest_revision) = load(workspace)?;
    let maps = build_knowledge_maps(workspace, &snapshot.documents, &manifest);
    let identity = graph_identity(workspace, &snapshot, manifest_revision.as_deref(), &maps)?;
    let all_maps = [&maps.capabilities, &maps.architecture, &maps.topics];
    let query_lower = query.to_lowercase();
    let query_terms = query_lower
        .split(|character: char| {
            character.is_whitespace() || matches!(character, ',' | '/' | ':' | '，' | '、')
        })
        .filter(|term| term.chars().count() >= 2)
        .collect::<Vec<_>>();
    let matched_nodes = all_maps
        .iter()
        .flat_map(|map| map.nodes.iter())
        .filter(|node| {
            node_id.is_some_and(|id| id == node.id)
                || (!query_lower.is_empty()
                    && format!("{} {} {}", node.label, node.detail, node.tags.join(" "))
                        .to_ascii_lowercase()
                        .contains(&query_lower))
        })
        .collect::<Vec<_>>();
    if let Some(id) = node_id {
        if !matched_nodes.iter().any(|node| node.id == id) {
            bail!("未知知识图谱节点：{id}");
        }
    }
    let linked = matched_nodes
        .iter()
        .flat_map(|node| node.document_paths.iter())
        .map(|path| normalize(path))
        .collect::<HashSet<_>>();
    let max_tokens = max_tokens.clamp(200, 12_000);
    let max_rule_tokens = max_rule_tokens.clamp(200, 6_000);
    let max_documents = max_documents.clamp(1, 24);
    let historical_requested = ["历史", "旧", "报告", "trace", "e2e", "report", "archive"]
        .iter()
        .any(|term| query_lower.contains(term));
    let mut mandatory_paths = vec![".github/copilot-instructions.md", "AGENTS.md"];
    if ["文档", "知识", "治理", "authority", "document"]
        .iter()
        .any(|term| query_lower.contains(term))
    {
        mandatory_paths.push(".github/instructions/document-authority.instructions.md");
    }
    let mut mandatory_rules = Vec::new();
    let mut used_rule_tokens = 0u64;
    for path in mandatory_paths {
        let Some(document) = snapshot
            .documents
            .iter()
            .find(|document| normalize(&document.path) == normalize(path))
        else {
            continue;
        };
        let tokens = document.metadata.token_estimate.max(1);
        if !mandatory_rules.is_empty() && used_rule_tokens.saturating_add(tokens) > max_rule_tokens
        {
            continue;
        }
        used_rule_tokens = used_rule_tokens.saturating_add(tokens);
        mandatory_rules.push(json!({
            "reason":"shared_minimum_rule_or_task_router",
            "document":compact_document(document, &manifest),
        }));
    }
    let mut candidates = snapshot
        .documents
        .iter()
        .filter(|document| {
            !mandatory_rules.iter().any(|rule| {
                rule.pointer("/document/path").and_then(Value::as_str)
                    == Some(document.path.as_str())
            })
        })
        .filter(|document| {
            let normalized_path = normalize(&document.path);
            if linked.contains(&normalized_path)
                || contextual_entrypoint_score(&normalized_path, &query_lower) > 0
            {
                return true;
            }
            if historical_requested {
                return true;
            }
            let path = document.path.replace('\\', "/");
            let facets = effective_facets_with_metadata(
                document,
                manifest.governance_facets.get(&path),
                manifest.document_metadata.get(&path),
            );
            !is_historical_noise(document) && facets.retrieval != "excluded"
        })
        .map(|document| {
            let text = format!(
                "{} {} {}",
                document.path,
                document.title,
                document.metadata.headings.join(" ")
            )
            .to_lowercase();
            let path = normalize(&document.path);
            let term_score = query_terms
                .iter()
                .filter(|term| text.contains(*term))
                .count()
                * 20;
            let authority_score = usize::from(document.metadata.default_retrieval) * 15
                + usize::from(matches!(
                    document.metadata.authority.as_str(),
                    "binding" | "authoritative"
                )) * 20;
            let entrypoint_score = contextual_entrypoint_score(&path, &query_lower);
            let score = usize::from(linked.contains(&path)) * 100
                + term_score
                + authority_score
                + entrypoint_score;
            (score, document)
        })
        .filter(|(score, _)| *score >= 20)
        .collect::<Vec<_>>();
    candidates.sort_by(|(left_score, left), (right_score, right)| {
        right_score.cmp(left_score).then(left.path.cmp(&right.path))
    });
    let mut selected = Vec::new();
    let mut used_tokens = 0u64;
    for (score, document) in candidates {
        let tokens = document.metadata.token_estimate.max(1);
        let authoritative_entrypoint =
            contextual_entrypoint_score(&normalize(&document.path), &query_lower) > 0;
        let planned_tokens = if authoritative_entrypoint {
            tokens.min(1_200)
        } else {
            tokens
        };
        if selected.len() >= max_documents
            || (!selected.is_empty() && used_tokens.saturating_add(planned_tokens) > max_tokens)
        {
            continue;
        }
        used_tokens = used_tokens.saturating_add(planned_tokens);
        selected.push(json!({
            "score":score,
            "reason":context_reason(&document.path, score, linked.contains(&normalize(&document.path))),
            "read_plan":{
                "mode":if planned_tokens < tokens {"sectional"} else {"full_if_needed"},
                "estimated_selected_tokens":planned_tokens,
                "estimated_full_document_tokens":tokens,
                "max_chars":planned_tokens.saturating_mul(4),
            },
            "document":compact_document(document, &manifest)
        }));
    }
    Ok(json!({
        "catalog_revision": snapshot.revision,
        "identity": identity,
        "query": query,
        "node_id": node_id,
        "matched_nodes": matched_nodes.iter().map(|node| json!({"id":node.id,"view":node.view,"label":node.label,"status":node.documentation_status})).collect::<Vec<_>>(),
        "mandatory_rules": mandatory_rules,
        "relevant_documents": selected,
        "budget": {
            "rules":{"max_tokens":max_rule_tokens,"estimated_tokens_selected":used_rule_tokens,"documents_selected":mandatory_rules.len()},
            "relevant_content":{"max_tokens":max_tokens,"estimated_tokens_selected":used_tokens,"max_documents":max_documents,"documents_selected":selected.len()},
            "classification_model_tokens":0,"markdown_bodies_read":0
        },
        "read_instruction": "按顺序只读取当前任务真正需要的文档；先读标题层级，仍有歧义再调用 project_docs_read。"
    }))
}

fn is_historical_noise(document: &homecli_proto::ProjectDocumentEntry) -> bool {
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

fn contextual_entrypoint_score(path: &str, query: &str) -> usize {
    let mut score = 0;
    if path == "docs/system-architecture.md"
        && ["系统", "架构", "system", "architecture"]
            .iter()
            .any(|term| query.contains(term))
    {
        score += 180;
    }
    if path == "docs/codex-desktop-pc-supervision.md"
        && ["pc", "监督", "supervision", "codex"]
            .iter()
            .any(|term| query.contains(term))
    {
        score += 170;
    }
    if path == "docs/project-document-governance-mcp.md"
        && ["文档", "知识", "治理", "mcp", "document"]
            .iter()
            .any(|term| query.contains(term))
    {
        score += 190;
    }
    if path == "docs/supervised-pc-project-development.md"
        && ["pc", "监督", "项目"]
            .iter()
            .any(|term| query.contains(term))
    {
        score += 120;
    }
    score
}

fn context_reason(path: &str, score: usize, linked: bool) -> Value {
    let path = normalize(path);
    json!({
        "graph_linked":linked,
        "authoritative_entrypoint":matches!(path.as_str(),
            "docs/system-architecture.md"
            | "docs/codex-desktop-pc-supervision.md"
            | "docs/project-document-governance-mcp.md"
            | "docs/supervised-pc-project-development.md"
        ),
        "ranking_score":score,
    })
}

fn load(
    workspace: &Path,
) -> Result<(
    ProjectDocumentsSnapshot,
    DocumentSectionManifest,
    Option<String>,
)> {
    let snapshot = collect_project_documents_with_options(
        workspace,
        ProjectDocumentScanOptions {
            seed_missing_defaults: false,
            catalog_only: true,
            include_analysis: false,
        },
    )?;
    let manifest = fs::read_to_string(workspace.join(SECTION_CONFIG_PATH)).ok();
    let manifest_revision = manifest.as_deref().map(content_revision);
    Ok((
        snapshot,
        parse_manifest(manifest.as_deref())?,
        manifest_revision,
    ))
}

fn graph_identity<T: Serialize>(
    workspace: &Path,
    snapshot: &ProjectDocumentsSnapshot,
    manifest_revision: Option<&str>,
    maps: &T,
) -> Result<Value> {
    let canonical_workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf())
        .to_string_lossy()
        .to_string();
    Ok(json!({
        "workspace": snapshot.workspace_path,
        "canonical_workspace": canonical_workspace,
        "manifest_revision": manifest_revision,
        "knowledge_map_revision": content_revision(&serde_json::to_string(maps)?),
    }))
}

fn map_for_view<'a>(maps: &'a ProjectKnowledgeMaps, view: &str) -> &'a ProjectKnowledgeMap {
    match view {
        "architecture" => &maps.architecture,
        "topics" => &maps.topics,
        _ => &maps.capabilities,
    }
}

fn select_map<'a>(
    map: &'a ProjectKnowledgeMap,
    root_id: Option<&str>,
    depth: usize,
    query: Option<&str>,
    max_nodes: usize,
) -> Result<(
    Vec<&'a ProjectKnowledgeMapNode>,
    Vec<&'a ProjectKnowledgeMapEdge>,
    bool,
)> {
    let root = root_id.unwrap_or(&map.root_id);
    if !map.nodes.iter().any(|node| node.id == root) {
        bail!("当前视图不存在 root_id：{root}");
    }
    let query = query.unwrap_or_default().trim().to_ascii_lowercase();
    let by_id = map
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();
    let mut selected = HashSet::new();
    if !query.is_empty() {
        for node in map.nodes.iter().filter(|node| {
            format!(
                "{} {} {} {}",
                node.label,
                node.detail,
                node.tags.join(" "),
                node.document_paths.join(" ")
            )
            .to_ascii_lowercase()
            .contains(&query)
        }) {
            let mut cursor = Some(node.id.as_str());
            while let Some(id) = cursor {
                selected.insert(id.to_string());
                cursor = by_id.get(id).and_then(|item| {
                    (!item.parent_id.is_empty()).then_some(item.parent_id.as_str())
                });
            }
        }
    } else {
        let children =
            map.nodes
                .iter()
                .fold(HashMap::<&str, Vec<&str>>::new(), |mut output, node| {
                    output
                        .entry(node.parent_id.as_str())
                        .or_default()
                        .push(node.id.as_str());
                    output
                });
        let mut queue = vec![(root, 0usize)];
        while let Some((id, level)) = queue.pop() {
            selected.insert(id.to_string());
            if level < depth {
                queue.extend(
                    children
                        .get(id)
                        .into_iter()
                        .flatten()
                        .map(|child| (*child, level + 1)),
                );
            }
        }
    }
    let mut nodes = map
        .nodes
        .iter()
        .filter(|node| selected.contains(&node.id))
        .collect::<Vec<_>>();
    let truncated = nodes.len() > max_nodes;
    nodes.truncate(max_nodes);
    let visible = nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    let edges = map
        .edges
        .iter()
        .filter(|edge| {
            visible.contains(edge.source.as_str()) && visible.contains(edge.target.as_str())
        })
        .collect::<Vec<_>>();
    Ok((nodes, edges, truncated))
}

fn ensure_view(view: &str) -> Result<()> {
    if matches!(view, "capabilities" | "architecture" | "topics") {
        Ok(())
    } else {
        bail!("view 只支持 overview、capabilities、architecture 或 topics")
    }
}

fn normalize(value: &str) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}
