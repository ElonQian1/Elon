// server/src/project_ws_job.rs
//! 项目 WebSocket 任务调度器：管理同一项目/用户/请求的运行中任务。
//!
//! 从 `project_api.rs` 抽出，避免单文件继续膨胀。`handle_project_ws` 调用
//! `get_or_start_project_ws_job` 创建或附加任务；`cancel_project_ws_job` 取消任务。
//! 其余函数 (`run_project_ws_job` / `emit_project_job_event` / `schedule_project_job_cleanup`)
//! 是模块内部细节。

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock,
    },
    time::Duration,
};
use tokio::sync::{broadcast, watch, Mutex};

use crate::{
    billing,
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_chat::run_project_agent_with_scheduler,
    project_execution_mode::ProjectExecutionMode,
    project_keys::project_ws_job_key,
    project_trace_events::record_server_message,
    project_ws_protocol::{
        enrich_project_ws_event, is_terminal_project_ws_message, is_terminal_task_status,
        task_control_event, terminal_backlog_from_task, ProjectAttachmentRef,
        PROJECT_WS_BACKLOG_LIMIT,
    },
    store::ProjectAccess,
    types::{AppState, WsMessage},
};

pub(crate) static PROJECT_WS_JOBS: LazyLock<Mutex<HashMap<String, Arc<ProjectWsJob>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) struct ProjectWsJob {
    pub(crate) key: String,
    pub(crate) fingerprint: String,
    pub(crate) task_id: String,
    pub(crate) trace_id: Option<String>,
    pub(crate) cancel_tx: watch::Sender<bool>,
    pub(crate) backlog: Mutex<Vec<String>>,
    pub(crate) broadcaster: broadcast::Sender<String>,
    pub(crate) finished: AtomicBool,
}

pub(crate) async fn get_or_start_project_ws_job(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    download_base: String,
    conversation_id: String,
    message: String,
    project_icon_data_url: Option<String>,
    agent_name: Option<String>,
    attachments: Option<Vec<ProjectAttachmentRef>>,
    execution_mode: ProjectExecutionMode,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    trace_id: Option<String>,
    client_request_id: String,
    fingerprint: String,
) -> Arc<ProjectWsJob> {
    let key = project_ws_job_key(&project.id, &user_id, &conversation_id, &client_request_id);
    let mut jobs = PROJECT_WS_JOBS.lock().await;
    if let Some(existing) = jobs.get(&key) {
        if existing.fingerprint == fingerprint {
            if let Some(trace_id) = trace_id.as_deref() {
                state.server_traces.record(
                    trace_id,
                    "ws_project_join_existing_job",
                    serde_json::json!({
                        "task_id": &existing.task_id,
                        "finished": existing.finished.load(Ordering::SeqCst),
                    }),
                );
            }
            return existing.clone();
        }
        if !existing.finished.load(Ordering::SeqCst) {
            if let Some(trace_id) = trace_id.as_deref() {
                state.server_traces.record(
                    trace_id,
                    "ws_project_attach_running_job",
                    serde_json::json!({
                        "task_id": &existing.task_id,
                        "reason": "different_fingerprint",
                    }),
                );
            }
            let notice =
                WsMessage::progress("同一个请求仍在后台处理，正在继续同步已有任务进度。").to_json();
            let _ = existing.broadcaster.send(notice);
            return existing.clone();
        }
        jobs.remove(&key);
    }

    let persisted = state
        .store
        .get_task_by_client_request(
            &project.id,
            &user_id,
            Some(&conversation_id),
            &client_request_id,
        )
        .ok()
        .flatten();
    if let Some(task) = persisted
        .as_ref()
        .filter(|task| is_terminal_task_status(&task.status))
    {
        let events = state
            .store
            .list_task_events(&task.id, PROJECT_WS_BACKLOG_LIMIT)
            .unwrap_or_default();
        let backlog = terminal_backlog_from_task(task, events);
        let (broadcast_tx, _) = broadcast::channel::<String>(256);
        let (cancel_tx, _cancel_rx) = watch::channel(false);
        let job = Arc::new(ProjectWsJob {
            key: key.clone(),
            fingerprint,
            task_id: task.id.clone(),
            trace_id: trace_id.clone(),
            cancel_tx,
            backlog: Mutex::new(backlog),
            broadcaster: broadcast_tx,
            finished: AtomicBool::new(true),
        });
        if let Some(trace_id) = trace_id.as_deref() {
            state.server_traces.record(
                trace_id,
                "ws_project_restore_terminal_task",
                serde_json::json!({
                    "task_id": &task.id,
                    "status": &task.status,
                }),
            );
        }
        jobs.insert(key.clone(), job.clone());
        schedule_project_job_cleanup(key, job.clone());
        return job;
    }

    if let Err(msg) = billing::check_can_call(&state.store, &user_id) {
        let raw = WsMessage::error(msg).to_json();
        let (broadcast_tx, _) = broadcast::channel::<String>(256);
        let (cancel_tx, _cancel_rx) = watch::channel(false);
        let job = Arc::new(ProjectWsJob {
            key: key.clone(),
            fingerprint,
            task_id: "tsk_billing_blocked".into(),
            trace_id: trace_id.clone(),
            cancel_tx,
            backlog: Mutex::new(vec![raw]),
            broadcaster: broadcast_tx,
            finished: AtomicBool::new(true),
        });
        jobs.insert(key.clone(), job.clone());
        schedule_project_job_cleanup(key, job.clone());
        return job;
    }

    let (task_id, restart_notice) = if let Some(task) = persisted {
        let notice = if task.status == "interrupted" {
            Some("上次任务被服务器重启中断，正在用同一个任务记录继续处理。".to_string())
        } else {
            Some("正在恢复服务器中已有的运行中任务。".to_string())
        };
        let _ = state.store.set_task_running(&task.id);
        (task.id, notice)
    } else {
        match state.store.create_task_with_client_request(
            &project.id,
            &user_id,
            Some(&conversation_id),
            Some(&client_request_id),
            &message,
        ) {
            Ok(task_id) => (task_id, None),
            Err(error) => {
                let raw = WsMessage::error(format!("创建任务记录失败: {}", error)).to_json();
                let (broadcast_tx, _) = broadcast::channel::<String>(256);
                let (cancel_tx, _cancel_rx) = watch::channel(false);
                let job = Arc::new(ProjectWsJob {
                    key: key.clone(),
                    fingerprint,
                    task_id: "tsk_unknown".into(),
                    trace_id: trace_id.clone(),
                    cancel_tx,
                    backlog: Mutex::new(vec![raw]),
                    broadcaster: broadcast_tx,
                    finished: AtomicBool::new(true),
                });
                jobs.insert(key.clone(), job.clone());
                schedule_project_job_cleanup(key, job.clone());
                return job;
            }
        }
    };

    let (broadcast_tx, _) = broadcast::channel::<String>(256);
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let job = Arc::new(ProjectWsJob {
        key: key.clone(),
        fingerprint,
        task_id: task_id.clone(),
        trace_id: trace_id.clone(),
        cancel_tx,
        backlog: Mutex::new(Vec::new()),
        broadcaster: broadcast_tx,
        finished: AtomicBool::new(false),
    });
    jobs.insert(key.clone(), job.clone());

    let job_for_task = job.clone();
    tokio::spawn(async move {
        run_project_ws_job(
            state,
            user_id,
            project,
            download_base,
            conversation_id,
            message,
            project_icon_data_url,
            agent_name,
            attachments,
            execution_mode,
            pc_runtime_route,
            trace_id,
            task_id,
            job_for_task,
            restart_notice,
            cancel_rx,
        )
        .await;
    });

    job
}

async fn run_project_ws_job(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    download_base: String,
    conversation_id: String,
    message: String,
    project_icon_data_url: Option<String>,
    agent_name: Option<String>,
    attachments: Option<Vec<ProjectAttachmentRef>>,
    execution_mode: ProjectExecutionMode,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    trace_id: Option<String>,
    task_id: String,
    job: Arc<ProjectWsJob>,
    restart_notice: Option<String>,
    mut cancel_rx: watch::Receiver<bool>,
) {
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "ws_project_job_start",
            serde_json::json!({
                "task_id": &task_id,
                "project_id": &project.id,
                "conversation_id": &conversation_id,
                "message_chars": message.chars().count(),
                "agent": agent_name.as_deref(),
                "pc_runtime_route": pc_runtime_route.map(|route| route.as_request_value()),
                "execution_mode": execution_mode.as_str(),
            }),
        );
    }
    if let Some(message) = restart_notice {
        emit_project_job_event(
            &state,
            &task_id,
            &job,
            WsMessage::progress(message).to_json(),
        )
        .await;
    }

    // 软锁定：首次任务执行时记录 agent_name，后续切换时 APK 可据此提示用户
    if let Some(agent) = agent_name.as_deref().filter(|s| !s.is_empty()) {
        let _ = state.store.set_conversation_locked_agent_if_unset(
            &project.id,
            &user_id,
            &conversation_id,
            agent,
        );
    }

    emit_project_job_event(
        &state,
        &task_id,
        &job,
        task_control_event(
            "started",
            Some(&task_id),
            None,
            Some(&conversation_id),
            "任务开始执行。",
        ),
    )
    .await;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let state_clone = state.clone();
    let project_for_task = project.clone();
    let task_conversation_id = conversation_id.clone();
    let task_message = message.clone();
    let task_project_icon_data_url = project_icon_data_url.clone();
    let task_agent_name = agent_name.clone();
    let task_attachments = attachments.clone();
    let task_pc_runtime_route = pc_runtime_route;
    let task_trace_id = trace_id.clone();
    let agent_task = tokio::spawn(async move {
        run_project_agent_with_scheduler(
            state_clone,
            user_id,
            project_for_task,
            download_base,
            task_conversation_id,
            task_message,
            task_project_icon_data_url,
            task_agent_name,
            task_attachments,
            execution_mode,
            task_pc_runtime_route,
            task_trace_id,
            tx,
        )
        .await;
    });

    let mut reply = String::new();
    let mut apk_url = None;
    let mut error = None;
    let mut saw_terminal = false;
    loop {
        tokio::select! {
            changed = cancel_rx.changed() => {
                if changed.is_ok() && *cancel_rx.borrow() {
                    agent_task.abort();
                    let msg = "任务已取消。".to_string();
                    emit_project_job_event(
                        &state,
                        &task_id,
                        &job,
                        task_control_event(
                            "canceled",
                            Some(&task_id),
                            None,
                            Some(&conversation_id),
                            &msg,
                        ),
                    )
                    .await;
                    emit_project_job_event(
                        &state,
                        &task_id,
                        &job,
                        WsMessage::error(msg.clone())
                        .to_json(),
                    )
                    .await;
                    reply = msg.clone();
                    error = Some(msg);
                    saw_terminal = true;
                    break;
                }
            }
            next = rx.recv() => {
                let Some(progress) = next else {
                    break;
                };
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&progress) {
            match value.get("type").and_then(|t| t.as_str()) {
                Some("done") => {
                    reply = value["message"].as_str().unwrap_or("完成").to_string();
                    apk_url = value["apk_url"].as_str().map(ToOwned::to_owned);
                    saw_terminal = true;
                }
                Some("error") => {
                    let msg = value["message"].as_str().unwrap_or("发生错误").to_string();
                    reply = msg.clone();
                    error = Some(msg);
                    saw_terminal = true;
                }
                _ => {}
            }
        }
        let terminal = is_terminal_project_ws_message(&progress);
        emit_project_job_event(&state, &task_id, &job, progress).await;
        if terminal {
            break;
        }
            }
        }
    }
    let _ = agent_task.await;

    if !saw_terminal {
        let msg = "任务没有返回最终结果，请稍后重试或查看服务端日志。".to_string();
        let raw = WsMessage::error(msg.clone()).to_json();
        emit_project_job_event(&state, &task_id, &job, raw).await;
        reply = msg.clone();
        error = Some(msg);
    }

    let status = if error.is_some() { "failed" } else { "done" };
    let _ = state.store.finish_task(
        &task_id,
        status,
        Some(&reply),
        apk_url.as_deref(),
        error.as_deref(),
    );
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            if error.is_some() {
                "ws_project_task_failed"
            } else {
                "ws_project_task_done"
            },
            serde_json::json!({
                "task_id": &task_id,
                "status": status,
                "has_apk_url": apk_url.is_some(),
            }),
        );
    }
    job.finished.store(true, Ordering::SeqCst);
    schedule_project_job_cleanup(job.key.clone(), job);
}

pub(crate) async fn emit_project_job_event(
    state: &AppState,
    task_id: &str,
    job: &Arc<ProjectWsJob>,
    raw: String,
) {
    let raw = enrich_project_ws_event(raw, task_id);
    let ephemeral = is_ephemeral_progress_event(&raw);
    if !ephemeral {
        {
            let mut backlog = job.backlog.lock().await;
            backlog.push(raw.clone());
            if backlog.len() > PROJECT_WS_BACKLOG_LIMIT {
                let overflow = backlog.len() - PROJECT_WS_BACKLOG_LIMIT;
                backlog.drain(0..overflow);
            }
        }
        let _ = state.store.record_task_event(task_id, &raw);
    }
    if let Some(trace_id) = job.trace_id.as_deref() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
            record_server_message(state, trace_id, &value, raw.len());
        }
    }
    let _ = job.broadcaster.send(raw);
}

fn is_ephemeral_progress_event(raw: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return false;
    };
    if value.get("type").and_then(|value| value.as_str()) != Some("progress") {
        return false;
    }
    let Some(message) = value.get("message").and_then(|value| value.as_str()) else {
        return false;
    };
    is_cli_heartbeat_message(message)
}

fn is_cli_heartbeat_message(message: &str) -> bool {
    let clean = message.split_whitespace().collect::<Vec<_>>().join(" ");
    clean.starts_with("CLI 仍在运行")
        || clean.starts_with("AI 还在思考（已等待 ")
        || clean.starts_with("AI 还在后台处理（已等待 ")
        || clean.contains(" 正在处理中…（已等待 ") && clean.ends_with("s）")
}

pub(crate) async fn cancel_project_ws_job(
    project_id: &str,
    user_id: &str,
    conversation_id: &str,
    task_id: Option<&str>,
    client_request_id: Option<&str>,
) -> Option<String> {
    let jobs = PROJECT_WS_JOBS.lock().await;
    if let Some(task_id) = task_id {
        for job in jobs.values() {
            if job.task_id == task_id && !job.finished.load(Ordering::SeqCst) {
                let _ = job.cancel_tx.send(true);
                return Some(job.task_id.clone());
            }
        }
    }
    if let Some(client_request_id) = client_request_id {
        let key = project_ws_job_key(project_id, user_id, conversation_id, client_request_id);
        if let Some(job) = jobs.get(&key) {
            if !job.finished.load(Ordering::SeqCst) {
                let _ = job.cancel_tx.send(true);
                return Some(job.task_id.clone());
            }
        }
    }
    None
}

fn schedule_project_job_cleanup(key: String, job: Arc<ProjectWsJob>) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30 * 60)).await;
        let mut jobs = PROJECT_WS_JOBS.lock().await;
        if jobs
            .get(&key)
            .map(|existing| Arc::ptr_eq(existing, &job))
            .unwrap_or(false)
        {
            jobs.remove(&key);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::is_ephemeral_progress_event;
    use crate::types::WsMessage;

    #[test]
    fn detects_model_processing_heartbeat_as_ephemeral() {
        let raw = WsMessage::progress("Codex (GPT-5.5 · 推理 xhigh) 正在处理中…（已等待 235s）")
            .to_json();

        assert!(is_ephemeral_progress_event(&raw));
    }

    #[test]
    fn detects_silent_cli_heartbeat_as_ephemeral() {
        let raw =
            WsMessage::progress("AI 还在后台处理（已等待 120 秒，本轮已静默 80 秒）…").to_json();

        assert!(is_ephemeral_progress_event(&raw));
    }

    #[test]
    fn keeps_meaningful_progress_events_in_history() {
        let raw = WsMessage::progress("代码修改完成，正在提交并合并…").to_json();

        assert!(!is_ephemeral_progress_event(&raw));
    }
}
