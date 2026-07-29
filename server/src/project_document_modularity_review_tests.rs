use super::*;
use std::fs;

#[test]
fn keeps_large_discussions_as_sources_but_suggests_splitting_current_specs() {
    let root =
        std::env::temp_dir().join(format!("elon_modularity_{}", uuid::Uuid::new_v4().simple()));
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join(".elon")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/spec.md"),
        format!("# 规格\n\n## A\n\n{}\n", "正文\n".repeat(150)),
    )
    .unwrap();
    fs::write(
        root.join("docs/discussion.md"),
        format!("# 讨论\n\n## A\n\n{}\n", "记录\n".repeat(150)),
    )
    .unwrap();
    fs::write(
        root.join(".elon/document-sections.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "version":1,
            "governance_facets":{
                "docs/spec.md":{"retrieval":"on_demand","lifecycle":"active","authority":"authoritative","document_type":"spec"},
                "docs/discussion.md":{"retrieval":"excluded","lifecycle":"source_material","authority":"evidence","document_type":"discussion"}
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let review = review_document_modularity(
        &root,
        &["docs/spec.md".to_string(), "docs/discussion.md".to_string()],
        100,
        8_000,
        8,
    )
    .unwrap();
    let findings = review["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 2);
    assert!(findings.iter().any(|finding| {
        finding["path"] == "docs/spec.md"
            && finding["recommendation"] == "create_package_index_and_split_by_responsibility"
    }));
    assert!(findings.iter().any(|finding| {
        finding["path"] == "docs/discussion.md"
            && finding["recommendation"] == "retain_source_material_and_compile"
    }));
    fs::remove_dir_all(root).ok();
}
