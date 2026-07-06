    use super::*;
    use crate::context_compiler::config::ContextCompilerMode;
    use crate::context_compiler::repo_snapshot::RepoSnapshot;
    use crate::context_compiler::validation::ValidationPlan;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn saves_pack_and_bundle_under_data_dir() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_artifact_{}_{}",
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

        let artifact = save_context_artifacts(ContextArtifactsInput {
            data_dir: &dir,
            config: &config,
            trace_id: Some("trace/1"),
            user_id: "user@example",
            user_message: "hello task",
            pack: "hello",
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

        assert!(artifact.path.starts_with(&dir));
        assert_eq!(fs::read_to_string(&artifact.path).unwrap(), "hello");
        assert!(artifact.path.to_string_lossy().contains("trace1"));
        assert!(artifact.bundle_dir.starts_with(&dir));
        assert!(artifact.bundle_dir.join("repo_snapshot.json").is_file());
        assert!(artifact.bundle_dir.join("agent_prompt.md").is_file());
        assert!(artifact.files.len() >= 8);

        fs::remove_dir_all(dir).unwrap();
    }
