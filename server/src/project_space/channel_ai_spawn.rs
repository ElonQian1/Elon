use std::{sync::Arc, time::Duration};
use tokio::sync::watch;

use crate::{
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_auth::can_edit,
    project_chat::run_project_agent_with_scheduler,
    project_execution_mode::ProjectExecutionMode,
    project_landing,
    project_space_ai_progress::{
        is_pc_cli_heartbeat_progress, pc_dispatch_started_progress, pc_tool_result_timeout_progress,
    },
    project_space_task_control::{
        register_channel_ai_task_control, remove_channel_ai_task_control,
    },
    project_space_task_result::result_message,
    project_space_task_watchdog::{channel_ai_heartbeat_only_timeout, ChannelAiPendingTools},
    project_tool_approval_recovery, project_tool_approvals,
    project_ws_protocol::enrich_project_ws_event,
    store::ProjectAccess,
    types::{AppState, WsMessage},
};

use super::channel_ai_recovery::{
    pc_cli_communication_error_result, record_recovery_started, record_recovery_timeout,
    ChannelAiRecoveryTick, ChannelAiRecoveryWatchdog,
};
use super::{publish_channel_message_updated, CHANNEL_AI_CANCEL_MESSAGE};

pub(super) struct ChannelAiTask {
    pub(crate) state: Arc<AppState>,
    pub(crate) user_id: String,
    pub(crate) project: crate::store::ProjectAccess,
    pub(crate) project_id: String,
    pub(crate) channel_id: String,
    pub(crate) conversation_id: String,
    pub(crate) task_id: String,
    pub(crate) download_base: String,
    pub(crate) content: String,
    pub(crate) agent: Option<String>,
    pub(crate) runtime_route: Option<PcRuntimeRoutePreference>,
    pub(crate) direct_pc_cli: bool,
    pub(crate) module_key: Option<String>,
    pub(crate) module_preflight_note: Option<String>,
    pub(crate) trace_id: String,
}

fn publish_channel_task_updated(task: &ChannelAiTask, kind: &str) {
    publish_channel_message_updated(
        task.state.as_ref(),
        &task.project_id,
        &task.channel_id,
        Some(&task.conversation_id),
        Some(&task.task_id),
        kind,
    );
}

pub(super) fn insert_channel_ai_progress(task: &ChannelAiTask, content: &str) {
    if task
        .state
        .store
        .insert_project_channel_message(
            &task.project_id,
            &task.channel_id,
            None,
            "ai_progress",
            content,
            Some(&task.task_id),
            None,
        )
        .is_ok()
    {
        publish_channel_task_updated(task, "ai_progress");
    }
}

pub(super) fn insert_channel_ai_result(task: &ChannelAiTask, content: &str) {
    if matches!(
        task.state.store.insert_project_channel_ai_result_once(
            &task.project_id,
            &task.channel_id,
            content,
            &task.task_id,
        ),
        Ok(true)
    ) {
        publish_channel_task_updated(task, "ai_result");
    }
}

pub(super) fn spawn_channel_ai_task(task: ChannelAiTask) {
    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        register_channel_ai_task_control(
            &task.task_id,
            &task.project_id,
            &task.channel_id,
            cancel_tx,
        );
        let run_state = task.state.clone();
        let run_project = task.project.clone();
        let run_user_id = task.user_id.clone();
        let run_conversation_id = task.conversation_id.clone();
        let run_content = task.content.clone();
        let run_agent = task.agent.clone();
        let run_runtime_route = task.runtime_route;
        let run_direct_pc_cli = task.direct_pc_cli;
        let run_module_preflight_note = task.module_preflight_note.clone();
        let run_trace_id = task.trace_id.clone();
        let download_base = task.download_base.clone();
        let runner = tokio::spawn(async move {
            run_project_agent_with_scheduler(
                run_state,
                run_user_id,
                run_project,
                download_base,
                run_conversation_id,
                run_content,
                None,
                run_agent,
                None,
                ProjectExecutionMode::Execute,
                run_runtime_route,
                run_direct_pc_cli,
                run_module_preflight_note,
                Some(run_trace_id),
                tx,
            )
            .await;
        });

        let mut final_reply = String::new();
        let mut final_status = "done".to_string();
        let mut apk_url = None;
        let mut final_done_result_pending = false;
        let mut error = None;
        let heartbeat_only_timeout = channel_ai_heartbeat_only_timeout();
        let mut recovery_watchdog = ChannelAiRecoveryWatchdog::new(heartbeat_only_timeout);
        let mut pending_tools = ChannelAiPendingTools::new();
        let mut watchdog = tokio::time::interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                raw = rx.recv() => {
                    let Some(raw) = raw else { break };
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
                        let _ = task
                            .state
                            .store
                            .record_task_event(&task.task_id, &enrich_project_ws_event(raw.clone(), &task.task_id));
                        let event_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let message = value
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim();
                        recovery_watchdog
                            .note_cli_event(is_pc_cli_heartbeat_progress(event_type, message));
                        pending_tools.note_event(event_type, &value);
                        match event_type {
                            "pc_dispatch_started" => {
                                if let Some(request_id) = value
                                    .get("pc_req_id")
                                    .or_else(|| value.get("req_id"))
                                    .and_then(|value| value.as_str())
                                    .map(str::trim)
                                    .filter(|value| !value.is_empty())
                                {
                                    if let Err(error) = task
                                        .state
                                        .store
                                        .bind_project_execution_task_id(request_id, &task.task_id)
                                    {
                                        tracing::warn!(
                                            task_id = %task.task_id,
                                            request_id,
                                            %error,
                                            "failed to bind PC completion replay to cloud task"
                                        );
                                    }
                                }
                                if let Some(content) = pc_dispatch_started_progress(&value) {
                                    insert_channel_ai_progress(&task, &content);
                                }
                            }
                            "progress" if !message.is_empty() => {
                                insert_channel_ai_progress(&task, message);
                            }
                            "tool_approval_required" => {
                                project_tool_approvals::register_required(
                                    &task.project_id,
                                    &task.channel_id,
                                    &task.task_id,
                                    &value,
                                );
                                if let Ok(content) = serde_json::to_string(&value) {
                                    insert_channel_ai_progress(&task, &content);
                                }
                            }
                            "tool_approval_decision"
                            | "tool_call"
                            | "tool_result"
                            | "runtime_status"
                            | "runtime_summary"
                            | "usage" => {
                                if let Ok(content) = serde_json::to_string(&value) {
                                    insert_channel_ai_progress(&task, &content);
                                }
                            }
                            "assistant_message" | "assistant_chunk" => {
                                let text = value
                                    .get("text")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .trim();
                                if !text.is_empty() {
                                    if let Ok(content) = serde_json::to_string(&value) {
                                        insert_channel_ai_progress(&task, &content);
                                    }
                                }
                            }
                            "done" => {
                                remove_channel_ai_task_control(&task.task_id);
                                project_tool_approvals::clear_task(&task.task_id);
                                final_reply = message.if_blank("AI 开发任务已完成。").to_string();
                                apk_url = value
                                    .get("apk_url")
                                    .and_then(|v| v.as_str())
                                    .map(ToOwned::to_owned);
                                final_done_result_pending = true;
                            }
                            "error" => {
                                remove_channel_ai_task_control(&task.task_id);
                                project_tool_approvals::clear_task(&task.task_id);
                                let msg = message.if_blank("AI 开发任务失败。").to_string();
                                let (status, reply, label) = pc_cli_communication_error_result(&msg);
                                final_status = status.to_string();
                                final_reply = reply;
                                error = Some(msg.clone());
                                insert_channel_ai_result(
                                    &task,
                                    &result_message(&final_reply, None, Some(label)),
                                );
                            }
                            _ => {}
                        }
                    }
                }
                changed = cancel_rx.changed() => {
                    if changed.is_ok() && *cancel_rx.borrow() {
                        runner.abort();
                        project_tool_approvals::clear_task(&task.task_id);
                        final_status = "canceled".to_string();
                        final_reply = CHANNEL_AI_CANCEL_MESSAGE.to_string();
                        error = Some(CHANNEL_AI_CANCEL_MESSAGE.to_string());
                        insert_channel_ai_result(
                            &task,
                            &result_message(CHANNEL_AI_CANCEL_MESSAGE, None, Some("已停止")),
                        );
                        break;
                    }
                }
                _ = watchdog.tick() => {
                    if let Some((pending_tool, timeout_secs)) = pending_tools.timed_out() {
                        runner.abort();
                        project_tool_approvals::clear_task(&task.task_id);
                        final_status = "failed".to_string();
                        let pending_label = pending_tool.label();
                        if let Some(raw_status) = pc_tool_result_timeout_progress(
                            timeout_secs,
                            pending_tool.tool(),
                            pending_tool.summary(),
                        ) {
                            let _ = task.state.store.record_task_event(
                                &task.task_id,
                                &enrich_project_ws_event(raw_status.clone(), &task.task_id),
                            );
                            insert_channel_ai_progress(&task, &raw_status);
                        }
                        let msg = format!(
                            "本机 AI 已开始执行 {}，但在 {} 秒内没有返回工具结果或最终完成事件；已停止本轮任务。请查看任务过程中的最后一条命令，必要时重启一龙 PC 节点客户端并检查 Codex 网络/代理状态后重试。",
                            pending_label,
                            timeout_secs
                        );
                        final_reply = msg.clone();
                        error = Some(msg.clone());
                        let raw_error = WsMessage::error(&msg).to_json();
                        let _ = task.state.store.record_task_event(
                            &task.task_id,
                            &enrich_project_ws_event(raw_error, &task.task_id),
                        );
                        insert_channel_ai_result(&task, &result_message(&msg, None, Some("超时")));
                        break;
                    } else if !pending_tools.has_pending() {
                        match recovery_watchdog.tick() {
                            ChannelAiRecoveryTick::Healthy => {}
                            ChannelAiRecoveryTick::StartRecovery { timeout_secs } => {
                                record_recovery_started(&task, timeout_secs);
                                continue;
                            }
                            ChannelAiRecoveryTick::RecoveryTimeout { timeout_secs, recovery_secs } => {
                                runner.abort();
                                project_tool_approvals::clear_task(&task.task_id);
                                final_status = "interrupted".to_string();
                                let msg = record_recovery_timeout(&task, timeout_secs, recovery_secs);
                                final_reply = msg.clone();
                                error = Some(msg);
                                break;
                            }
                        }
                    }
                }
            }
        }
        let runner_result = runner.await;
        if let Some(unexpected_exit) = mark_unexpected_runner_exit(
            &mut final_status,
            &mut final_reply,
            &mut error,
            final_done_result_pending,
            runner_result.as_ref().err(),
        ) {
            let raw_error = WsMessage::error(&unexpected_exit).to_json();
            let _ = task.state.store.record_task_event(
                &task.task_id,
                &enrich_project_ws_event(raw_error, &task.task_id),
            );
            insert_channel_ai_result(
                &task,
                &result_message(&unexpected_exit, None, Some("异常结束")),
            );
        }
        remove_channel_ai_task_control(&task.task_id);
        project_tool_approvals::clear_task(&task.task_id);
        if final_reply.is_empty() {
            final_reply = "AI 开发任务已结束。".to_string();
        }
        let task_finished = matches!(
            task.state.store.finish_running_task(
                &task.task_id,
                &final_status,
                Some(&final_reply),
                apk_url.as_deref(),
                error.as_deref(),
            ),
            Ok(true)
        );
        if task_finished {
            if task.module_key.as_deref() == Some(crate::store::UI_TUNER_MODULE_KEY) {
                if let Err(error) = task.state.store.record_ui_tuner_task_completion(
                    &task.task_id,
                    &final_status,
                    &final_reply,
                ) {
                    tracing::warn!(task_id = %task.task_id, %error, "ui-tuner task completion writeback failed");
                }
                let memory_state = task.state.clone();
                let memory_user_id = task.user_id.clone();
                let memory_project_id = task.project_id.clone();
                let memory_conversation_id = task.conversation_id.clone();
                let memory_user_message = task.content.clone();
                let memory_assistant_reply = final_reply.clone();
                tokio::spawn(async move {
                    crate::user_memory_extract::extract_and_save_memories_scoped(
                        memory_state,
                        memory_user_id,
                        memory_user_message,
                        memory_assistant_reply,
                        crate::store::MEMORY_SCOPE_PROJECT.to_string(),
                        Some(memory_project_id),
                        Some(memory_conversation_id),
                    )
                    .await;
                });
            }
            if final_done_result_pending {
                let result = result_message(&final_reply, apk_url.as_deref(), None);
                insert_channel_ai_result(&task, &result);
            }
            publish_channel_task_updated(&task, "conversation_result");
        }
    });
}

fn mark_unexpected_runner_exit(
    final_status: &mut String,
    final_reply: &mut String,
    error: &mut Option<String>,
    final_done_result_pending: bool,
    runner_error: Option<&tokio::task::JoinError>,
) -> Option<String> {
    if final_done_result_pending
        || !final_reply.trim().is_empty()
        || error.is_some()
        || final_status != "done"
    {
        return None;
    }

    let detail = runner_error
        .map(|join_error| format!(" 技术原因：{}。", join_error))
        .unwrap_or_default();
    let message = format!(
        "AI 开发任务异常结束：服务器没有收到 AI 的完成或错误事件，本轮未确认完成，也不会继续在后台处理。请重试任务。{}",
        detail
    );
    *final_status = "failed".to_string();
    *final_reply = message.clone();
    *error = Some(message.clone());
    Some(message)
}

#[cfg(test)]

fn can_start_channel_ai(role: &str) -> bool {
    can_edit(role)
}

pub(super) trait BlankFallback {
    fn if_blank<'a>(&'a self, fallback: &'a str) -> &'a str;
}

impl BlankFallback for str {
    fn if_blank<'a>(&'a self, fallback: &'a str) -> &'a str {
        if self.trim().is_empty() {
            fallback
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{can_start_channel_ai, mark_unexpected_runner_exit};

    #[test]
    fn channel_ai_requires_edit_role() {
        assert!(can_start_channel_ai("owner"));
        assert!(can_start_channel_ai("admin"));
        assert!(can_start_channel_ai("editor"));
        assert!(!can_start_channel_ai("member"));
        assert!(!can_start_channel_ai("observer"));
        assert!(!can_start_channel_ai("viewer"));
    }

    #[test]
    fn unexpected_runner_exit_is_recorded_as_failed() {
        let mut status = "done".to_string();
        let mut reply = String::new();
        let mut error = None;

        let message = mark_unexpected_runner_exit(&mut status, &mut reply, &mut error, false, None)
            .expect("unexpected exit should produce a failure message");

        assert_eq!(status, "failed");
        assert_eq!(error.as_deref(), Some(message.as_str()));
        assert!(message.contains("未确认完成"));
        assert!(message.contains("重试任务"));
    }

    #[test]
    fn expected_terminal_states_are_not_overwritten() {
        let mut status = "done".to_string();
        let mut reply = "已完成".to_string();
        let mut error = None;

        assert!(
            mark_unexpected_runner_exit(&mut status, &mut reply, &mut error, true, None,).is_none()
        );
        assert_eq!(status, "done");
        assert_eq!(reply, "已完成");
        assert!(error.is_none());
    }
}
