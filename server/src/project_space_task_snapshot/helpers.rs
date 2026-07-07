use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use homecli_proto::AgentToServer;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    project_mobile::ensure_mobile_project,
    project_space,
    store::{ProjectAccess, ProjectChannelMessage, TaskEventRecord, TaskSnapshot},
    types::AppState,
};

use super::{
    LocalJournalProbe, PcDispatchContext, RuntimeEventView, RuntimeEventsResponse,
    RuntimeSnapshotResponse, TaskAttachState, TaskSnapshotContext,
    DEFAULT_EVENT_LIMIT, LOCAL_JOURNAL_PROBE_RETRY_DELAY, LOCAL_JOURNAL_PROBE_TIMEOUT,
    TASK_MESSAGE_LIMIT,
};

pub(super) fn load_task_snapshot_context(
    state: &AppState,
    user_id: &str,
    project: ProjectAccess,
    channel_id: &str,
    task_id: &str,
    since: i64,
    limit: usize,
) -> Result<TaskSnapshotContext, Response> {
    let channel_kind = state
        .store
        .get_project_channel_kind(&project.id, channel_id)
        .map_err(|e| json_error(StatusCode::NOT_FOUND, e.to_string()))?;
    if channel_kind != "ai_development" {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "只有 AI开发 频道支持任务快照",
        ));
    }
    let task = state
        .store
        .get_channel_task_snapshot(&project.id, channel_id, task_id)
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "任务不存在或不属于该频道"))?;
    let messages = state
        .store
        .list_project_channel_messages(user_id, &project.id, channel_id, TASK_MESSAGE_LIMIT)
        .map_err(|e| json_error(StatusCode::BAD_REQUEST, e.to_string()))?;
    let events = state
        .store
        .list_task_events_after(task_id, since, limit)
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let last_event_seq = state
        .store
        .latest_task_event_seq(task_id)
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let pc_dispatch = state
        .store
        .list_task_events(task_id, usize::MAX)
        .ok()
        .and_then(|events| derive_pc_dispatch_context(events.iter().map(String::as_str)));
    Ok(TaskSnapshotContext {
        project,
        task,
        messages,
        events,
        last_event_seq,
        pc_dispatch,
    })
}

pub(super) fn project_from_auth(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<(String, ProjectAccess), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|e| json_error(StatusCode::UNAUTHORIZED, e.to_string()))?;
    let project = project_access(state, &user.id, project_id)
        .map_err(|e| json_error(StatusCode::FORBIDDEN, e.to_string()))?;
    Ok((user.id, project))
}

pub(super) fn user_project_from_auth(
    state: &AppState,
    headers: &HeaderMap,
    user_id: &str,
    project_id: &str,
    project_title: Option<&str>,
) -> Result<(String, ProjectAccess), Response> {
    let effective_user_id = if state.require_login {
        auth_from_headers(state, headers)
            .map_err(|e| json_error(StatusCode::UNAUTHORIZED, e.to_string()))?
            .id
    } else {
        user_id.to_string()
    };
    ensure_mobile_project(state, &effective_user_id, project_id, project_title)
        .map(|(user, project)| (user.id, project))
        .map_err(|e| json_error(StatusCode::BAD_REQUEST, e.to_string()))
}

pub(super) fn attach_state(project_id: &str, channel_id: &str, task: &TaskSnapshot) -> TaskAttachState {
    if project_space::is_channel_ai_task_active(&task.id, project_id, channel_id) {
        return TaskAttachState {
            status: "live",
            live: true,
            reason: "服务端仍持有该任务的运行控制句柄",
        };
    }
    if task.status == "running" {
        return TaskAttachState {
            status: "detached",
            live: false,
            reason: "任务仍标记为运行中，但当前服务端没有运行控制句柄；需要重新 attach 或继续",
        };
    }
    TaskAttachState {
        status: "terminal",
        live: false,
        reason: "任务已经进入终态，只能基于快照继续新一轮处理",
    }
}

pub(super) fn render_events(events: Vec<TaskEventRecord>) -> Vec<RuntimeEventView> {
    events
        .into_iter()
        .map(|event| RuntimeEventView {
            seq: event.seq,
            created_at: event.created_at,
            event: serde_json::from_str(&event.event_json)
                .unwrap_or_else(|_| json!({ "type": "invalid_json", "raw": event.event_json })),
        })
        .collect()
}

pub(super) async fn resolve_local_journal(
    state: &AppState,
    dispatch: Option<&PcDispatchContext>,
    since: i64,
    limit: usize,
) -> LocalJournalProbe {
    let Some(dispatch) = dispatch else {
        return LocalJournalProbe {
            status: "missing_pc_req_id".to_string(),
            reachable: false,
            pc_req_id: None,
            agent_id: None,
            node_display_name: None,
            cli: None,
            message: "云端任务事件中没有找到 PC 请求 ID，只能使用云端快照继续。".to_string(),
            error: None,
            snapshot: None,
        };
    };
    let Some(agent_id) = dispatch
        .agent_id
        .as_deref()
        .filter(|value| !value.is_empty())
    else {
        return LocalJournalProbe {
            status: "missing_agent_id".to_string(),
            reachable: false,
            pc_req_id: Some(dispatch.pc_req_id.clone()),
            agent_id: None,
            node_display_name: dispatch.node_display_name.clone(),
            cli: dispatch.cli.clone(),
            message: "云端知道本轮 PC 请求 ID，但缺少节点 ID，暂时不能读取本机 journal。"
                .to_string(),
            error: None,
            snapshot: None,
        };
    };

    match dispatch_cli_task_journal_inspect_with_retry(
        state,
        agent_id,
        &dispatch.pc_req_id,
        since.max(0) as usize,
        limit.clamp(1, 100),
    )
    .await
    {
        Ok(AgentToServer::CliTaskJournalSnapshot {
            ok: true,
            snapshot,
            error,
            ..
        }) => LocalJournalProbe {
            status: "available".to_string(),
            reachable: true,
            pc_req_id: Some(dispatch.pc_req_id.clone()),
            agent_id: Some(agent_id.to_string()),
            node_display_name: dispatch.node_display_name.clone(),
            cli: dispatch.cli.clone(),
            message: local_journal_message(snapshot.as_ref()),
            error,
            snapshot,
        },
        Ok(AgentToServer::CliTaskJournalSnapshot {
            ok: false,
            snapshot,
            error,
            ..
        }) => LocalJournalProbe {
            status: "unavailable".to_string(),
            reachable: true,
            pc_req_id: Some(dispatch.pc_req_id.clone()),
            agent_id: Some(agent_id.to_string()),
            node_display_name: dispatch.node_display_name.clone(),
            cli: dispatch.cli.clone(),
            message: "PC 节点在线，但没有返回可用的本机 journal 恢复快照。".to_string(),
            error,
            snapshot,
        },
        Ok(other) => LocalJournalProbe {
            status: "unexpected_response".to_string(),
            reachable: true,
            pc_req_id: Some(dispatch.pc_req_id.clone()),
            agent_id: Some(agent_id.to_string()),
            node_display_name: dispatch.node_display_name.clone(),
            cli: dispatch.cli.clone(),
            message: "PC 节点返回了非 journal 快照响应。".to_string(),
            error: Some(format!("{other:?}")),
            snapshot: None,
        },
        Err(error) => LocalJournalProbe {
            status: "agent_offline_or_timeout".to_string(),
            reachable: false,
            pc_req_id: Some(dispatch.pc_req_id.clone()),
            agent_id: Some(agent_id.to_string()),
            node_display_name: dispatch.node_display_name.clone(),
            cli: dispatch.cli.clone(),
            message: "暂时不能读取本机 journal；Win 端重连后可以再次刷新任务快照。".to_string(),
            error: Some(error.to_string()),
            snapshot: None,
        },
    }
}

pub(super) async fn dispatch_cli_task_journal_inspect_with_retry(
    state: &AppState,
    agent_id: &str,
    task_id: &str,
    since: usize,
    limit: usize,
) -> std::result::Result<AgentToServer, anyhow::Error> {
    let first = state
        .agent_manager
        .dispatch_cli_task_journal_inspect(
            agent_id,
            task_id,
            since,
            limit,
            LOCAL_JOURNAL_PROBE_TIMEOUT,
        )
        .await;
    if first.is_ok() {
        return first;
    }

    tokio::time::sleep(LOCAL_JOURNAL_PROBE_RETRY_DELAY).await;
    state
        .agent_manager
        .dispatch_cli_task_journal_inspect(
            agent_id,
            task_id,
            since,
            limit,
            LOCAL_JOURNAL_PROBE_TIMEOUT,
        )
        .await
}

pub(super) fn local_journal_message(snapshot: Option<&Value>) -> String {
    let Some(snapshot) = snapshot else {
        return "已连接 PC 节点，但本机 journal 快照为空。".to_string();
    };
    let status = snapshot
        .get("resume")
        .and_then(|resume| resume.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let next_action = snapshot
        .get("resume")
        .and_then(|resume| resume.get("next_action"))
        .or_else(|| {
            snapshot
                .get("resume")
                .and_then(|resume| resume.get("nextAction"))
        })
        .and_then(Value::as_str)
        .unwrap_or("refresh_snapshot");
    let reason = snapshot
        .get("resume")
        .and_then(|resume| resume.get("reason"))
        .and_then(Value::as_str)
        .or_else(|| {
            snapshot
                .get("attach")
                .and_then(|attach| attach.get("reason"))
                .and_then(Value::as_str)
        })
        .unwrap_or("");
    if reason.is_empty() {
        format!("已读取本机 journal 恢复合同：状态 {status}，下一步 {next_action}。")
    } else {
        format!("已读取本机 journal 恢复合同：状态 {status}，下一步 {next_action}。{reason}")
    }
}

pub(super) fn derive_pc_dispatch_context<'a>(
    events: impl IntoIterator<Item = &'a str>,
) -> Option<PcDispatchContext> {
    let mut context = None;
    for raw in events {
        let Ok(value) = serde_json::from_str::<Value>(raw) else {
            continue;
        };
        if let Some(next) = pc_dispatch_context_from_event(&value) {
            context = Some(next);
        } else if context.is_none() {
            if let Some(req_id) = pc_req_id_from_event(&value) {
                context = Some(PcDispatchContext {
                    pc_req_id: req_id,
                    agent_id: None,
                    node_display_name: None,
                    cli: None,
                });
            }
        }
    }
    context
}

pub(super) fn pc_dispatch_context_from_event(value: &Value) -> Option<PcDispatchContext> {
    let event_type = value.get("type").and_then(Value::as_str).unwrap_or("");
    if event_type != "pc_dispatch_started" {
        return None;
    }
    let pc_req_id = pc_req_id_from_event(value)?;
    Some(PcDispatchContext {
        pc_req_id,
        agent_id: string_field(value, "agent_id"),
        node_display_name: string_field(value, "node_display_name"),
        cli: string_field(value, "cli"),
    })
}

pub(super) fn pc_req_id_from_event(value: &Value) -> Option<String> {
    let event_type = value.get("type").and_then(Value::as_str).unwrap_or("");
    let is_pc_event = matches!(
        event_type,
        "pc_dispatch_started"
            | "tool_approval_required"
            | "tool_approval_decision"
            | "tool_call"
            | "tool_result"
            | "usage"
    );
    if !is_pc_event {
        return None;
    }
    value
        .get("pc_req_id")
        .or_else(|| value.get("req_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|req_id| !req_id.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::derive_pc_dispatch_context;

    #[test]
    fn derives_pc_dispatch_context_from_dispatch_event() {
        let events = [
            r#"{"type":"progress","message":"running"}"#,
            r#"{"type":"pc_dispatch_started","pc_req_id":"req-local","task_id":"tsk-cloud","agent_id":"node-a","node_display_name":"一龙4060","cli":"codex"}"#,
        ];
        let context = derive_pc_dispatch_context(events).expect("pc dispatch context");
        assert_eq!(context.pc_req_id, "req-local");
        assert_eq!(context.agent_id.as_deref(), Some("node-a"));
        assert_eq!(context.node_display_name.as_deref(), Some("一龙4060"));
        assert_eq!(context.cli.as_deref(), Some("codex"));
    }

    #[test]
    fn ignores_non_pc_req_ids() {
        let events = [r#"{"type":"progress","req_id":"not-local"}"#];
        assert!(derive_pc_dispatch_context(events).is_none());
    }
}
