use anyhow::bail;
use homecli_proto::CliWorkspaceStatus;

use super::{pc_cli_model_id, pc_cli_usage_tokens, NativeSessionScope};
use crate::types::AppState;

pub(crate) fn start_pc_node_compute_run(
    state: &AppState,
    consumer_user_id: &str,
    node_id: &str,
    compute_call_id: &str,
    feature: &str,
    model: Option<&str>,
) -> anyhow::Result<crate::store::NodeComputeRun> {
    let provider_user_id = state.store.get_node_credential_owner(node_id)?;
    let model_id = pc_cli_model_id(model);
    let run = state
        .store
        .start_node_compute_run(crate::store::NodeComputeRunStart {
            compute_call_id,
            consumer_user_id,
            provider_user_id: provider_user_id.as_deref(),
            node_id,
            model_id: Some(&model_id),
            feature,
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
        .map_err(anyhow::Error::from)?;
    if run.consumer_user_id != consumer_user_id
        || run.provider_user_id != provider_user_id
        || run.node_id != node_id
        || run.model_id.as_deref() != Some(model_id.as_str())
        || run.feature != feature
        || run.usage_mode != "pc_agent_cli"
        || run.route_reason.as_deref() != Some("pc_agent_selected")
        || run.status != "started"
    {
        bail!("compute_call_id 已绑定其他 PC CLI 计算身份或终态");
    }
    Ok(run)
}

pub(crate) fn finish_pc_node_compute_run(
    state: &AppState,
    compute_call_id: &str,
    requested_status: &str,
    usage: Option<&crate::cli_usage::CliTokenUsage>,
    accounting_result: Option<&crate::store::TokenUsageAccountingResult>,
    node_transaction: Option<&crate::store::NodeTransaction>,
    error_message: Option<&str>,
) {
    let (prompt_tokens, completion_tokens) = usage.map(pc_cli_usage_tokens).unwrap_or((0, 0));
    let status = if requested_status == "settled" {
        if accounting_result
            .map(|result| result.deduplicated)
            .unwrap_or(false)
        {
            "deduplicated"
        } else if accounting_result.is_none() {
            "settlement_failed"
        } else if node_transaction.is_none() {
            "settlement_skipped"
        } else {
            "settled"
        }
    } else {
        requested_status
    };
    let billed_cost = node_transaction
        .map(|tx| tx.billed_cost_rmb_fen)
        .or_else(|| accounting_result.map(|result| result.cost_rmb_fen))
        .unwrap_or(0);
    let provider_earned = node_transaction
        .map(|tx| tx.provider_earned_fen)
        .unwrap_or(0);
    let settlement_status = node_transaction
        .map(|tx| tx.settlement_status.as_str())
        .or_else(|| accounting_result.map(|result| result.accounting_status.as_str()));
    if let Err(e) = state.store.finish_node_compute_run(
        compute_call_id,
        crate::store::NodeComputeRunFinish {
            provider_user_id: node_transaction.map(|tx| tx.provider_user_id.as_str()),
            status,
            prompt_tokens,
            completion_tokens,
            billed_cost_rmb_fen: billed_cost,
            provider_earned_fen: provider_earned,
            settlement_status,
            error_message,
        },
    ) {
        tracing::warn!(compute_call_id, "PC CLI 执行证明 finish 记录失败: {e:#}");
    }
}

pub(crate) fn record_pc_execution_started(
    state: &AppState,
    scope: Option<&NativeSessionScope>,
    node_id: &str,
    request_id: &str,
    requested_workspace_path: Option<&str>,
    model: Option<&str>,
) -> anyhow::Result<()> {
    let Some(scope) = scope else {
        return Ok(());
    };
    let recorded = state.store.record_project_execution_started(
        crate::store::ProjectExecutionSessionStart {
            project_id: &scope.project_id,
            conversation_id: &scope.conversation_id,
            user_id: &scope.user_id,
            node_id,
            request_id,
            requested_workspace_path,
            model,
        },
    )?;
    if !recorded {
        bail!("PC CLI req_id 已绑定其他项目执行身份");
    }
    Ok(())
}

pub(crate) fn pc_route_a_prompt_bootstrapped(
    state: &AppState,
    scope: Option<&NativeSessionScope>,
    cli_name: &str,
    agent_id: &str,
    cwd: Option<&str>,
    native_session_id: Option<&str>,
    development: bool,
) -> bool {
    let (Some(scope), Some(native_session_id)) = (scope, native_session_id) else {
        return false;
    };
    let workspace_key = pc_route_a_workspace_key(cwd);
    let _ = state.store.upsert_native_agent_session_if_no_active(
        &scope.project_id,
        &scope.user_id,
        Some(&scope.conversation_id),
        cli_name,
        agent_id,
        workspace_key,
        native_session_id,
    );
    state
        .store
        .get_native_agent_session_state(
            &scope.project_id,
            &scope.user_id,
            Some(&scope.conversation_id),
            cli_name,
            agent_id,
            workspace_key,
        )
        .ok()
        .flatten()
        .map(|session| {
            if development {
                session.dev_bootstrapped
            } else {
                session.chat_bootstrapped
            }
        })
        .unwrap_or(false)
}

pub(crate) fn mark_pc_route_a_prompt_bootstrapped(
    state: &AppState,
    scope: Option<&NativeSessionScope>,
    cli_name: &str,
    agent_id: &str,
    cwd: Option<&str>,
    native_session_id: Option<&str>,
    development: bool,
) {
    let (Some(scope), Some(native_session_id)) = (scope, native_session_id) else {
        return;
    };
    let workspace_key = pc_route_a_workspace_key(cwd);
    let _ = state.store.upsert_native_agent_session(
        &scope.project_id,
        &scope.user_id,
        Some(&scope.conversation_id),
        cli_name,
        agent_id,
        workspace_key,
        native_session_id,
    );
    let _ = state.store.mark_native_agent_session_bootstrapped(
        &scope.project_id,
        &scope.user_id,
        Some(&scope.conversation_id),
        cli_name,
        agent_id,
        workspace_key,
        native_session_id,
        development,
    );
}

fn pc_route_a_workspace_key(cwd: Option<&str>) -> &str {
    cwd.map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
}

pub(crate) fn record_pc_codex_thread_id(
    state: &AppState,
    scope: Option<&NativeSessionScope>,
    node_id: &str,
    requested_workspace_path: Option<&str>,
    workspace_status: Option<&CliWorkspaceStatus>,
    session_id: Option<&str>,
) {
    let Some(scope) = scope else {
        return;
    };
    let Some(session_id) = session_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    let workspace_path = workspace_status
        .map(|status| status.active_workspace_path.as_str())
        .or(requested_workspace_path)
        .unwrap_or("");

    if let Err(e) = state.store.upsert_native_agent_session(
        &scope.project_id,
        &scope.user_id,
        Some(&scope.conversation_id),
        "codex",
        node_id,
        workspace_path,
        session_id,
    ) {
        tracing::warn!("record PC Codex native session failed: {e:#}");
    }
    if let Err(e) = state.store.set_latest_task_thread_id(
        &scope.project_id,
        &scope.user_id,
        &scope.conversation_id,
        session_id,
    ) {
        tracing::warn!("record PC Codex task thread id failed: {e:#}");
    }
}

pub(crate) fn record_pc_execution_finished(
    state: &AppState,
    scope: Option<&NativeSessionScope>,
    node_id: &str,
    request_id: &str,
    exit_ok: bool,
    error: Option<&str>,
    model: Option<&str>,
    workspace_status: Option<&CliWorkspaceStatus>,
    usage: Option<&crate::cli_usage::CliTokenUsage>,
    accounting_result: Option<&crate::store::TokenUsageAccountingResult>,
) {
    let Some(scope) = scope else {
        return;
    };
    let status = if exit_ok { "done" } else { "failed" };
    let merge_status = workspace_status
        .and_then(|status| status.merge_status.as_deref())
        .or(Some("legacy_no_workspace_status"));
    let workspace_message = workspace_status.and_then(|status| status.merge_message.as_deref());
    let last_error = error.or_else(|| (!exit_ok).then_some(workspace_message).flatten());

    match state.store.record_project_execution_finished(
        crate::store::ProjectExecutionSessionFinish {
            request_id,
            project_id: &scope.project_id,
            conversation_id: &scope.conversation_id,
            user_id: &scope.user_id,
            node_id,
            base_workspace_path: workspace_status
                .and_then(|status| status.base_workspace_path.as_deref()),
            active_workspace_path: workspace_status
                .map(|status| status.active_workspace_path.as_str()),
            branch: workspace_status.and_then(|status| status.branch.as_deref()),
            isolated: workspace_status
                .map(|status| status.isolated)
                .unwrap_or(false),
            status,
            merge_status,
            last_error,
            model,
            prompt_tokens: usage.map(|usage| usage.input_tokens.max(0)),
            cached_input_tokens: usage.map(|usage| usage.cached_input_tokens.max(0)),
            completion_tokens: usage.map(|usage| usage.output_tokens.max(0)),
            reasoning_tokens: usage.map(|usage| usage.reasoning_tokens.max(0)),
            total_tokens: usage.map(|usage| usage.total_tokens.max(0)),
            token_usage_event_id: accounting_result
                .map(|result| result.token_usage_event_id.as_str()),
            billing_event_id: accounting_result
                .and_then(|result| result.billing_event_id.as_deref()),
        },
    ) {
        Ok(true) => {}
        Ok(false) => tracing::warn!(
            request_id,
            node_id,
            "record project execution finish rejected an identity mismatch"
        ),
        Err(e) => tracing::warn!("record project execution finish failed: {e:#}"),
    }
}

pub(crate) fn record_pc_execution_without_cli_done(
    state: &AppState,
    scope: Option<&NativeSessionScope>,
    node_id: &str,
    request_id: &str,
    exit_ok: bool,
    error: Option<&str>,
    model: Option<&str>,
) {
    record_pc_execution_finished(
        state, scope, node_id, request_id, exit_ok, error, model, None, None, None,
    );
}
