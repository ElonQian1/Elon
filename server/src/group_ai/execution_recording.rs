use serde_json::{json, Value};

use crate::{
    group_ai::types::ProjectAiMatter,
    store::{NodeComputeRun, ProjectExecutionSession},
    types::AppState,
};

pub(super) fn insert_event(
    state: &AppState,
    matter: &ProjectAiMatter,
    actor_user_id: Option<&str>,
    event_type: &str,
    payload: Value,
) {
    if let Err(error) = state.store.insert_project_ai_event(
        &matter.project_id,
        &matter.id,
        actor_user_id,
        event_type,
        payload,
    ) {
        tracing::warn!(
            matter_id = matter.id,
            event_type,
            "群体 AI 事件写入失败: {error:#}"
        );
    } else {
        crate::project_events::publish_group_ai_matter_event(
            state,
            &matter.project_id,
            &matter.id,
            actor_user_id,
            event_type,
            "群体 AI Matter 有新的执行事件。",
        );
    }
}

pub(super) fn write_channel_notice(
    state: &AppState,
    matter: &ProjectAiMatter,
    actor_user_id: &str,
    message: &str,
) {
    let content = format!("{}：{}", matter.title, message);
    if let Err(error) = state.store.insert_project_channel_message(
        &matter.project_id,
        &matter.channel_id,
        Some(actor_user_id),
        "ai_progress",
        &content,
        None,
        None,
    ) {
        tracing::warn!(matter_id = matter.id, "群体 AI 频道通知写入失败: {error:#}");
    }
}

pub(super) fn session_payload(session: Option<&ProjectExecutionSession>) -> Value {
    session
        .map(|session| {
            json!({
                "request_id": session.request_id,
                "status": session.status,
                "merge_status": session.merge_status.as_deref(),
                "base_workspace_path": session.base_workspace_path.as_deref(),
                "active_workspace_path": session.active_workspace_path.as_deref(),
                "branch": session.branch.as_deref(),
                "isolated": session.isolated,
                "last_error": session.last_error.as_deref(),
                "token_usage_event_id": session.token_usage_event_id.as_deref(),
                "billing_event_id": session.billing_event_id.as_deref()
            })
        })
        .unwrap_or(Value::Null)
}

pub(super) fn compute_run_payload(run: Option<&NodeComputeRun>) -> Value {
    run.map(|run| {
        json!({
            "compute_call_id": run.compute_call_id,
            "status": run.status,
            "settlement_status": run.settlement_status.as_deref(),
            "prompt_tokens": run.prompt_tokens,
            "completion_tokens": run.completion_tokens,
            "billed_cost_rmb_fen": run.billed_cost_rmb_fen,
            "provider_earned_fen": run.provider_earned_fen,
            "error_message": run.error_message.as_deref()
        })
    })
    .unwrap_or(Value::Null)
}

pub(super) fn assignment_status_from_compute_run(run: Option<&NodeComputeRun>) -> &'static str {
    match run.map(|run| run.status.as_str()) {
        Some("settled" | "deduplicated") => "settled",
        Some("settled_no_provider") => "settled_no_provider",
        Some("failed" | "settlement_failed") => "failed",
        _ => "completed",
    }
}
