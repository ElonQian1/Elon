use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    artifact::{save_context_artifacts, ContextArtifactsInput},
    config::{ContextCompilerConfig, ContextCompilerMode},
    directory_summary::{DirectoryRoleCount, DirectorySummary},
    model::{
        BuildCommand, CodeRelationship, ContextEvidence, EvidenceSnippet, RankedFile, RankedSymbol,
        RelationshipKind, RepoContextIndex, RustAnalyzerLspLocation, RustAnalyzerLspLocationRole,
        RustAnalyzerLspQueryResult, RustAnalyzerLspReport, RustAnalyzerLspStatus,
        RustAnalyzerReport, RustIndex, RustSymbol, SemanticQueryMethod, SymbolGraphSummary,
        SymbolKind, SymbolVisibility, TaskProfile, TestTarget,
    },
    project_manifests::{ProjectManifestReport, ProjectManifestSummary, ReadmeSummary},
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
    let project_manifests = ProjectManifestReport {
        readmes: vec![ReadmeSummary {
            path: "README.md".to_string(),
            title: Some("Demo".to_string()),
            headings: vec!["Demo".to_string()],
            preview: Some("A demo project.".to_string()),
        }],
        manifests: vec![ProjectManifestSummary {
            path: "package.json".to_string(),
            kind: "package_json",
            name: Some("web".to_string()),
            version: Some("1.0.0".to_string()),
            description: None,
            scripts: vec!["test".to_string()],
            dependencies: vec!["react".to_string()],
            features: Vec::new(),
        }],
        warnings: Vec::new(),
    };
    let directory_summaries = vec![DirectorySummary {
        path: ".".to_string(),
        direct_files: 2,
        subtree_source_files: 1,
        subtree_lines: 42,
        role_counts: vec![DirectoryRoleCount {
            role: "entrypoint".to_string(),
            files: 1,
        }],
        key_files: vec!["README.md".to_string()],
        child_directories: vec!["server".to_string()],
    }];

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
        project_manifests: Some(&project_manifests),
        directory_summaries: &directory_summaries,
        repo_index: Some(&repo_index),
        relevant_files: &[],
        validation_plan: &validation,
    })
    .unwrap();

    assert!(artifact.bundle_dir.join("repo_map.md").is_file());
    assert!(artifact.bundle_dir.join("summaries.md").is_file());
    assert!(artifact.bundle_dir.join("project_manifests.md").is_file());
    assert!(artifact.bundle_dir.join("project_manifests.json").is_file());
    assert!(artifact.bundle_dir.join("directory_summaries.md").is_file());
    assert!(artifact
        .bundle_dir
        .join("directory_summaries.json")
        .is_file());
    assert!(artifact.bundle_dir.join("directories.jsonl").is_file());
    assert!(artifact.bundle_dir.join("symbols.jsonl").is_file());
    assert!(artifact.bundle_dir.join("symbol_index.jsonl").is_file());
    assert!(artifact.bundle_dir.join("symbol_edges.jsonl").is_file());
    assert!(artifact.bundle_dir.join("symbol_lookup.json").is_file());
    assert!(artifact.bundle_dir.join("symbol_index.sqlite").is_file());
    assert!(artifact.bundle_dir.join("edges.tsv").is_file());
    assert!(artifact.bundle_dir.join("chunks.jsonl").is_file());
    assert!(artifact.bundle_dir.join("tests.jsonl").is_file());
    assert!(artifact.bundle_dir.join("lsp_locations.jsonl").is_file());
    assert!(artifact.bundle_dir.join("semantic_facts.jsonl").is_file());

    assert!(fs::read_to_string(artifact.bundle_dir.join("repo_map.md"))
        .unwrap()
        .contains("Ranked Files"));
    assert!(
        fs::read_to_string(artifact.bundle_dir.join("project_manifests.md"))
            .unwrap()
            .contains("package.json")
    );
    assert!(
        fs::read_to_string(artifact.bundle_dir.join("directory_summaries.md"))
            .unwrap()
            .contains("subtree source files")
    );
    assert!(
        fs::read_to_string(artifact.bundle_dir.join("directories.jsonl"))
            .unwrap()
            .contains("\"path\":\".\"")
    );
    assert!(
        fs::read_to_string(artifact.bundle_dir.join("symbols.jsonl"))
            .unwrap()
            .contains("build_context_pack")
    );
    let symbol_index = fs::read_to_string(artifact.bundle_dir.join("symbol_index.jsonl")).unwrap();
    assert!(symbol_index.contains("\"qualified_name\""));
    assert!(symbol_index.contains("build_context_pack"));
    assert!(
        fs::read_to_string(artifact.bundle_dir.join("symbol_edges.jsonl"))
            .unwrap()
            .contains("\"source\":\"rust_analyzer_lsp\"")
    );
    let symbol_lookup = fs::read_to_string(artifact.bundle_dir.join("symbol_lookup.json")).unwrap();
    assert!(symbol_lookup.contains("search_symbols"));
    assert!(symbol_lookup.contains("symbol_count"));
    let symbol_db = artifact.bundle_dir.join("symbol_index.sqlite");
    let conn = rusqlite::Connection::open(&symbol_db).unwrap();
    let symbol_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM symbols", [], |row| row.get(0))
        .unwrap();
    assert!(symbol_count >= 1);
    let stored_symbol_name: String = conn
        .query_row(
            "SELECT name FROM symbols WHERE id = ?1",
            ["server/src/context_compiler/context_pack.rs::build_context_pack"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_symbol_name, "build_context_pack");
    let edge_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .unwrap();
    assert!(edge_count >= 3);
    let lookup_summary: String = conn
        .query_row(
            "SELECT value FROM metadata WHERE key = 'lookup_summary_json'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(lookup_summary.contains("workspace_symbols_named"));
    let term_hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM symbol_terms WHERE term = 'build_context_pack'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(term_hits > 0);
    let lsp_edges: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE source = 'rust_analyzer_lsp'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(lsp_edges > 0);
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
    assert!(
        fs::read_to_string(artifact.bundle_dir.join("lsp_locations.jsonl"))
            .unwrap()
            .contains("\"role\":\"workspace_symbol\"")
    );
    let semantic_facts =
        fs::read_to_string(artifact.bundle_dir.join("semantic_facts.jsonl")).unwrap();
    assert!(semantic_facts.contains("\"fact_kind\":\"references\""));
    assert!(semantic_facts.contains("\"fact_kind\":\"workspace_symbols\""));
    assert!(semantic_facts.contains("\"fact_kind\":\"definitions\""));
    assert!(semantic_facts.contains("\"fact_kind\":\"hover_type\""));
    assert!(semantic_facts.contains("pub(crate) fn build_context_pack"));
    let manifest = fs::read_to_string(artifact.bundle_dir.join("manifest.json")).unwrap();
    assert!(manifest.contains("tests.jsonl"));
    assert!(manifest.contains("project_manifests.md"));
    assert!(manifest.contains("directory_summaries.md"));
    assert!(manifest.contains("directories.jsonl"));
    assert!(manifest.contains("symbol_index.jsonl"));
    assert!(manifest.contains("symbol_edges.jsonl"));
    assert!(manifest.contains("symbol_lookup.json"));
    assert!(manifest.contains("symbol_index.sqlite"));
    assert!(manifest.contains("semantic_facts.jsonl"));

    drop(conn);
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
            ..RustIndex::default()
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
                attempted: 4,
                succeeded: 4,
                results: vec![
                    RustAnalyzerLspQueryResult {
                        method: SemanticQueryMethod::WorkspaceSymbol,
                        path: ".".to_string(),
                        line: 1,
                        symbol: Some("build_context_pack".to_string()),
                        status: RustAnalyzerLspStatus::Succeeded,
                        duration_ms: 1,
                        summary: Some("1 item(s)".to_string()),
                        locations: vec![RustAnalyzerLspLocation {
                            role: RustAnalyzerLspLocationRole::WorkspaceSymbol,
                            path: "server/src/context_compiler/context_pack.rs".to_string(),
                            line: 10,
                            end_line: None,
                            symbol: Some("build_context_pack".to_string()),
                        }],
                        warning: None,
                    },
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
