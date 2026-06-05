use anyhow::{anyhow, Result};
use std::{path::Path, sync::Arc, time::Instant};

use super::{
    ai_cli_output::extract_thread_id,
    ai_cli_process::{
        cap_option_timeout, configured_timeout_cap, run_cli_command_traced, supports_codex_sessions,
    },
    ai_cli_prompts::build_prewarm_cli_prompt,
    ai_cli_trace::{record_prewarm_session_hit, CliTraceContext},
    NativeSessionScope,
};
use crate::types::AppState;

const DEFAULT_PREWARM_TIMEOUT_CAP_SECS: u64 = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct PrewarmResult {
    pub reused: bool,
    pub thread_id: Option<String>,
    pub elapsed_ms: u128,
}

pub async fn prewarm_codex_session(
    workspace: &Path,
    option_id: Option<&str>,
    native_session_scope: NativeSessionScope,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
) -> Result<PrewarmResult> {
    let started = Instant::now();
    let option = state
        .ai_cli
        .find_option(option_id)
        .cloned()
        .ok_or_else(|| anyhow!("no local AI CLI option is available"))?;
    if !supports_codex_sessions(&option) {
        return Err(anyhow!(
            "Codex CLI session prewarm requires a Codex CLI option"
        ));
    }

    std::fs::create_dir_all(workspace)?;
    let workspace_key = workspace.display().to_string();
    let existing_session_id = state.store.get_native_agent_session(
        &native_session_scope.project_id,
        &native_session_scope.user_id,
        Some(&native_session_scope.conversation_id),
        &option.provider,
        &option.id,
        &workspace_key,
    )?;
    if let Some(thread_id) = existing_session_id {
        record_prewarm_session_hit(
            state,
            trace_id,
            &native_session_scope,
            &workspace_key,
            Some(&thread_id),
            started.elapsed().as_millis(),
        );
        return Ok(PrewarmResult {
            reused: true,
            thread_id: Some(thread_id),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let mut prewarm_option = option.clone();
    cap_option_timeout(
        &mut prewarm_option,
        configured_timeout_cap(
            "AI_CLI_PREWARM_TIMEOUT_SECS",
            DEFAULT_PREWARM_TIMEOUT_CAP_SECS,
        ),
    );
    let prompt = build_prewarm_cli_prompt(workspace, &prewarm_option);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let output = run_cli_command_traced(
        &prewarm_option,
        workspace,
        &prompt,
        None,
        &tx,
        Some(CliTraceContext {
            state,
            trace_id,
            operation: "prewarm",
            attempt: "initial",
            route: None,
            development_task: None,
            prompt_bootstrapped: None,
        }),
    )
    .await?;
    let thread_id = extract_thread_id(&output.stdout);
    if let Some(thread_id) = thread_id.as_deref() {
        let _ = state.store.upsert_native_agent_session_if_no_active(
            &native_session_scope.project_id,
            &native_session_scope.user_id,
            Some(&native_session_scope.conversation_id),
            &option.provider,
            &option.id,
            &workspace_key,
            thread_id,
        );
    }

    Ok(PrewarmResult {
        reused: false,
        thread_id,
        elapsed_ms: started.elapsed().as_millis(),
    })
}
