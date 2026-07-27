use super::{
    compare_discussion_versions, list_discussion_versions, load_discussion_graph_version,
    trace_discussion_node,
};
use crate::project_discussion_graph_model::{
    DiscussionGraph, DiscussionNode, DiscussionSource, DISCUSSION_GRAPH_PATH,
};
use std::{fs, path::Path};

#[test]
fn exposes_semantic_history_diff_and_node_trace_without_reading_sources() {
    let root = std::env::temp_dir().join(format!(
        "elon_discussion_history_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".elon")).unwrap();
    git(&root, &["init", "-q"]);
    let first = graph("open", "最初方案");
    fs::write(
        root.join(DISCUSSION_GRAPH_PATH),
        serde_json::to_vec_pretty(&first).unwrap(),
    )
    .unwrap();
    let first_commit = commit(&root, "docs: 保存第一版讨论图");
    let second = graph("accepted", "确认方案");
    fs::write(
        root.join(DISCUSSION_GRAPH_PATH),
        serde_json::to_vec_pretty(&second).unwrap(),
    )
    .unwrap();
    let second_commit = commit(&root, "docs: 确认讨论结论");

    let history = list_discussion_versions(&root, 10).unwrap();
    assert_eq!(history["versions"].as_array().unwrap().len(), 2);
    assert_eq!(history["budget"]["chat_bodies_read"], 0);
    let snapshot = load_discussion_graph_version(&root, &first_commit).unwrap();
    assert_eq!(snapshot.graph.nodes[0].status, "open");
    let diff = compare_discussion_versions(&root, &first_commit, Some(&second_commit)).unwrap();
    assert_eq!(diff["counts"]["nodes_changed"], 1);
    assert_eq!(
        diff["nodes"]["changed"][0]["from_status"],
        serde_json::json!("open")
    );
    let trace = trace_discussion_node(&root, "root", 20).unwrap();
    assert!(trace["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["to_status"] == "accepted"));
    fs::remove_dir_all(root).unwrap();
}

fn graph(status: &str, summary: &str) -> DiscussionGraph {
    DiscussionGraph {
        sources: vec![DiscussionSource {
            id: "chat".into(),
            title: "原始讨论".into(),
            reference: "docs/inbox/conversations/chat.md".into(),
            ..Default::default()
        }],
        nodes: vec![DiscussionNode {
            id: "root".into(),
            root_id: "root".into(),
            kind: "decision".into(),
            title: "开放商业网络".into(),
            summary: summary.into(),
            status: status.into(),
            source_refs: vec!["chat#1".into()],
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn commit(root: &Path, message: &str) -> String {
    git(root, &["add", DISCUSSION_GRAPH_PATH]);
    git(
        root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-q",
            "-m",
            message,
        ],
    );
    let output = crate::git_command_error::git_command()
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn git(root: &Path, args: &[&str]) {
    assert!(crate::git_command_error::git_command()
        .current_dir(root)
        .args(args)
        .status()
        .unwrap()
        .success());
}
