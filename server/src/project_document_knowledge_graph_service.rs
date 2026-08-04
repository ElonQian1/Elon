//! Bounded, metadata-only graph queries for MCP consumers.

mod context_match;

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
    project_document_federation::{
        analyze_federation, health_node_matches_path, KnowledgeNodeHealth,
    },
    project_document_files::content_revision,
    project_document_governance::{parse_manifest, DocumentSectionManifest, SECTION_CONFIG_PATH},
    project_document_governance_facets::effective_facets_with_metadata,
    project_document_knowledge_graph::build_knowledge_maps,
    project_document_knowledge_graph_model::{
        ProjectKnowledgeMap, ProjectKnowledgeMapEdge, ProjectKnowledgeMapNode, ProjectKnowledgeMaps,
    },
};
use context_match::{
    context_query_terms, context_reason, explicit_document_matches, governance_intent_score,
    is_historical_noise, is_task_specific_customization, manifest_entrypoint_score,
};

const EXPLICIT_DOCUMENT_SCORE: usize = 10_000;

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
    plan_context_scoped(
        workspace,
        query,
        node_id,
        None,
        max_tokens,
        max_documents,
        max_rule_tokens,
    )
}

pub(crate) fn plan_context_scoped(
    workspace: &Path,
    query: &str,
    node_id: Option<&str>,
    federation_scope_id: Option<&str>,
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
    let federation_scope = federation_scope_id
        .filter(|value| !value.trim().is_empty())
        .map(|scope_id| {
            analyze_federation(workspace, &snapshot.documents, &manifest)?
                .nodes
                .into_iter()
                .find(|node| node.id == scope_id)
                .ok_or_else(|| anyhow!("未知知识节点：{scope_id}"))
        })
        .transpose()?;
    let all_maps = [&maps.capabilities, &maps.architecture, &maps.topics];
    let query_lower = query.to_lowercase();
    let query_terms = context_query_terms(&query_lower);
    let explicit_document_paths = explicit_document_matches(&snapshot.documents, &query_lower);
    let mut scored_nodes = all_maps
        .iter()
        .flat_map(|map| map.nodes.iter())
        .filter_map(|node| {
            if federation_scope
                .as_ref()
                .is_some_and(|scope| !graph_node_in_scope(node, scope))
            {
                return None;
            }
            if let Some(id) = node_id {
                return (id == node.id).then_some((usize::MAX, node));
            }
            let text =
                format!("{} {} {}", node.label, node.detail, node.tags.join(" ")).to_lowercase();
            let score = query_terms
                .iter()
                .filter(|term| text.contains(term.as_str()))
                .count();
            (score > 0).then_some((score, node))
        })
        .collect::<Vec<_>>();
    scored_nodes.sort_by(|(left_score, left), (right_score, right)| {
        right_score
            .cmp(left_score)
            .then(left.order.cmp(&right.order))
            .then(left.id.cmp(&right.id))
    });
    let matched_nodes = scored_nodes.into_iter().take(12).collect::<Vec<_>>();
    if let Some(id) = node_id {
        if !matched_nodes.iter().any(|(_, node)| node.id == id) {
            bail!("未知知识图谱节点：{id}");
        }
    }
    let strongest_node_score = matched_nodes
        .iter()
        .filter_map(|(score, _)| (*score != usize::MAX).then_some(*score))
        .max()
        .unwrap_or(0);
    let node_score_threshold = if strongest_node_score <= 2 {
        strongest_node_score
    } else {
        strongest_node_score - 1
    };
    let strongly_matched_nodes = matched_nodes
        .iter()
        .copied()
        .filter(|(score, _)| *score == usize::MAX || (*score > 0 && *score >= node_score_threshold))
        .collect::<Vec<_>>();
    let linked = strongly_matched_nodes
        .iter()
        .flat_map(|(_, node)| node.document_paths.iter())
        .map(|path| normalize(path))
        .collect::<HashSet<_>>();
    let linked_entrypoint_scores = strongly_matched_nodes.iter().fold(
        HashMap::<String, usize>::new(),
        |mut scores, (score, node)| {
            if !node.entrypoint.is_empty() {
                let relevance = if *score == usize::MAX {
                    12
                } else {
                    (*score).min(12)
                };
                let boost = 80 + relevance * 40;
                scores
                    .entry(normalize(&node.entrypoint))
                    .and_modify(|current| *current = (*current).max(boost))
                    .or_insert(boost);
            }
            scores
        },
    );
    let linked_entrypoints = linked_entrypoint_scores
        .keys()
        .cloned()
        .collect::<HashSet<_>>();
    let max_tokens = max_tokens.clamp(200, 12_000);
    let max_rule_tokens = max_rule_tokens.clamp(200, 6_000);
    let max_documents = max_documents.clamp(1, 24);
    let source_material_requested = [
        "历史", "旧", "报告", "讨论", "追溯", "来源", "trace", "e2e", "report", "archive", "source",
    ]
    .iter()
    .any(|term| query_lower.contains(term));
    let proposal_requested = ["草稿", "提案", "draft", "proposal"]
        .iter()
        .any(|term| query_lower.contains(term));
    let historical_requested = source_material_requested || proposal_requested;
    let customizations_requested = [
        "指令",
        "规则",
        "定制",
        "提示词",
        "代理",
        "agent",
        "prompt",
        "skill",
        "instruction",
        "policy",
    ]
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
            if federation_scope
                .as_ref()
                .is_some_and(|scope| !health_node_matches_path(scope, &document.path))
            {
                return false;
            }
            !mandatory_rules.iter().any(|rule| {
                rule.pointer("/document/path").and_then(Value::as_str)
                    == Some(document.path.as_str())
            })
        })
        .filter(|document| {
            let normalized_path = normalize(&document.path);
            let explicitly_requested = explicit_document_paths.contains(&normalized_path);
            if is_task_specific_customization(document) && !customizations_requested {
                return explicitly_requested;
            }
            let manifest_path = document.path.replace('\\', "/");
            let facets = effective_facets_with_metadata(
                document,
                manifest.governance_facets.get(&manifest_path),
                manifest.document_metadata.get(&manifest_path),
            );
            if facets.retrieval == "excluded" && !historical_requested && !explicitly_requested {
                return false;
            }
            let entrypoint_score =
                manifest_entrypoint_score(&normalized_path, &query_terms, &manifest);
            if explicitly_requested
                || linked.contains(&normalized_path)
                || linked_entrypoints.contains(&normalized_path)
                || entrypoint_score > 0
            {
                return true;
            }
            if historical_requested {
                return true;
            }
            !is_historical_noise(document)
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
            let explicitly_requested = explicit_document_paths.contains(&path);
            let manifest_path = document.path.replace('\\', "/");
            let term_score = query_terms
                .iter()
                .filter(|term| text.contains(term.as_str()))
                .count()
                * 20;
            let facets = effective_facets_with_metadata(
                document,
                manifest.governance_facets.get(&manifest_path),
                manifest.document_metadata.get(&manifest_path),
            );
            let authority_score = usize::from(facets.retrieval != "excluded") * 15
                + usize::from(matches!(
                    facets.authority.as_str(),
                    "binding" | "authoritative"
                )) * 20;
            let lifecycle_score = usize::from(
                source_material_requested
                    && (facets.lifecycle == "source_material"
                        || document.metadata.role == "discussion"),
            ) * 120
                + usize::from(
                    proposal_requested
                        && (facets.lifecycle == "draft" || facets.authority == "proposal"),
                ) * 80;
            let entrypoint_score = manifest_entrypoint_score(&path, &query_terms, &manifest);
            let score = usize::from(explicitly_requested) * EXPLICIT_DOCUMENT_SCORE
                + usize::from(linked_entrypoints.contains(&path)) * 800
                + usize::from(linked.contains(&path)) * 100
                + linked_entrypoint_scores.get(&path).copied().unwrap_or(0)
                + term_score
                + authority_score
                + lifecycle_score
                + entrypoint_score
                + governance_intent_score(&query_lower, term_score, &facets);
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
        let path = normalize(&document.path);
        let manifest_path = document.path.replace('\\', "/");
        let knowledge_entrypoint = linked_entrypoints.contains(&path)
            || manifest_entrypoint_score(&path, &query_terms, &manifest) > 0;
        let facets = effective_facets_with_metadata(
            document,
            manifest.governance_facets.get(&manifest_path),
            manifest.document_metadata.get(&manifest_path),
        );
        let base_planned_tokens = if knowledge_entrypoint {
            tokens.min(1_200)
        } else {
            tokens
        };
        let remaining_tokens = max_tokens.saturating_sub(used_tokens);
        let planned_tokens = if knowledge_entrypoint
            && base_planned_tokens > remaining_tokens
            && remaining_tokens >= 200
        {
            remaining_tokens
        } else {
            base_planned_tokens
        };
        if selected.len() >= max_documents
            || used_tokens.saturating_add(planned_tokens) > max_tokens
        {
            continue;
        }
        used_tokens = used_tokens.saturating_add(planned_tokens);
        selected.push(json!({
            "score":score,
            "reason":context_reason(
                score,
                explicit_document_paths.contains(&path),
                linked.contains(&path),
                knowledge_entrypoint,
                matches!(facets.authority.as_str(), "binding" | "authoritative"),
            ),
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
        "federation_scope_id": federation_scope_id,
        "matched_nodes": matched_nodes.iter().map(|(score, node)| json!({
            "id":node.id,
            "view":node.view,
            "label":node.label,
            "status":node.documentation_status,
            "score":score,
            "entrypoint":node.entrypoint,
            "document_paths":node.document_paths.iter().take(8).collect::<Vec<_>>(),
            "implementation_refs":node.implementation_refs.iter().take(8).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
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

fn graph_node_in_scope(node: &ProjectKnowledgeMapNode, scope: &KnowledgeNodeHealth) -> bool {
    std::iter::once(node.entrypoint.as_str())
        .chain(node.document_paths.iter().map(String::as_str))
        .filter(|path| !path.trim().is_empty())
        .any(|path| health_node_matches_path(scope, path))
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
