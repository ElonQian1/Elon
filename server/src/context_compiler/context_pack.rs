use super::{
    config::ContextCompilerConfig, context_pack_render, context_quality_render, impact_render,
    model::RepoContextIndex, relevance::RelevantFile, repo_map_tags_render,
    repo_snapshot::RepoSnapshot, rust_analyzer_probe_render, rust_project::RustProjectSummary,
    semantic_query_plan_render, validation::ValidationPlan,
};

pub(crate) fn build_context_pack(
    config: &ContextCompilerConfig,
    user_message: &str,
    snapshot: &RepoSnapshot,
    rust_project: Option<&RustProjectSummary>,
    repo_index: Option<&RepoContextIndex>,
    relevant_files: &[RelevantFile],
    validation_plan: &ValidationPlan,
    llm_brief: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str("<task_context_pack version=\"0.2\" source=\"elon-context-compiler\">\n\n");
    out.push_str("<instructions>\n");
    out.push_str("这份上下文包是只读预检产物，只能作为导航证据，不是真实代码的替代品。\n");
    out.push_str("执行修改前必须读取真实文件确认；如果上下文不足，请继续用当前 CLI 工具查询。\n");
    out.push_str("</instructions>\n\n");
    out.push_str("<task>\n");
    out.push_str("用户原始请求：\n");
    out.push_str(user_message.trim());
    out.push_str("\n</task>\n\n");

    if let Some(index) = repo_index {
        context_pack_render::render_task_profile(&mut out, &index.task);
    }

    if let Some(brief) = llm_brief.filter(|value| !value.trim().is_empty()) {
        out.push_str("<llm_brief model_role=\"context-compressor\">\n");
        out.push_str(brief.trim());
        out.push_str("\n</llm_brief>\n\n");
    }

    context_pack_render::render_repo_snapshot(&mut out, snapshot);
    context_pack_render::render_rust_project(&mut out, rust_project);
    context_pack_render::render_repo_index(&mut out, repo_index);
    repo_map_tags_render::render_repo_map_tags(
        &mut out,
        repo_index.map(|index| &index.graph.repo_map_tags),
    );
    rust_analyzer_probe_render::render_rust_analyzer_probes(
        &mut out,
        repo_index.map(|index| &index.rust_analyzer.probes),
    );
    semantic_query_plan_render::render_semantic_query_plan(
        &mut out,
        repo_index.map(|index| &index.semantic_plan),
    );
    context_quality_render::render_context_quality(
        &mut out,
        repo_index.map(|index| &index.quality),
    );
    context_pack_render::render_source_size_risks(&mut out, snapshot);
    impact_render::render_impact_analysis(&mut out, repo_index.map(|index| &index.impact));
    context_pack_render::render_context_evidence(&mut out, repo_index.map(|index| &index.evidence));
    context_pack_render::render_retrieval_evidence(&mut out, relevant_files);
    context_pack_render::render_validation_guidance(&mut out, validation_plan);
    context_pack_render::render_output_contract(&mut out);
    out.push_str("</task_context_pack>");

    truncate_pack(out, config.max_pack_chars)
}

fn truncate_pack(mut pack: String, max_chars: usize) -> String {
    if pack.chars().count() <= max_chars {
        return pack;
    }
    let mut truncated = pack.chars().take(max_chars).collect::<String>();
    truncated.push_str("\n\n<!-- context pack truncated by ELON_CONTEXT_COMPILER_MAX_CHARS -->");
    pack.clear();
    truncated
}

#[cfg(test)]
mod tests {
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
}
