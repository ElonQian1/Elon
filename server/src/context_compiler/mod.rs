mod artifact;
mod config;
mod context_pack;
mod hunyuan_brief;
mod relevance;
mod repo_snapshot;
mod rust_project;

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
    let relevant_files =
        relevance::find_relevant_files(workspace, user_message, config.max_relevant_files);
    let deterministic_pack = context_pack::build_context_pack(
        &config,
        user_message,
        &snapshot,
        rust_project.as_ref(),
        &relevant_files,
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
        &relevant_files,
        llm_brief.as_deref(),
    );
    let artifact =
        artifact::save_context_pack(&state.data_dir, &config, trace_id, user_id, &final_pack);

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
                "pack_chars": final_pack.chars().count(),
                "artifact_path": artifact.as_ref().map(|item| item.path.display().to_string()),
                "artifact_bytes": artifact.as_ref().map(|item| item.bytes),
            }),
        );
    }
    tracing::info!(
        mode = config.mode.as_str(),
        llm_brief = llm_brief.is_some(),
        relevant_files = relevant_files.len(),
        rust_project = rust_project.is_some(),
        artifact_path = artifact
            .as_ref()
            .map(|item| item.path.display().to_string())
            .unwrap_or_default(),
        "context compiler completed"
    );

    if config.mode == ContextCompilerMode::Inject {
        Some(final_pack)
    } else {
        None
    }
}
