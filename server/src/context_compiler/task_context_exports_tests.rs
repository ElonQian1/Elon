use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    artifact::{save_context_artifacts, ContextArtifactsInput},
    config::{ContextCompilerConfig, ContextCompilerMode},
    repo_snapshot::RepoSnapshot,
    validation::ValidationPlan,
};

#[test]
fn saves_task_context_pack_and_harness_current_task_exports() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "elon_task_context_exports_{}_{}",
        std::process::id(),
        nonce
    ));
    let config = test_config();
    let snapshot = RepoSnapshot {
        git_head: Some("abc123".to_string()),
        git_branch: Some("main".to_string()),
        git_dirty: true,
        git_status_short: vec![" M server/src/context_compiler/artifact.rs".to_string()],
        has_origin: true,
        top_level_entries: Vec::new(),
        instruction_docs: Vec::new(),
        manifests: Vec::new(),
        large_files: Vec::new(),
        source_file_count: 0,
    };
    let validation = ValidationPlan {
        commands: Vec::new(),
        notes: vec!["read source first".to_string()],
    };

    let artifact = save_context_artifacts(ContextArtifactsInput {
        data_dir: &dir,
        config: &config,
        trace_id: Some("trace/current-task"),
        user_id: "user@example",
        user_message: "完善 repo map",
        pack: "<task_context_pack>hello</task_context_pack>",
        llm_brief: None,
        snapshot: &snapshot,
        rust_project: None,
        project_manifests: None,
        directory_summaries: &[],
        repo_index: None,
        relevant_files: &[],
        validation_plan: &validation,
    })
    .unwrap();

    let task_pack = artifact.bundle_dir.join("task_context_pack.md");
    let current_md = artifact
        .bundle_dir
        .join(".ai")
        .join("context")
        .join("current-task.md");
    let current_json = artifact
        .bundle_dir
        .join(".ai")
        .join("context")
        .join("current-task.json");
    assert_eq!(
        fs::read_to_string(task_pack).unwrap(),
        "<task_context_pack>hello</task_context_pack>"
    );
    assert_eq!(
        fs::read_to_string(current_md).unwrap(),
        "<task_context_pack>hello</task_context_pack>"
    );

    let json = fs::read_to_string(current_json).unwrap();
    assert!(json.contains("\"user_request\": \"完善 repo map\""));
    assert!(json.contains("\"harness_markdown\": \".ai/context/current-task.md\""));
    assert!(json.contains("\"dirty\": true"));

    let manifest = fs::read_to_string(artifact.bundle_dir.join("manifest.json")).unwrap();
    assert!(manifest.contains("\"task_context_pack.md\""));
    assert!(manifest.contains("\".ai/context/current-task.json\""));

    fs::remove_dir_all(dir).unwrap();
}

fn test_config() -> ContextCompilerConfig {
    ContextCompilerConfig {
        enabled: true,
        mode: ContextCompilerMode::Shadow,
        agent_name: "hunyuan".to_string(),
        llm_brief_enabled: false,
        rust_analysis_enabled: true,
        rust_analyzer_enabled: true,
        rust_analyzer_probe_enabled: false,
        rust_analyzer_probe_timeout_ms: 4_000,
        rust_analyzer_lsp_enabled: false,
        rust_analyzer_lsp_timeout_ms: 6_000,
        rust_analyzer_lsp_max_queries: 16,
        max_relevant_files: 4,
        max_rust_files: 40,
        max_symbols: 20,
        max_relationships: 20,
        max_rust_analyzer_files: 2,
        max_pack_chars: 20_000,
        save_pack_enabled: true,
        artifact_max_bytes: 100_000,
        rust_probe_enabled: true,
    }
}
