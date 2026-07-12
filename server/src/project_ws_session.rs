//! 项目 WebSocket 会话生命周期管理。
//!
//! 负责单个 WS 连接从握手到断开的完整消息循环；任务的创建和执行由
//! [`crate::project_ws_job`] 负责，协议解析由 [`crate::project_ws_protocol`] 负责。

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use std::sync::{atomic::Ordering, Arc};
use tokio::sync::broadcast;

use crate::{
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_attachment_notes::{
        append_project_attachment_notes, project_message_with_attachment_fallback,
    },
    project_auth::project_access,
    project_execution_mode::ProjectExecutionMode,
    project_keys::{clean_trace_id, project_ws_fingerprint},
    project_trace_events::record_server_transport,
    project_workspace_recovery,
    project_ws_job::{cancel_project_ws_job, emit_project_job_event, get_or_start_project_ws_job},
    project_ws_protocol::{
        is_terminal_project_ws_message, parse_project_message, project_client_request_id,
        task_control_event,
    },
    store::{ProjectAccess, PublicUser},
    types::{AppState, WsMessage},
    ui_design_tasks::append_ui_design_task_context,
};

/// 单个已升级的 WebSocket 连接的完整会话循环。
///
/// 负责：更新推送、协议握手、消息接收、任务调度、进度回放、断线感知。
pub(crate) async fn handle_project_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    user: PublicUser,
    mut project: ProjectAccess,
    download_base: String,
    client_version_code: Option<i64>,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut update_rx = crate::app_update::subscribe();

    if let Some(event) =
        crate::app_update::latest_update_event_for_client(&state, client_version_code).await
    {
        if sender.send(Message::Text(event)).await.is_err() {
            return;
        }
    }

    // 方案8: 告知客户端当前服务器协议版本；旧客户端忽略未知 type 即可
    let hello = crate::ws_message::WsMessage::ProtocolHello {
        server_version: crate::ws_message::SERVER_PROTOCOL_VERSION,
        min_client_version: crate::ws_message::MIN_CLIENT_PROTOCOL_VERSION,
    }
    .to_json();
    if sender.send(Message::Text(hello)).await.is_err() {
        return;
    }

    loop {
        let text = tokio::select! {
            update = update_rx.recv() => {
                if let Ok(event) = update {
                    if crate::app_update::is_newer_for_client(&event, client_version_code)
                        && sender.send(Message::Text(event)).await.is_err()
                    {
                        break;
                    }
                }
                continue;
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Ping(payload))) => {
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                        continue;
                    }
                    Some(Ok(Message::Pong(_))) => continue,
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Binary(_))) => continue,
                    Some(Err(_)) => break,
                }
            }
        };

        let request = parse_project_message(&text);
        let op = request.op.as_deref().unwrap_or("run").to_ascii_lowercase();

        let conversation_id = state
            .store
            .ensure_conversation(
                &project.id,
                &user.id,
                request.conversation_id.as_deref(),
                request.conversation_title.as_deref(),
            )
            .unwrap_or_else(|_| "default".into());
        if op == "cancel" {
            let canceled_task_id = cancel_project_ws_job(
                &project.id,
                &user.id,
                &conversation_id,
                request.task_id.as_deref(),
                request.client_request_id.as_deref(),
            )
            .await;
            let payload = match canceled_task_id.as_deref() {
                Some(task_id) => task_control_event(
                    "cancel_requested",
                    Some(task_id),
                    request.client_request_id.as_deref(),
                    Some(&conversation_id),
                    "已接收取消请求，任务会尽快停止。",
                ),
                None => task_control_event(
                    "cancel_ignored",
                    request.task_id.as_deref(),
                    request.client_request_id.as_deref(),
                    Some(&conversation_id),
                    "没有找到可取消的运行中任务。",
                ),
            };
            if sender.send(Message::Text(payload)).await.is_err() {
                break;
            }
            continue;
        }

        let display_message = project_message_with_attachment_fallback(
            request.message.trim().to_string(),
            request.attachments.as_deref(),
        );
        if display_message.is_empty() {
            continue;
        }
        let pc_runtime_route = match request.pc_runtime_route() {
            Ok(route) => route,
            Err(message) => {
                if sender
                    .send(Message::Text(WsMessage::error(message).to_json()))
                    .await
                    .is_err()
                {
                    break;
                }
                continue;
            }
        };
        let direct_pc_cli = request.direct_pc_cli.unwrap_or(false);
        if should_auto_bind_local_node(pc_runtime_route) {
            if let (Some(node_id), Some(workspace_path)) = (
                request
                    .local_node_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                request
                    .local_workspace_path
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
            ) {
                if project.node_id.as_deref() != Some(node_id)
                    || project.workspace_path.as_deref() != Some(workspace_path)
                {
                    match project_workspace_recovery::bind_existing_pc_workspace(
                        &state,
                        &user.id,
                        &project.id,
                        &project.role,
                        node_id,
                        workspace_path,
                    )
                    .await
                    {
                        Ok(_) => {
                            if let Ok(updated) = project_access(&state, &user.id, &project.id) {
                                project = updated;
                            }
                        }
                        Err((_status, message)) => {
                            if sender
                                .send(Message::Text(WsMessage::error(message).to_json()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                            continue;
                        }
                    }
                }
            }
        }
        let attachments = request.attachments.clone();
        let message = append_project_attachment_notes(
            &state,
            &project,
            &conversation_id,
            display_message.clone(),
            request.attachments.as_deref(),
        );
        let message = match append_ui_design_task_context(
            message,
            request.ui_design_task.as_ref(),
            request.attachments.as_deref(),
            true,
        ) {
            Ok(message) => message,
            Err(message) => {
                if sender
                    .send(Message::Text(WsMessage::error(message).to_json()))
                    .await
                    .is_err()
                {
                    break;
                }
                continue;
            }
        };

        let trace_id = clean_trace_id(request.trace_id.as_deref());
        let client_request_id =
            project_client_request_id(&request, &project.id, &user.id, &conversation_id, &message);
        state.server_traces.record(
            &trace_id,
            "ws_project_message_received",
            serde_json::json!({
                "project_id": &project.id,
                "user_id": &user.id,
                "conversation_id": &conversation_id,
                "client_request_id": &client_request_id,
                "message_chars": message.chars().count(),
                "display_message_chars": display_message.chars().count(),
                "agent": request.agent.as_deref(),
                "pc_runtime_route": pc_runtime_route.map(|route| route.as_request_value()),
                "direct_pc_cli": direct_pc_cli,
                "execution_mode": request.execution_mode.as_deref(),
                "plan_mode": request.plan_mode,
            }),
        );
        let execution_mode = ProjectExecutionMode::from_request(
            request.execution_mode.as_deref(),
            request.plan_mode,
        );
        let fingerprint = project_ws_fingerprint(
            &conversation_id,
            request.agent.as_deref(),
            pc_runtime_route.map(|route| route.as_request_value()),
            direct_pc_cli,
            execution_mode.as_str(),
            request.project_icon_data_url.as_deref(),
            &message,
        );
        let job = get_or_start_project_ws_job(
            state.clone(),
            user.id.clone(),
            project.clone(),
            download_base.clone(),
            conversation_id.clone(),
            message,
            display_message,
            request.project_icon_data_url,
            request.agent,
            attachments,
            execution_mode,
            pc_runtime_route,
            direct_pc_cli,
            Some(trace_id.clone()),
            client_request_id.clone(),
            fingerprint,
        )
        .await;

        if sender
            .send(Message::Text(task_control_event(
                "accepted",
                Some(&job.task_id),
                Some(&client_request_id),
                Some(&conversation_id),
                "请求已进入任务队列。",
            )))
            .await
            .is_err()
        {
            break;
        }

        let mut job_rx = job.broadcaster.subscribe();
        let backlog = job.backlog.lock().await.clone();
        let mut replayed_terminal = false;
        let mut replay_failed = false;
        for progress in backlog {
            if sender.send(Message::Text(progress.clone())).await.is_err() {
                record_server_transport(
                    &state,
                    &trace_id,
                    "server_replay_to_phone_failed",
                    &progress,
                    &job.task_id,
                );
                replay_failed = true;
                break;
            }
            record_server_transport(
                &state,
                &trace_id,
                "server_message_replayed_to_phone",
                &progress,
                &job.task_id,
            );
            if is_terminal_project_ws_message(&progress) {
                replayed_terminal = true;
                break;
            }
        }
        if replay_failed {
            break;
        }
        if replayed_terminal {
            continue;
        }

        let mut client_disconnected = false;
        loop {
            tokio::select! {
                progress = job_rx.recv() => {
                    match progress {
                        Ok(progress) => {
                            if sender.send(Message::Text(progress.clone())).await.is_err() {
                                record_server_transport(
                                    &state,
                                    &trace_id,
                                    "server_send_to_phone_failed",
                                    &progress,
                                    &job.task_id,
                                );
                                client_disconnected = true;
                                break;
                            }
                            record_server_transport(
                                &state,
                                &trace_id,
                                "server_message_forwarded_to_phone",
                                &progress,
                                &job.task_id,
                            );
                            if should_stop_forwarding_after_send(&progress, job.finished.load(Ordering::SeqCst)) {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                incoming = receiver.next() => {
                    match incoming {
                        Some(Ok(Message::Text(text))) => {
                            let runtime_request = parse_project_message(&text);
                            let runtime_op = runtime_request
                                .op
                                .as_deref()
                                .unwrap_or("run")
                                .to_ascii_lowercase();
                            if runtime_op == "runtime_note" {
                                let note_preview = summarize_runtime_note(&runtime_request.message);
                                state.server_traces.record(
                                    &trace_id,
                                    "ws_project_runtime_note_received",
                                    serde_json::json!({
                                        "task_id": &job.task_id,
                                        "message_chars": runtime_request.message.chars().count(),
                                    }),
                                );
                                let ack = task_control_event(
                                    "runtime_note_received",
                                    Some(&job.task_id),
                                    runtime_request.client_request_id.as_deref(),
                                    Some(&conversation_id),
                                    &format!("当前任务提醒已记录：{}", note_preview),
                                );
                                emit_project_job_event(&state, &job.task_id, &job, ack).await;
                            } else {
                                tracing::info!(
                                    task_id = %job.task_id,
                                    "received project WebSocket message while request was running; ignoring {} bytes",
                                    text.len()
                                );
                            }
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            if sender.send(Message::Pong(payload)).await.is_err() {
                                client_disconnected = true;
                                break;
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {}
                        Some(Ok(Message::Close(_))) | None => {
                            client_disconnected = true;
                            break;
                        }
                        Some(Ok(Message::Binary(_))) => {}
                        Some(Err(_)) => {
                            client_disconnected = true;
                            break;
                        }
                    }
                }
                update = update_rx.recv() => {
                    if let Ok(event) = update {
                        if crate::app_update::is_newer_for_client(&event, client_version_code)
                            && sender.send(Message::Text(event)).await.is_err()
                        {
                            client_disconnected = true;
                            break;
                        }
                    }
                }
            }
        }
        if client_disconnected {
            state.server_traces.record(
                &trace_id,
                "server_client_disconnected",
                serde_json::json!({
                    "task_id": &job.task_id,
                    "background_job_continues": !job.finished.load(Ordering::SeqCst),
                }),
            );
            tracing::info!(
                task_id = %job.task_id,
                "project WebSocket disconnected while task was running; background job continues"
            );
            break;
        }
    }
}

fn summarize_runtime_note(value: &str) -> String {
    let preview = value
        .replace('\n', " ")
        .trim()
        .chars()
        .take(60)
        .collect::<String>();
    if value.chars().count() > 60 {
        format!("{}...", preview)
    } else {
        preview
    }
}

fn should_stop_forwarding_after_send(raw: &str, _job_finished: bool) -> bool {
    // A finished job can still have its terminal message queued behind the
    // last progress event. Stop this WS loop only after done/error is sent.
    is_terminal_project_ws_message(raw)
}

#[cfg(test)]
mod tests {
    use super::should_stop_forwarding_after_send;

    #[test]
    fn finished_job_does_not_stop_before_terminal_message() {
        let progress = r#"{"type":"progress","message":"syncing artifacts"}"#;
        let done = r#"{"type":"done","message":"ok"}"#;

        assert!(!should_stop_forwarding_after_send(progress, true));
        assert!(should_stop_forwarding_after_send(done, false));
        assert!(should_stop_forwarding_after_send(done, true));
    }
}

fn should_auto_bind_local_node(route: Option<PcRuntimeRoutePreference>) -> bool {
    matches!(
        route,
        Some(PcRuntimeRoutePreference::RouteA | PcRuntimeRoutePreference::RouteB)
    )
}
