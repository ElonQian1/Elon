use homecli_proto::{AgentToServer, CliWorkspaceStatus};

use crate::{cli_usage, node_agent_codex_session, node_agent_task_journal};

pub(crate) fn duplicate_cli_prompt_done(req_id: String) -> AgentToServer {
    AgentToServer::CliDone {
        req_id,
        exit_ok: false,
        error: Some("该任务已在本机节点运行，已拒绝重复启动，避免重复 CLI 进程堆积。".to_string()),
        session_id: None,
        prompt_tokens: None,
        cached_input_tokens: None,
        completion_tokens: None,
        reasoning_tokens: None,
        total_tokens: None,
        model: None,
        workspace_status: None,
    }
}

pub(crate) fn cli_done_message(
    req_id: String,
    exit_ok: bool,
    error: Option<String>,
    usage: Option<cli_usage::CliTokenUsage>,
    model: Option<String>,
    workspace_status: Option<CliWorkspaceStatus>,
    session_id: Option<String>,
) -> AgentToServer {
    let usage = usage.and_then(cli_usage::CliTokenUsage::normalized);
    AgentToServer::CliDone {
        req_id,
        exit_ok,
        error,
        session_id,
        prompt_tokens: usage.as_ref().map(|u| u.input_tokens.max(0) as u64),
        cached_input_tokens: usage.as_ref().map(|u| u.cached_input_tokens.max(0) as u64),
        completion_tokens: usage.as_ref().map(|u| u.output_tokens.max(0) as u64),
        reasoning_tokens: usage.as_ref().map(|u| u.reasoning_tokens.max(0) as u64),
        total_tokens: usage.as_ref().map(|u| u.total_tokens.max(0) as u64),
        model,
        workspace_status,
    }
}

pub(crate) fn latest_codex_session_id(
    cli_name: &str,
    codex_plan: &node_agent_codex_session::CodexSessionPlan,
    task_journal: &node_agent_task_journal::TaskJournal,
) -> Option<String> {
    if cli_name != "codex" {
        return None;
    }
    codex_plan
        .scope_key
        .as_deref()
        .and_then(|key| task_journal.load_codex_session(key).ok().flatten())
        .or_else(|| codex_plan.session_id.clone())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
