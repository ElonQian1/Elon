use super::{prepare_safe_discussion_repair, review_discussion_graph, review_sources};
use crate::project_discussion_graph_model::{
    DiscussionGraph, DiscussionNode, DiscussionSource, DISCUSSION_GRAPH_PATH,
};
use std::{fs, path::PathBuf};

#[test]
fn finds_semantic_quality_issues_and_only_prepares_deterministic_repairs() {
    let root = std::env::temp_dir().join(format!(
        "elon_discussion_review_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".elon")).unwrap();
    let graph = DiscussionGraph {
        sources: vec![DiscussionSource {
            id: "chat".into(),
            title: "已丢失聊天".into(),
            reference: "docs/inbox/conversations/missing.md".into(),
            ..Default::default()
        }],
        nodes: vec![
            DiscussionNode {
                id: "root".into(),
                kind: "topic".into(),
                title: "开放商业网络".into(),
                source_refs: vec!["chat#1".into()],
                ..Default::default()
            },
            DiscussionNode {
                id: "decision".into(),
                root_id: "decision".into(),
                parent_id: "root".into(),
                kind: "decision".into(),
                title: "开放协议".into(),
                status: "accepted".into(),
                authority: "source".into(),
                source_refs: vec!["chat#section-2".into()],
                document_paths: vec!["docs/missing-decision.md".into()],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    fs::write(
        root.join(DISCUSSION_GRAPH_PATH),
        serde_json::to_vec_pretty(&graph).unwrap(),
    )
    .unwrap();

    let report = review_discussion_graph(&root).unwrap();
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.rule == "node.root_mismatch" && issue.auto_fixable));
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.rule == "node.authority_mismatch" && !issue.auto_fixable));
    assert!(report.issues.iter().any(|issue| {
        issue.rule == "source.compilation_incomplete" && issue.severity == "error"
    }));
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.rule == "source.revision_missing"));
    let repair = prepare_safe_discussion_repair(&root).unwrap();
    assert_eq!(repair["safe_repair_count"], 2);
    assert_eq!(
        repair["proposal"]["graph"]["nodes"][1]["root_id"],
        serde_json::json!("root")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn finds_unknown_sources_and_unprocessed_chunk_anchors() {
    let root = std::env::temp_dir().join(format!(
        "elon_discussion_source_review_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".elon")).unwrap();
    fs::create_dir_all(root.join("docs/inbox/conversations")).unwrap();
    fs::write(
        root.join("docs/inbox/conversations/source.md"),
        "# 已登记来源\n",
    )
    .unwrap();
    let graph = DiscussionGraph {
        sources: vec![DiscussionSource {
            id: "source".into(),
            title: "已登记来源".into(),
            reference: "docs/inbox/conversations/source.md".into(),
            content_revision: "revision".into(),
            chunk_count: 1,
            processed_chunk_ids: vec!["chunk-0001".into()],
            compilation_status: "complete".into(),
            ..Default::default()
        }],
        nodes: vec![
            DiscussionNode {
                id: "unknown".into(),
                kind: "topic".into(),
                title: "未知来源".into(),
                source_refs: vec!["missing#chunk-0001".into()],
                ..Default::default()
            },
            DiscussionNode {
                id: "unprocessed".into(),
                kind: "topic".into(),
                title: "未处理分块".into(),
                source_refs: vec!["source#chunk-0002".into()],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let mut issues = Vec::new();
    review_sources(&root, &graph, &mut issues);
    assert!(issues
        .iter()
        .any(|issue| issue.rule == "source.reference_unknown"));
    assert!(issues
        .iter()
        .any(|issue| issue.rule == "source.anchor_unprocessed"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn repository_discussion_sources_are_complete_and_traceable() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .expect("server must live below repository root");
    let report = review_discussion_graph(root).unwrap();
    let source_errors = report
        .issues
        .iter()
        .filter(|issue| {
            matches!(
                issue.rule,
                "source.reference_unknown"
                    | "source.reference_missing"
                    | "source.compilation_incomplete"
                    | "source.revision_missing"
                    | "source.anchor_unprocessed"
            )
        })
        .map(|issue| format!("{}: {}", issue.rule, issue.detail))
        .collect::<Vec<_>>();
    assert!(
        source_errors.is_empty(),
        "discussion source errors:\n{}",
        source_errors.join("\n")
    );
}
