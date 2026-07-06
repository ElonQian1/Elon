    use super::*;
    use crate::context_compiler::{
        config::ContextCompilerMode,
        model::{
            ContextEvidence, ImpactFact, ImpactKind, RepoContextIndex, RustImpactAnalysis,
            TaskProfile,
        },
    };

    fn test_config() -> ContextCompilerConfig {
        ContextCompilerConfig {
            enabled: true,
            mode: ContextCompilerMode::Inject,
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

    fn test_snapshot() -> RepoSnapshot {
        RepoSnapshot {
            git_head: Some("abc123".to_string()),
            git_branch: Some("main".to_string()),
            git_dirty: false,
            git_status_short: Vec::new(),
            has_origin: true,
            top_level_entries: vec!["server/".to_string()],
            instruction_docs: vec!["AGENTS.md".to_string()],
            manifests: vec!["Cargo.toml".to_string()],
            large_files: Vec::new(),
            source_file_count: 1,
        }
    }

    #[test]
    fn context_pack_contains_navigation_warning_and_relevant_file() {
        let config = test_config();
        let snapshot = test_snapshot();
        let relevant = vec![RelevantFile {
            path: "server/src/context_compiler/mod.rs".to_string(),
            score: 9,
            lines: 120,
            role: "source",
            reasons: vec!["path contains `context`".to_string()],
            matches: Vec::new(),
        }];
        let validation = ValidationPlan {
            commands: Vec::new(),
            notes: vec!["read files".to_string()],
        };

        let pack = build_context_pack(
            &config,
            "实现 context compiler",
            &snapshot,
            None,
            None,
            &[],
            None,
            &relevant,
            &validation,
            None,
        );

        assert!(pack.contains("只读预检产物"));
        assert!(pack.contains("server/src/context_compiler/mod.rs"));
        assert!(pack.contains("Cargo.toml"));
        assert!(pack.contains("<retrieval_evidence>"));
        assert!(pack.contains("<final_instructions>"));
    }

    #[test]
    fn context_pack_includes_rust_project_summary() {
        let config = test_config();
        let snapshot = RepoSnapshot {
            git_dirty: true,
            git_status_short: vec![" M src/lib.rs".to_string()],
            ..test_snapshot()
        };
        let rust = RustProjectSummary {
            root_package: Some("elon-server".to_string()),
            workspace: true,
            workspace_members: vec!["server".to_string()],
            manifests: Vec::new(),
            toolchain: Some("stable".to_string()),
        };
        let validation = ValidationPlan {
            commands: Vec::new(),
            notes: Vec::new(),
        };

        let pack = build_context_pack(
            &config,
            "任务",
            &snapshot,
            Some(&rust),
            None,
            &[],
            None,
            &[],
            &validation,
            None,
        );

        assert!(pack.contains("<rust_project>"));
        assert!(pack.contains("root_package: elon-server"));
        assert!(pack.contains("git_dirty: true"));
    }

    #[test]
    fn context_pack_includes_task_and_evidence_sections() {
        let config = test_config();
        let snapshot = test_snapshot();
        let index = RepoContextIndex {
            task: TaskProfile {
                keywords: vec!["repo".to_string(), "map".to_string()],
                likely_domains: vec!["rust_context_compiler".to_string()],
                ..TaskProfile::default()
            },
            evidence: ContextEvidence {
                missing_context: vec!["no direct test file identified".to_string()],
                recommended_actions: vec!["Open edit targets first".to_string()],
                ..ContextEvidence::default()
            },
            impact: RustImpactAnalysis {
                function_call_sites: vec![ImpactFact {
                    subject: "build_context_pack".to_string(),
                    path: "server/src/context_compiler/context_pack.rs".to_string(),
                    line: 12,
                    kind: ImpactKind::FunctionCallSite,
                    evidence: "build_context_pack(...)".to_string(),
                    reason: "call-like token hit".to_string(),
                }],
                ..RustImpactAnalysis::default()
            },
            ..RepoContextIndex::default()
        };
        let validation = ValidationPlan {
            commands: Vec::new(),
            notes: Vec::new(),
        };

        let pack = build_context_pack(
            &config,
            "继续完善 repo map",
            &snapshot,
            None,
            None,
            &[],
            Some(&index),
            &[],
            &validation,
            None,
        );

        assert!(pack.contains("<task_understanding>"));
        assert!(pack.contains("<impact_analysis>"));
        assert!(pack.contains("<missing_context_policy>"));
        assert!(pack.contains("<recommended_agent_actions>"));
    }
