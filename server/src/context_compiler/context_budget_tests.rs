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
fn saves_context_budget_sidecars_for_task_pack() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "elon_context_budget_{}_{}",
        std::process::id(),
        nonce
    ));
    let config = test_config();
    let snapshot = RepoSnapshot {
        git_head: Some("abc123".to_string()),
        git_branch: Some("main".to_string()),
        git_dirty: false,
        git_status_short: Vec::new(),
        has_origin: true,
        top_level_entries: Vec::new(),
        instruction_docs: Vec::new(),
        manifests: Vec::new(),
        large_files: Vec::new(),
        source_file_count: 0,
    };
    let validation = ValidationPlan {
        commands: Vec::new(),
        notes: Vec::new(),
    };
    let pack = r#"<task_context_pack version="1">
<instructions>
read real files first
</instructions>

<repo_map>
- server/src/context_compiler
</repo_map>

<symbol_graph>
- build_context_pack -> save_context_artifacts
</symbol_graph>

<relevant_files>
<file path="server/src/context_compiler/artifact.rs">
```rust
fn save_context_artifacts() {}
```
</file>
</relevant_files>
</task_context_pack>"#;

    let artifact = save_context_artifacts(ContextArtifactsInput {
        data_dir: &dir,
        config: &config,
        trace_id: Some("trace/budget"),
        user_id: "user@example",
        user_message: "完善 token budget",
        pack,
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

    let budget_json = fs::read_to_string(artifact.bundle_dir.join("context_budget.json")).unwrap();
    assert!(budget_json.contains("\"name\": \"repo_map\""));
    assert!(budget_json.contains("\"name\": \"symbol_graph\""));
    assert!(budget_json.contains("\"group\": \"full_source\""));

    let budget_md = fs::read_to_string(artifact.bundle_dir.join("context_budget.md")).unwrap();
    assert!(budget_md.contains("# Context Budget"));
    assert!(budget_md.contains("| full_source |"));

    let current_task_json = fs::read_to_string(
        artifact
            .bundle_dir
            .join(".ai")
            .join("context")
            .join("current-task.json"),
    )
    .unwrap();
    assert!(current_task_json.contains("\"context_budget\": \"context_budget.json\""));

    let manifest = fs::read_to_string(artifact.bundle_dir.join("manifest.json")).unwrap();
    assert!(manifest.contains("\"context_budget.json\""));
    assert!(manifest.contains("\"context_budget.md\""));

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
