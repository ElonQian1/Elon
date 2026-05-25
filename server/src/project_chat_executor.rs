//! 项目级 AI agent 在隔离执行 worktree 中的运行循环（从 project_chat.rs 抽出）。
//!
//! 这里只负责：从 ProjectConversationWorkspace 启动 agent 任务、转发 WS 事件、
//! 在结束时回收 worktree。project_chat.rs 调用本模块发起执行。

use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    agent,
    project_completion::ensure_done_event_has_project_apk_url,
    project_conversation_workspace::{
        merge_conversation_worktree, project_merge_execution_key, ProjectConversationWorkspace,
    },
    project_ws_protocol::{is_done_project_ws_message, is_terminal_project_ws_message},
    store::ProjectAccess,
    types::{AppState, WsMessage},
};
pub(crate) async fn run_project_agent_in_execution_workspace(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    download_base: String,
    conversation_id: String,
    message: String,
    agent_name: Option<String>,
    trace_id: Option<String>,
    execution_workspace: ProjectConversationWorkspace,
    tx: UnboundedSender<String>,
) {
    let (agent_tx, mut agent_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let agent_state = state.clone();
    let agent_user_id = user_id.clone();
    let agent_project = project.clone();
    let agent_download_base = download_base.clone();
    let agent_conversation_id = conversation_id.clone();
    let agent_message = message.clone();
    let agent_name_for_task = agent_name.clone();
    let agent_trace_id = trace_id.clone();
    let agent_workspace = execution_workspace.active_workspace.clone();

    let agent_task = tokio::spawn(async move {
        agent::run_for_project_in_workspace(
            &agent_user_id,
            &agent_project,
            &agent_workspace,
            &agent_download_base,
            Some(&agent_conversation_id),
            &agent_message,
            agent_name_for_task.as_deref(),
            agent_trace_id.as_deref(),
            &agent_state,
            agent_tx,
        )
        .await;
    });

    let mut terminal_raw = None;
    let mut terminal_is_done = false;
    while let Some(raw) = agent_rx.recv().await {
        if is_terminal_project_ws_message(&raw) {
            terminal_is_done = is_done_project_ws_message(&raw);
            terminal_raw = Some(raw);
            break;
        }
        let _ = tx.send(raw);
    }

    if let Err(error) = agent_task.await {
        let _ = tx.send(
            WsMessage::Error {
                message: format!("AI 任务异常结束: {}", error),
            }
            .to_json(),
        );
        return;
    }

    while let Ok(raw) = agent_rx.try_recv() {
        if is_terminal_project_ws_message(&raw) {
            terminal_is_done = is_done_project_ws_message(&raw);
            terminal_raw = Some(raw);
        } else {
            let _ = tx.send(raw);
        }
    }

    if terminal_is_done && execution_workspace.is_isolated() {
        let merge_key = project_merge_execution_key(&project.id);
        let merge_tx = tx.clone();
        let merge_state = state.clone();
        let merge_trace_id = trace_id.clone();
        let merge_project_id = project.id.clone();
        let merge_permit = state
            .project_task_scheduler
            .acquire(&merge_key, move || {
                if let Some(trace_id) = merge_trace_id.as_deref() {
                    merge_state.server_traces.record(
                        trace_id,
                        "server_project_merge_queue_wait",
                        serde_json::json!({ "project_id": &merge_project_id }),
                    );
                }
                let _ = merge_tx.send(
                    WsMessage::Progress {
                        message: "代码已在会话分支完成，正在等待项目合并锁。".into(),
                    }
                    .to_json(),
                );
            })
            .await;
        let _keep_merge_permit = merge_permit;
        let _ = tx.send(
            WsMessage::Progress {
                message: "正在把会话分支串行合并回项目主工作区。".into(),
            }
            .to_json(),
        );
        match merge_conversation_worktree(&execution_workspace) {
            Ok(summary) => {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_project_merge_done",
                        serde_json::json!({
                            "project_id": &project.id,
                            "conversation_id": &conversation_id,
                            "summary": summary,
                        }),
                    );
                }
                let _ = tx.send(WsMessage::Progress { message: summary }.to_json());
            }
            Err(error) => {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_project_merge_failed",
                        serde_json::json!({
                            "project_id": &project.id,
                            "conversation_id": &conversation_id,
                            "error": error.to_string(),
                        }),
                    );
                }
                let _ = tx.send(
                    WsMessage::Error {
                        message: format!(
                            "会话代码已完成，但合并回项目主分支失败: {}。请处理冲突后重试。",
                            error
                        ),
                    }
                    .to_json(),
                );
                return;
            }
        }
    }

    if terminal_is_done {
        if let Some(raw) = terminal_raw.take() {
            let original = raw.clone();
            let mut workspaces = vec![execution_workspace.active_workspace.as_path()];
            if execution_workspace.is_isolated() {
                workspaces.insert(0, execution_workspace.base_workspace.as_path());
            }
            let (raw, apk_url) =
                ensure_done_event_has_project_apk_url(raw, &download_base, &workspaces);
            if raw != original {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_project_done_apk_url_filled",
                        serde_json::json!({
                            "project_id": &project.id,
                            "conversation_id": &conversation_id,
                            "apk_url": apk_url,
                        }),
                    );
                }
            }
            terminal_raw = Some(raw);
        }
    }

    if let Some(raw) = terminal_raw {
        let _ = tx.send(raw);
    } else {
        let _ = tx.send(
            WsMessage::Error {
                message: "AI 任务结束但没有返回完成状态。".into(),
            }
            .to_json(),
        );
    }
}