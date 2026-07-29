use super::{test_document_retrieval, RetrievalAcceptanceCase, RETRIEVAL_CASES_PATH};
use serde_json::json;
use std::{fs, path::PathBuf};

fn fixture(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_retrieval_acceptance_{label}_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join(".elon")).unwrap();
    fs::create_dir_all(root.join("docs/drafts")).unwrap();
    fs::write(
        root.join("docs/current.md"),
        "# 当前能力\n\n已实现的开放商业能力。\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/drafts/future.md"),
        "# 未来提案\n\n尚未实现的开放商业能力。\n",
    )
    .unwrap();
    fs::write(
        root.join(".elon/document-sections.json"),
        serde_json::to_vec_pretty(&json!({
            "version": 1,
            "home": {
                "title": "测试",
                "entrypoint": "docs/current.md",
                "start_here": ["docs/current.md"]
            },
            "sections": [{
                "id": "commerce",
                "label": "开放商业",
                "detail": "能力",
                "color": "#57A6C7",
                "entrypoint": "docs/current.md"
            }],
            "assignments": {
                "docs/current.md": "custom:commerce",
                "docs/drafts/future.md": "custom:commerce"
            },
            "governance_facets": {
                "docs/current.md": {
                    "retrieval": "on_demand",
                    "lifecycle": "active",
                    "authority": "authoritative",
                    "document_type": "capability_baseline"
                },
                "docs/drafts/future.md": {
                    "retrieval": "excluded",
                    "lifecycle": "draft",
                    "authority": "non_authoritative",
                    "document_type": "proposal"
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();
    root
}

#[test]
fn runs_inline_retrieval_cases_without_reading_markdown_bodies() {
    let root = fixture("inline");
    let result = test_document_retrieval(
        &root,
        Some(vec![RetrievalAcceptanceCase {
            id: "current".into(),
            query: "请读取 docs/current.md 的当前能力".into(),
            node_id: None,
            expected_paths: vec!["docs/current.md".into()],
            forbidden_paths: vec!["docs/drafts/future.md".into()],
            require_first: Some("docs/current.md".into()),
        }]),
        1_000,
        4,
    )
    .unwrap();
    assert_eq!(result.pointer("/summary/success"), Some(&json!(true)));
    assert_eq!(
        result.pointer("/budget/markdown_bodies_read"),
        Some(&json!(0))
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn loads_versioned_cases_from_project_manifest() {
    let root = fixture("manifest");
    fs::write(
        root.join(RETRIEVAL_CASES_PATH),
        serde_json::to_vec_pretty(&json!({
            "version": 1,
            "cases": [{
                "id": "future-draft",
                "query": "核对 docs/drafts/future.md",
                "expected_paths": ["docs/drafts/future.md"],
                "forbidden_paths": []
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    let result = test_document_retrieval(&root, None, 1_000, 4).unwrap();
    assert_eq!(result["source"], RETRIEVAL_CASES_PATH);
    assert_eq!(result.pointer("/summary/passed"), Some(&json!(1)));
    fs::remove_dir_all(root).ok();
}

#[test]
fn repository_retrieval_cases_remain_valid() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .expect("server must live below repository root");
    let result = test_document_retrieval(root, None, 3_000, 8).unwrap();
    assert_eq!(
        result.pointer("/summary/success"),
        Some(&json!(true)),
        "{}",
        serde_json::to_string_pretty(&result).unwrap()
    );
}
