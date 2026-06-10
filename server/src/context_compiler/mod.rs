mod artifact;
mod cargo_index;
mod config;
mod context_pack;
mod hunyuan_brief;
mod model;
mod relevance;
mod repo_snapshot;
mod rust_analyzer;
mod rust_project;
mod rust_symbols;
mod symbol_graph;
mod validation;

use std::{path::Path, sync::Arc};

use crate::types::AppState;

use self::config::{ContextCompilerConfig, ContextCompilerMode};

pub(crate) async fn compile_preflight_note(
    state: &Arc<AppState>,
    workspace: &Path,
    user_id: &str,
    user_message: &str,
    trace_id: Option<&str>,
) -> Option<String> {
    let config = ContextCompilerConfig::from_env();
    if !config.enabled {
        return None;
    }

    let snapshot = repo_snapshot::collect_repo_snapshot(workspace);
    let rust_project = config
        .rust_probe_enabled
        .then(|| rust_project::collect_rust_project_summary(workspace))
        .flatten();
    let repo_index = if config.rust_analysis_enabled {
        let cargo = cargo_index::collect_cargo_index(workspace);
        let rust = rust_symbols::collect_rust_index(workspace, config.max_rust_files);
        let graph = symbol_graph::build_symbol_graph(
            workspace,
            &rust,
            user_message,
            config.max_symbols,
            config.max_relationships,
        );
        let rust_analyzer = rust_analyzer::collect_rust_analyzer_report(
            workspace,
            &rust,
            &graph,
            config.rust_analyzer_enabled,
            config.max_rust_analyzer_files,
        );
        Some(model::RepoContextIndex {
            cargo,
            rust,
            graph,
            rust_analyzer,
        })
    } else {
        None
    };
    let relevant_files =
        relevance::find_relevant_files(workspace, user_message, config.max_relevant_files);
    let validation_plan =
        validation::build_validation_plan(&snapshot, rust_project.as_ref(), &relevant_files);
    let deterministic_pack = context_pack::build_context_pack(
        &config,
        user_message,
        &snapshot,
        rust_project.as_ref(),
        repo_index.as_ref(),
        &relevant_files,
        &validation_plan,
        None,
    );
    let llm_brief =
        hunyuan_brief::build_llm_brief(state, &config, user_id, user_message, &deterministic_pack)
            .await;
    let final_pack = context_pack::build_context_pack(
        &config,
        user_message,
        &snapshot,
        rust_project.as_ref(),
        repo_index.as_ref(),
        &relevant_files,
        &validation_plan,
        llm_brief.as_deref(),
    );
    let artifact = artifact::save_context_artifacts(artifact::ContextArtifactsInput {
        data_dir: &state.data_dir,
        config: &config,
        trace_id,
        user_id,
        user_message,
        pack: &final_pack,
        llm_brief: llm_brief.as_deref(),
        snapshot: &snapshot,
        rust_project: rust_project.as_ref(),
        repo_index: repo_index.as_ref(),
        relevant_files: &relevant_files,
        validation_plan: &validation_plan,
    });

    if let Some(trace_id) = trace_id {
        state.server_traces.record(
            trace_id,
            "server_context_compiler_done",
            serde_json::json!({
                "mode": config.mode.as_str(),
                "injected": config.mode == ContextCompilerMode::Inject,
                "agent": config.agent_name,
                "llm_brief": llm_brief.is_some(),
                "relevant_files": relevant_files.len(),
                "rust_project": rust_project.is_some(),
                "rust_analysis": repo_index.as_ref().map(|index| serde_json::json!({
                    "cargo_packages": index.cargo.packages.len(),
                    "rust_files": index.rust.files_scanned,
                    "rust_symbols": index.rust.symbols.len(),
                    "relationships": index.graph.relationships.len(),
                    "ra_available": index.rust_analyzer.available,
                    "ra_files": index.rust_analyzer.files_enhanced,
                })),
                "pack_chars": final_pack.chars().count(),
                "artifact_path": artifact.as_ref().map(|item| item.path.display().to_string()),
                "artifact_bundle_dir": artifact.as_ref().map(|item| item.bundle_dir.display().to_string()),
                "artifact_file_count": artifact.as_ref().map(|item| item.files.len()),
                "artifact_bytes": artifact.as_ref().map(|item| item.bytes),
                "validation_commands": validation_plan.commands.len(),
            }),
        );
    }
    tracing::info!(
        mode = config.mode.as_str(),
        llm_brief = llm_brief.is_some(),
        relevant_files = relevant_files.len(),
        rust_project = rust_project.is_some(),
        rust_symbols = repo_index
            .as_ref()
            .map(|index| index.rust.symbols.len())
            .unwrap_or_default(),
        rust_analyzer = repo_index
            .as_ref()
            .map(|index| index.rust_analyzer.available)
            .unwrap_or_default(),
        artifact_path = artifact
            .as_ref()
            .map(|item| item.path.display().to_string())
            .unwrap_or_default(),
        artifact_bundle_dir = artifact
            .as_ref()
            .map(|item| item.bundle_dir.display().to_string())
            .unwrap_or_default(),
        validation_commands = validation_plan.commands.len(),
        "context compiler completed"
    );

    if config.mode == ContextCompilerMode::Inject {
        Some(final_pack)
    } else {
        None
    }
}
