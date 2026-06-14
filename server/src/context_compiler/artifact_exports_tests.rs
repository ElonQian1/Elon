use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    artifact::{save_context_artifacts, ContextArtifactsInput},
    config::{ContextCompilerConfig, ContextCompilerMode},
    model::{
        BuildCommand, CodeRelationship, ContextEvidence, EvidenceSnippet, RankedFile, RankedSymbol,
        RelationshipKind, RepoContextIndex, RustAnalyzerLspLocation, RustAnalyzerLspLocationRole,
        RustAnalyzerLspQueryResult, RustAnalyzerLspReport, RustAnalyzerLspStatus,
        RustAnalyzerReport, RustIndex, RustSymbol, SemanticQueryMethod, SymbolGraphSummary,
        SymbolKind, SymbolVisibility, TaskProfile, TestTarget,
    },
    repo_snapshot::RepoSnapshot,
    validation::{ValidationCommand, ValidationPlan},
};

#[test]
fn saves_repo_map_projection_exports_when_repo_index_exists() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "elon_context_exports_{}_{}",
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
        commands: vec![ValidationCommand {
            command: "cargo test --manifest-path server/Cargo.toml context_compiler".to_string(),
            reason: "context compiler export changed".to_string(),
            required: true,
        }],
        notes: Vec::new(),
    };
    let repo_index = test_repo_index();

    let artifact = save_context_artifacts(ContextArtifactsInput {
        data_dir: &dir,
        config: &config,
        trace_id: Some("trace/exports"),
        user_id: "user@example",
        user_message: "repo map exports",
        pack: "hello",
        llm_brief: None,
        snapshot: &snapshot,
        rust_project: None,
        repo_index: Some(&repo_index),
        relevant_files: &[],
        validation_plan: &validation,
    })
    .unwrap();

    assert!(artifact.bundle_dir.join("repo_map.md").is_file());
    assert!(artifact.bundle_dir.join("summaries.md").is_file());
    assert!(artifact.bundle_dir.join("symbols.jsonl").is_file());
    assert!(artifact.bundle_dir.join("edges.tsv").is_file());
    assert!(artifact.bundle_dir.join("chunks.jsonl").is_file());
    assert!(artifact.bundle_dir.join("tests.jsonl").is_file());
    assert!(artifact.bundle_dir.join("lsp_locations.jsonl").is_file());
    assert!(artifact.bundle_dir.join("semantic_facts.jsonl").is_file());

    assert!(fs::read_to_string(artifact.bundle_dir.join("repo_map.md"))
        .unwrap()
        .contains("Ranked Files"));
    assert!(
        fs::read_to_string(artifact.bundle_dir.join("symbols.jsonl"))
            .unwrap()
            .contains("build_context_pack")
    );
    assert!(fs::read_to_string(artifact.bundle_dir.join("edges.tsv"))
        .unwrap()
        .contains("symbol_graph"));
    assert!(fs::read_to_string(artifact.bundle_dir.join("chunks.jsonl"))
        .unwrap()
        .contains("context_evidence"));
    let tests_jsonl = fs::read_to_string(artifact.bundle_dir.join("tests.jsonl")).unwrap();
    assert!(tests_jsonl.contains("\"test_kind\":\"test_target\""));
    assert!(tests_jsonl.contains("\"test_kind\":\"validation_command\""));
    assert!(tests_jsonl.contains("artifact_exports_tests.rs"));
    assert!(
        fs::read_to_string(artifact.bundle_dir.join("lsp_locations.jsonl"))
            .unwrap()
            .contains("\"role\":\"reference\"")
    );
    assert!(
        fs::read_to_string(artifact.bundle_dir.join("lsp_locations.jsonl"))
            .unwrap()
            .contains("\"role\":\"definition\"")
    );
    let semantic_facts =
        fs::read_to_string(artifact.bundle_dir.join("semantic_facts.jsonl")).unwrap();
    assert!(semantic_facts.contains("\"fact_kind\":\"references\""));
    assert!(semantic_facts.contains("\"fact_kind\":\"definitions\""));
    assert!(semantic_facts.contains("\"fact_kind\":\"hover_type\""));
    assert!(semantic_facts.contains("pub(crate) fn build_context_pack"));
    let manifest = fs::read_to_string(artifact.bundle_dir.join("manifest.json")).unwrap();
    assert!(manifest.contains("tests.jsonl"));
    assert!(manifest.contains("semantic_facts.jsonl"));

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

fn test_repo_index() -> RepoContextIndex {
    RepoContextIndex {
        task: TaskProfile {
            keywords: vec!["repo".to_string(), "map".to_string()],
            ..TaskProfile::default()
        },
        rust: RustIndex {
            files_scanned: 1,
            symbols: vec![RustSymbol {
                id: "server/src/context_compiler/context_pack.rs::build_context_pack".to_string(),
                name: "build_context_pack".to_string(),
                kind: SymbolKind::Function,
                path: "server/src/context_compiler/context_pack.rs".to_string(),
                line_start: 10,
                line_end: 60,
                visibility: SymbolVisibility::Crate,
                signature: "pub(crate) fn build_context_pack(...) -> String".to_string(),
                parent: None,
                docs: None,
                role: "caller",
                safety_notes: Vec::new(),
            }],
            warnings: Vec::new(),
        },
        graph: SymbolGraphSummary {
            ranked_files: vec![RankedFile {
                path: "server/src/context_compiler/context_pack.rs".to_string(),
                role: "source",
                score: 9.5,
                symbol_count: 1,
                top_symbols: vec!["build_context_pack".to_string()],
                reasons: vec!["test ranked file".to_string()],
            }],
            ranked_symbols: vec![RankedSymbol {
                id: "server/src/context_compiler/context_pack.rs::build_context_pack".to_string(),
                name: "build_context_pack".to_string(),
                kind: SymbolKind::Function,
                path: "server/src/context_compiler/context_pack.rs".to_string(),
                line_start: 10,
                line_end: 60,
                score: 9.0,
                reasons: vec!["test ranked symbol".to_string()],
            }],
            relationships: vec![CodeRelationship {
                from_path: "server/src/context_compiler/mod.rs".to_string(),
                to_symbol_id: "server/src/context_compiler/context_pack.rs::build_context_pack"
                    .to_string(),
                to_symbol_name: "build_context_pack".to_string(),
                to_path: "server/src/context_compiler/context_pack.rs".to_string(),
                kind: RelationshipKind::CallsOrMentions,
                line: 42,
                reason: "test relationship".to_string(),
            }],
            ..SymbolGraphSummary::default()
        },
        rust_analyzer: RustAnalyzerReport {
            lsp: RustAnalyzerLspReport {
                enabled: true,
                attempted: 3,
                succeeded: 3,
                results: vec![
                    RustAnalyzerLspQueryResult {
                        method: SemanticQueryMethod::Definition,
                        path: "server/src/context_compiler/context_pack.rs".to_string(),
                        line: 10,
                        symbol: Some("build_context_pack".to_string()),
                        status: RustAnalyzerLspStatus::Succeeded,
                        duration_ms: 1,
                        summary: Some("1 item(s)".to_string()),
                        locations: vec![RustAnalyzerLspLocation {
                            role: RustAnalyzerLspLocationRole::Definition,
                            path: "server/src/context_compiler/context_pack.rs".to_string(),
                            line: 10,
                            end_line: None,
                            symbol: Some("build_context_pack".to_string()),
                        }],
                        warning: None,
                    },
                    RustAnalyzerLspQueryResult {
                        method: SemanticQueryMethod::References,
                        path: "server/src/context_compiler/context_pack.rs".to_string(),
                        line: 10,
                        symbol: Some("build_context_pack".to_string()),
                        status: RustAnalyzerLspStatus::Succeeded,
                        duration_ms: 1,
                        summary: Some("1 item(s)".to_string()),
                        locations: vec![RustAnalyzerLspLocation {
                            role: RustAnalyzerLspLocationRole::Reference,
                            path: "server/src/context_compiler/mod.rs".to_string(),
                            line: 42,
                            end_line: None,
                            symbol: Some("build_context_pack".to_string()),
                        }],
                        warning: None,
                    },
                    RustAnalyzerLspQueryResult {
                        method: SemanticQueryMethod::Hover,
                        path: "server/src/context_compiler/context_pack.rs".to_string(),
                        line: 10,
                        symbol: Some("build_context_pack".to_string()),
                        status: RustAnalyzerLspStatus::Succeeded,
                        duration_ms: 1,
                        summary: Some(
                            "pub(crate) fn build_context_pack(...) -> String".to_string(),
                        ),
                        locations: Vec::new(),
                        warning: None,
                    },
                ],
                ..RustAnalyzerLspReport::default()
            },
            ..RustAnalyzerReport::default()
        },
        evidence: ContextEvidence {
            snippets: vec![EvidenceSnippet {
                id: "S1".to_string(),
                path: "server/src/context_compiler/context_pack.rs".to_string(),
                role: "edit-target",
                symbols: vec![
                    "server/src/context_compiler/context_pack.rs::build_context_pack".to_string(),
                ],
                line_start: 10,
                line_end: 20,
                sha256: "abc".to_string(),
                reason: "context_evidence test".to_string(),
                content: "pub(crate) fn build_context_pack() {}".to_string(),
            }],
            test_targets: vec![TestTarget {
                path: "server/src/context_compiler/artifact_exports_tests.rs".to_string(),
                reason: "export projection coverage".to_string(),
            }],
            build_commands: vec![BuildCommand {
                command: "cargo test --manifest-path server/Cargo.toml artifact_exports"
                    .to_string(),
                reason: "verify projection sidecars".to_string(),
            }],
            ..ContextEvidence::default()
        },
        ..RepoContextIndex::default()
    }
}
