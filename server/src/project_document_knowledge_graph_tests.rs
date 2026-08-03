use super::build_knowledge_maps;
use crate::{
    project_docs_scan::{collect_project_documents_with_options, ProjectDocumentScanOptions},
    project_document_governance::parse_manifest,
    project_document_knowledge_graph_model::{
        normalize_graph_config, ProjectKnowledgeEdgeConfig, ProjectKnowledgeGraphConfig,
        ProjectKnowledgeNodeConfig,
    },
    project_document_knowledge_graph_service::{get_map, get_node, plan_context, review_map},
};
use std::time::Instant;
use std::{fs, path::PathBuf};

fn fixture(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_knowledge_graph_{label}_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join(".elon")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("docs/README.md"),
        "# 项目总览\n\n## 架构\n\n当前入口。\n",
    )
    .unwrap();
    fs::write(root.join("docs/API.md"), "# API 参考\n\n## GET /health\n").unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(
        root.join(".elon/document-sections.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "version": 1,
            "profile": "software-api",
            "home": {"title":"示例项目","summary":"测试统一图谱","entrypoint":"docs/README.md","start_here":["docs/README.md"]},
            "sections": [
                {"id":"overview","label":"总览","detail":"入口","color":"#9A74E8","entrypoint":"docs/README.md"},
                {"id":"reference","label":"接口","detail":"API","color":"#4FA9B8","entrypoint":"docs/API.md"}
            ],
            "assignments": {"docs/README.md":"custom:overview","docs/API.md":"custom:reference"},
            "governance_facets": {
                "docs/README.md": {"retrieval":"on_demand","lifecycle":"active","authority":"authoritative","document_type":"overview"},
                "docs/API.md": {"retrieval":"on_demand","lifecycle":"active","authority":"authoritative","document_type":"api_reference"}
            },
            "document_metadata": {
                "docs/README.md": {"doc_type":"overview","owner":"docs-team","reviewed_at":"2026-07-20"},
                "docs/API.md": {"doc_type":"api-reference","owner":"api-team","reviewed_at":"2026-07-20"}
            },
            "knowledge_graph": {
                "nodes": [
                    {"id":"cap-api","view":"capabilities","kind":"capability","label":"健康检查","detail":"公开健康接口","color":"#4FA9B8","entrypoint":"docs/API.md","document_paths":["docs/API.md"],"implementation_refs":["file:src/main.rs"]},
                    {"id":"arch-service","view":"architecture","kind":"service","label":"API 服务","detail":"HTTP 服务","color":"#5F8FE3","entrypoint":"docs/README.md","document_paths":["docs/README.md"],"implementation_refs":["file:src/main.rs"]}
                ],
                "edges": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
    root
}

#[test]
fn builds_three_separate_metadata_only_views() {
    let root = fixture("views");
    let snapshot = collect_project_documents_with_options(
        &root,
        ProjectDocumentScanOptions {
            seed_missing_defaults: false,
            catalog_only: true,
            include_analysis: false,
        },
    )
    .unwrap();
    let content = fs::read_to_string(root.join(".elon/document-sections.json")).unwrap();
    let manifest = parse_manifest(Some(&content)).unwrap();
    let maps = build_knowledge_maps(&root, &snapshot.documents, &manifest);

    assert!(maps
        .capabilities
        .nodes
        .iter()
        .any(|node| node.id == "cap-api"));
    assert!(maps
        .architecture
        .nodes
        .iter()
        .any(|node| node.id == "arch-service"));
    assert!(maps
        .topics
        .nodes
        .iter()
        .any(|node| node.id == "topic-overview"));
    let capability = maps
        .capabilities
        .nodes
        .iter()
        .find(|node| node.id == "cap-api")
        .unwrap();
    assert_eq!(capability.implementation_status, "verified");
    assert_eq!(capability.document_paths, vec!["docs/API.md"]);
    assert_eq!(maps.capabilities.budget.markdown_bodies_read, 0);
    fs::remove_dir_all(root).ok();
}

#[test]
fn parent_nodes_aggregate_child_documents_and_implementation_evidence() {
    let root = fixture("aggregate");
    let snapshot = collect_project_documents_with_options(
        &root,
        ProjectDocumentScanOptions {
            seed_missing_defaults: false,
            catalog_only: true,
            include_analysis: false,
        },
    )
    .unwrap();
    let mut manifest = parse_manifest(Some(
        &fs::read_to_string(root.join(".elon/document-sections.json")).unwrap(),
    ))
    .unwrap();
    manifest.knowledge_graph.nodes = vec![
        ProjectKnowledgeNodeConfig {
            id: "cap-parent".into(),
            view: "capabilities".into(),
            label: "父能力".into(),
            ..Default::default()
        },
        ProjectKnowledgeNodeConfig {
            id: "cap-child".into(),
            view: "capabilities".into(),
            label: "子能力".into(),
            parent_id: "cap-parent".into(),
            entrypoint: "docs/API.md".into(),
            document_paths: vec!["docs/API.md".into()],
            implementation_refs: vec!["file:src/main.rs".into()],
            ..Default::default()
        },
    ];
    let maps = build_knowledge_maps(&root, &snapshot.documents, &manifest);
    let parent = maps
        .capabilities
        .nodes
        .iter()
        .find(|node| node.id == "cap-parent")
        .unwrap();
    assert!(parent.document_paths.contains(&"docs/API.md".to_string()));
    assert_eq!(parent.entrypoint_source, "child_aggregate");
    assert_eq!(parent.implementation_status, "verified");
    assert!(!maps
        .capabilities
        .diagnostics
        .findings
        .iter()
        .any(|finding| {
            finding.node_id == "cap-parent"
                && matches!(
                    finding.code,
                    "undocumented_node" | "missing_implementation_evidence"
                )
        }));

    manifest
        .knowledge_graph
        .nodes
        .iter_mut()
        .for_each(|node| node.implementation_refs.clear());
    let without_evidence = build_knowledge_maps(&root, &snapshot.documents, &manifest);
    assert_eq!(
        without_evidence.capabilities.stats.implementation_declared,
        0
    );
    assert_eq!(
        without_evidence
            .capabilities
            .diagnostics
            .implementation_score,
        Some(0)
    );
    assert!(without_evidence.capabilities.diagnostics.structural_score < 70);
    fs::remove_dir_all(root).ok();
}

#[test]
fn graph_model_rejects_cycles_and_unknown_views() {
    let node = |id: &str, parent: &str, view: &str| ProjectKnowledgeNodeConfig {
        id: id.to_string(),
        view: view.to_string(),
        label: id.to_string(),
        parent_id: parent.to_string(),
        ..Default::default()
    };
    assert!(normalize_graph_config(ProjectKnowledgeGraphConfig {
        nodes: vec![
            node("a", "b", "capabilities"),
            node("b", "a", "capabilities")
        ],
        edges: Vec::new(),
    })
    .is_err());
    assert!(normalize_graph_config(ProjectKnowledgeGraphConfig {
        nodes: vec![node("a", "", "governance")],
        edges: Vec::new(),
    })
    .is_err());
}

#[test]
fn graph_model_rejects_edges_to_unknown_nodes() {
    let graph = ProjectKnowledgeGraphConfig {
        nodes: vec![ProjectKnowledgeNodeConfig {
            id: "a".to_string(),
            view: "capabilities".to_string(),
            label: "A".to_string(),
            ..Default::default()
        }],
        edges: vec![ProjectKnowledgeEdgeConfig {
            id: "bad".to_string(),
            source: "a".to_string(),
            target: "missing".to_string(),
            ..Default::default()
        }],
    };
    assert!(normalize_graph_config(graph).is_err());
}

#[test]
fn graph_model_rejects_cross_view_edges_that_cannot_be_rendered() {
    let graph = ProjectKnowledgeGraphConfig {
        nodes: vec![
            ProjectKnowledgeNodeConfig {
                id: "cap".to_string(),
                view: "capabilities".to_string(),
                label: "Capability".to_string(),
                ..ProjectKnowledgeNodeConfig::default()
            },
            ProjectKnowledgeNodeConfig {
                id: "arch".to_string(),
                view: "architecture".to_string(),
                label: "Architecture".to_string(),
                ..ProjectKnowledgeNodeConfig::default()
            },
        ],
        edges: vec![ProjectKnowledgeEdgeConfig {
            id: "cross-view".to_string(),
            source: "cap".to_string(),
            target: "arch".to_string(),
            ..ProjectKnowledgeEdgeConfig::default()
        }],
    };
    let error = normalize_graph_config(graph).unwrap_err();
    assert!(error.to_string().contains("不能跨越"));
}

#[test]
fn mcp_graph_queries_are_bounded_and_do_not_read_bodies() {
    let root = fixture("queries");
    let map = get_map(&root, "capabilities", None, 2, None, 10).unwrap();
    assert_eq!(map["budget"]["markdown_bodies_read"], 0);
    assert_eq!(map["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(
        map["identity"]["canonical_workspace"],
        root.canonicalize().unwrap().to_string_lossy().as_ref()
    );
    assert!(map["identity"]["manifest_revision"].is_string());
    assert!(map["identity"]["knowledge_map_revision"].is_string());

    let node = get_node(&root, "cap-api").unwrap();
    assert_eq!(node["documents"][0]["path"], "docs/API.md");
    assert!(node["documents"][0].get("content").is_none());

    let review = review_map(&root, "capabilities").unwrap();
    assert_eq!(
        review["suggestion_target"],
        ".elon/document-organization-suggestions.json#proposed_knowledge_graph"
    );
    assert_eq!(
        review["identity"]["knowledge_map_revision"],
        map["identity"]["knowledge_map_revision"]
    );

    let plan = plan_context(&root, "健康检查", Some("cap-api"), 1_000, 4, 2_000).unwrap();
    assert_eq!(plan["budget"]["markdown_bodies_read"], 0);
    assert_eq!(
        plan["relevant_documents"][0]["document"]["path"],
        "docs/API.md"
    );
    assert_eq!(
        plan["relevant_documents"][0]["document"]["authority"],
        "authoritative"
    );
    assert_eq!(
        plan["relevant_documents"][0]["document"]["default_retrieval"],
        true
    );
    assert_eq!(
        plan["relevant_documents"][0]["reason"]["authoritative_entrypoint"],
        true
    );
    assert!(plan["budget"]["rules"].is_object());
    assert!(plan["budget"]["relevant_content"].is_object());
    fs::remove_dir_all(root).ok();
}

#[test]
fn explicit_draft_path_overrides_default_retrieval_exclusion() {
    let root = fixture("explicit-draft");
    fs::create_dir_all(root.join("docs/drafts")).unwrap();
    fs::write(
        root.join("docs/drafts/sui-shadow-settlement.md"),
        "# Sui 影子结算计划\n\n待评审草案。\n",
    )
    .unwrap();
    let manifest_path = root.join(".elon/document-sections.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["governance_facets"]["docs/drafts/sui-shadow-settlement.md"] = serde_json::json!({
        "retrieval":"excluded",
        "lifecycle":"draft",
        "authority":"proposal",
        "document_type":"technical_design"
    });
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let plan = plan_context(
        &root,
        "请读取 docs/drafts/sui-shadow-settlement.md 并实现下一阶段",
        None,
        1_000,
        4,
        2_000,
    )
    .unwrap();
    let selected = plan["relevant_documents"].as_array().unwrap();
    assert_eq!(
        selected[0]["document"]["path"],
        "docs/drafts/sui-shadow-settlement.md"
    );
    assert_eq!(selected[0]["document"]["default_retrieval"], false);
    assert_eq!(selected[0]["reason"]["explicit_document_match"], true);
    fs::remove_dir_all(root).ok();
}

#[test]
fn current_fact_questions_prioritize_the_current_status_document() {
    let root = fixture("current-status");
    fs::write(
        root.join("AI_CURRENT.md"),
        "# 当前事实\n\n这是当前实现、建设中能力和已否决路线的短快照。\n",
    )
    .unwrap();
    let manifest_path = root.join(".elon/document-sections.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["assignments"]["AI_CURRENT.md"] = serde_json::json!("custom:overview");
    manifest["governance_facets"]["AI_CURRENT.md"] = serde_json::json!({
        "retrieval": "required",
        "lifecycle": "active",
        "authority": "authoritative",
        "document_type": "current_status"
    });
    manifest["document_metadata"]["AI_CURRENT.md"] = serde_json::json!({
        "doc_type": "current_status",
        "owner": "test",
        "reviewed_at": "2026-07-30",
        "version_status": "current"
    });
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let plan = plan_context(
        &root,
        "项目当前已经实现什么、正在建设什么、哪些已经否决？",
        None,
        3_000,
        8,
        1_000,
    )
    .unwrap();
    assert_eq!(
        plan["relevant_documents"][0]["document"]["path"],
        "AI_CURRENT.md"
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn route_status_questions_prioritize_the_relevant_decision() {
    let root = fixture("route-decision");
    let decision_path = "docs/decisions/reject-ai-to-ai-skill-route.md";
    fs::create_dir_all(root.join("docs/decisions")).unwrap();
    fs::write(
        root.join(decision_path),
        "# 否决 AI-to-AI Skill 路线\n\n该路线不作为当前主架构。\n",
    )
    .unwrap();
    let manifest_path = root.join(".elon/document-sections.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["assignments"][decision_path] = serde_json::json!("custom:overview");
    manifest["governance_facets"][decision_path] = serde_json::json!({
        "retrieval": "on_demand",
        "lifecycle": "active",
        "authority": "authoritative",
        "document_type": "decision"
    });
    manifest["document_metadata"][decision_path] = serde_json::json!({
        "doc_type": "decision",
        "owner": "test",
        "reviewed_at": "2026-07-30",
        "version_status": "current"
    });
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let plan = plan_context(
        &root,
        "项目现在是否采用 AI-to-AI Skill 或 Skill-to-Skill 调度作为主架构？",
        None,
        3_000,
        8,
        1_000,
    )
    .unwrap();
    assert_eq!(
        plan["relevant_documents"][0]["document"]["path"],
        decision_path
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn self_project_overview_is_bounded_and_fast_enough_for_interactive_use() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let started = Instant::now();
    let overview = get_map(&root, "overview", None, 2, None, 80).unwrap();
    let elapsed = started.elapsed();
    assert_eq!(overview["views"].as_array().unwrap().len(), 3);
    assert_eq!(overview["budget"]["markdown_bodies_read"], 0);
    let context = plan_context(
        &root,
        "文档知识架构治理 系统架构 PC监督 MCP 低token",
        None,
        12_000,
        12,
        6_000,
    )
    .unwrap();
    let paths = context["relevant_documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| {
            item.pointer("/document/path")
                .and_then(serde_json::Value::as_str)
        })
        .collect::<Vec<_>>();
    assert!(paths.contains(&"docs/system-architecture.md"), "{paths:?}");
    assert!(
        paths.contains(&"docs/project-document-governance-mcp.md"),
        "{paths:?}"
    );
    assert!(context["relevant_documents"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["document"]["default_retrieval"] == true));
    assert!(!paths.iter().any(|path| {
        let path = path.to_ascii_lowercase();
        path.contains("e2e") || path.contains("trace")
    }));
    assert!(!paths.iter().any(|path| path.starts_with(".github/agents/")
        || path.starts_with(".github/prompts/")
        || path.starts_with(".github/skills/")
        || path.starts_with(".github/instructions/")
        || matches!(*path, "AI_RULES.md" | "AI_TASK_TEMPLATE.md")));

    let supervision = plan_context(
        &root,
        "Codex 桌面监督 PC 节点执行与证据验收",
        None,
        4_000,
        8,
        1_600,
    )
    .unwrap();
    let supervision_paths = supervision["relevant_documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| {
            item.pointer("/document/path")
                .and_then(serde_json::Value::as_str)
        })
        .collect::<Vec<_>>();
    assert!(
        supervision_paths.contains(&"docs/supervised-pc-project-development.md"),
        "{supervision_paths:?}"
    );

    let bounded = plan_context(
        &root,
        "Codex 桌面通过 PC 节点协作开发与项目文档治理",
        None,
        2_400,
        10,
        1_600,
    )
    .unwrap();
    let bounded_paths = bounded["relevant_documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| {
            item.pointer("/document/path")
                .and_then(serde_json::Value::as_str)
        })
        .collect::<Vec<_>>();
    assert!(
        bounded_paths.contains(&"docs/supervised-pc-project-development.md"),
        "{bounded_paths:?}"
    );
    assert!(
        !bounded_paths.contains(&"docs/windows-client-defender.md"),
        "{bounded_paths:?}"
    );
    assert!(
        bounded["budget"]["relevant_content"]["estimated_tokens_selected"]
            .as_u64()
            .unwrap()
            <= 2_400
    );
    assert_eq!(bounded["budget"]["markdown_bodies_read"], 0);
    assert!(
        elapsed.as_secs() < 10,
        "self project overview took {elapsed:?}"
    );
}
