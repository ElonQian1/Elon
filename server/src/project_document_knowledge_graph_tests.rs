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

    let plan = plan_context(&root, "健康检查", Some("cap-api"), 1_000, 4).unwrap();
    assert_eq!(plan["budget"]["markdown_bodies_read"], 0);
    assert_eq!(plan["documents"][0]["document"]["path"], "docs/API.md");
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
    assert!(
        elapsed.as_secs() < 10,
        "self project overview took {elapsed:?}"
    );
}
