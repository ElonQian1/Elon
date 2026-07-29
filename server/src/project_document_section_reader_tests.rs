use super::*;
use std::fs;

#[test]
fn reads_one_heading_without_returning_the_whole_document() {
    let root = std::env::temp_dir().join(format!(
        "elon_section_reader_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/plan.md"),
        "# 计划\n\n## 第一阶段\n\n入口。\n\n### 验收\n\n通过。\n\n## 第二阶段\n\n未开始。\n",
    )
    .unwrap();
    let value = read_document_sections(
        &root,
        &[SectionReadRequest {
            path: "docs/plan.md".to_string(),
            heading: "第一阶段".to_string(),
            include_children: true,
        }],
        6_000,
        None,
    )
    .unwrap();
    let content = value["sections"][0]["content"].as_str().unwrap();
    assert!(content.contains("### 验收"));
    assert!(!content.contains("第二阶段"));
    assert_eq!(value["sections"][0]["start_line"], 3);
    fs::remove_dir_all(root).ok();
}
