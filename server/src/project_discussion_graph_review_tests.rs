use super::{prepare_safe_discussion_repair, review_discussion_graph};
use crate::project_discussion_graph_model::{
    DiscussionGraph, DiscussionNode, DiscussionSource, DISCUSSION_GRAPH_PATH,
};
use std::fs;

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
    let repair = prepare_safe_discussion_repair(&root).unwrap();
    assert_eq!(repair["safe_repair_count"], 2);
    assert_eq!(
        repair["proposal"]["graph"]["nodes"][1]["root_id"],
        serde_json::json!("root")
    );
    fs::remove_dir_all(root).unwrap();
}
