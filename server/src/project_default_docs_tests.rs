use super::*;
use serde::Deserialize;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct DefaultDocsManifest {
    documents: Vec<DefaultDocsManifestDocument>,
}

#[derive(Deserialize)]
struct DefaultDocsManifestDocument {
    path: String,
}

#[test]
fn default_docs_seed_missing_files_without_overwriting_user_docs() {
    let root = std::env::temp_dir().join(format!(
        "elon-default-project-docs-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("AGENTS.md"), "# User Rules\nkeep me").unwrap();

    let created = ensure_default_docs_in_workspace(&root).unwrap();
    let agents = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
    let codex = std::fs::read_to_string(root.join("CODEX.md")).unwrap();
    let copilot = std::fs::read_to_string(root.join(".github/copilot-instructions.md")).unwrap();
    let document_authority = std::fs::read_to_string(
        root.join(".github/instructions/document-authority.instructions.md"),
    )
    .unwrap();
    let discussion_knowledge = std::fs::read_to_string(
        root.join(".github/instructions/discussion-knowledge.instructions.md"),
    )
    .unwrap();
    let metadata = std::fs::read_to_string(root.join(".elon/default-docs.json")).unwrap();
    let knowledge_architecture =
        std::fs::read_to_string(root.join(".elon/document-sections.json")).unwrap();
    let _ = std::fs::remove_dir_all(&root);

    assert!(created > 0);
    assert_eq!(agents, "# User Rules\nkeep me");
    assert!(codex.contains(".github/copilot-instructions.md"));
    assert!(copilot.contains("共享规则权威来源"));
    assert!(document_authority.contains("路径权威上限"));
    assert!(discussion_knowledge.contains("每次应用都创建新版本"));
    assert!(metadata.contains("copilot-primary-bridged-agents"));
    assert!(knowledge_architecture.contains("项目知识库"));
    assert!(knowledge_architecture.contains("document_metadata"));
}

#[test]
fn default_docs_manifest_matches_seeded_files() {
    let manifest: DefaultDocsManifest = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../default-project-docs/files/elon/default-docs.json"
    )))
    .unwrap();
    let manifest_paths = manifest
        .documents
        .into_iter()
        .map(|doc| doc.path)
        .collect::<HashSet<_>>();
    let seeded_paths = DEFAULT_PROJECT_FILES
        .iter()
        .map(|doc| doc.path.to_string())
        .collect::<HashSet<_>>();

    assert_eq!(manifest_paths, seeded_paths);
}
