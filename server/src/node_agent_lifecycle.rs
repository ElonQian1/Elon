// server/src/node_agent_lifecycle.rs

use homecli_proto::NodeLifecycleReport;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

const SCHEMA: &str = "elon.pc_node.lifecycle.v1";
const HEARTBEAT_STALE_MS: u64 = 45_000;
const MAX_RECENT_EVENTS: usize = 80;

#[derive(Clone, Debug)]
pub(crate) struct NodeLifecycleTracker {
    path: PathBuf,
    events_path: PathBuf,
    session_id: String,
    started_at_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct LifecycleInputs<'a> {
    pub connected: bool,
    pub logged_in: bool,
    pub last_event: &'a str,
    pub active_task_count: usize,
    pub sidecar_session_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LifecycleFile {
    #[serde(default)]
    schema: String,
    #[serde(default)]
    current_session: Option<LifecycleSessionState>,
    #[serde(default)]
    previous_exit: Option<PreviousExit>,
    #[serde(default)]
    recent_events: Vec<LifecycleEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LifecycleSessionState {
    session_id: String,
    pid: u32,
    version: String,
    started_at_ms: u64,
    heartbeat_at_ms: u64,
    #[serde(default)]
    planned_shutdown_reason: Option<String>,
    #[serde(default)]
    shutdown_completed_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreviousExit {
    session_id: String,
    kind: String,
    reason: String,
    #[serde(default)]
    started_at_ms: Option<u64>,
    #[serde(default)]
    ended_at_ms: Option<u64>,
    #[serde(default)]
    last_heartbeat_at_ms: Option<u64>,
    #[serde(default)]
    heartbeat_age_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LifecycleEvent {
    at_ms: u64,
    session_id: String,
    event: String,
    detail: String,
}

impl NodeLifecycleTracker {
    pub(crate) fn start(version: &str) -> Self {
        Self::start_at(default_lifecycle_path(), version, now_ms())
    }

    fn start_at(path: PathBuf, version: &str, now: u64) -> Self {
        let session_id = format!("pcsession_{}", uuid::Uuid::new_v4().simple());
        let events_path = path.with_file_name("lifecycle-events.jsonl");
        let tracker = Self {
            path,
            events_path,
            session_id,
            started_at_ms: now,
        };
        tracker.start_session(version, now);
        tracker
    }

    pub(crate) fn record_heartbeat(&self) {
        self.update_file(|state, now| {
            if let Some(current) = state.current_session.as_mut() {
                current.heartbeat_at_ms = now;
            }
        });
    }

    pub(crate) fn mark_planned_shutdown(&self, reason: &str) {
        let reason = clean_reason(reason);
        self.update_file(|state, _now| {
            if let Some(current) = state.current_session.as_mut() {
                current.planned_shutdown_reason = Some(reason.clone());
            }
            push_event(
                state,
                &self.session_id,
                "planned_shutdown",
                &reason,
                now_ms(),
            );
        });
        self.append_event_line("planned_shutdown", &reason);
    }

    pub(crate) fn mark_shutdown_completed(&self, reason: &str) {
        let reason = clean_reason(reason);
        self.update_file(|state, now| {
            if let Some(current) = state.current_session.as_mut() {
                current.planned_shutdown_reason = Some(reason.clone());
                current.shutdown_completed_at_ms = Some(now);
            }
            push_event(state, &self.session_id, "shutdown_completed", &reason, now);
        });
        self.append_event_line("shutdown_completed", &reason);
    }

    pub(crate) fn report(&self, input: LifecycleInputs<'_>) -> NodeLifecycleReport {
        self.report_at(input, now_ms())
    }

    fn report_at(&self, input: LifecycleInputs<'_>, now: u64) -> NodeLifecycleReport {
        let state = read_lifecycle(&self.path);
        let current = state.current_session.as_ref();
        let heartbeat_at_ms = current
            .map(|session| session.heartbeat_at_ms)
            .or(Some(self.started_at_ms));
        let heartbeat_age_ms = heartbeat_at_ms.map(|heartbeat| now.saturating_sub(heartbeat));
        let previous = state.previous_exit.as_ref();
        let previous_kind = previous.map(|exit| exit.kind.as_str()).unwrap_or("");
        let restart_recovery = matches!(previous_kind, "unexpected_exit" | "stale_heartbeat")
            || input.active_task_count > 0
            || input.sidecar_session_count > 0;
        let (state_name, severity) = classify_current_state(
            input.connected,
            input.logged_in,
            heartbeat_age_ms,
            previous_kind,
        );
        let recommended_action = recommended_action(
            state_name,
            previous_kind,
            input.active_task_count,
            input.sidecar_session_count,
        );
        let summary = lifecycle_summary(
            state_name,
            input.last_event,
            previous_kind,
            input.active_task_count,
            input.sidecar_session_count,
        );

        NodeLifecycleReport {
            schema: SCHEMA.to_string(),
            session_id: Some(self.session_id.clone()),
            state: state_name.to_string(),
            severity: severity.to_string(),
            started_at_ms: current
                .map(|session| session.started_at_ms)
                .or(Some(self.started_at_ms)),
            heartbeat_at_ms,
            heartbeat_age_ms,
            connected: input.connected,
            logged_in: input.logged_in,
            last_event: non_empty(input.last_event),
            previous_session_id: previous.map(|exit| exit.session_id.clone()),
            previous_exit_kind: previous.map(|exit| exit.kind.clone()),
            previous_exit_reason: previous.map(|exit| exit.reason.clone()),
            previous_heartbeat_at_ms: previous.and_then(|exit| exit.last_heartbeat_at_ms),
            previous_heartbeat_age_ms: previous.and_then(|exit| exit.heartbeat_age_ms),
            active_task_count: input.active_task_count.min(u32::MAX as usize) as u32,
            sidecar_session_count: input.sidecar_session_count.min(u32::MAX as usize) as u32,
            restart_recovery,
            recommended_action: recommended_action.to_string(),
            summary,
        }
    }

    pub(crate) fn status_payload(&self, input: LifecycleInputs<'_>) -> Value {
        serde_json::to_value(self.report(input)).unwrap_or_else(|_| {
            json!({
                "schema": SCHEMA,
                "state": "unknown",
                "severity": "warning",
                "summary": "生命周期状态暂时无法读取。"
            })
        })
    }

    fn start_session(&self, version: &str, now: u64) {
        self.update_file_at(now, |state| {
            let previous = state
                .current_session
                .as_ref()
                .and_then(|session| previous_exit_from_session(session, now));
            if previous.is_some() {
                state.previous_exit = previous;
            }
            state.current_session = Some(LifecycleSessionState {
                session_id: self.session_id.clone(),
                pid: process::id(),
                version: version.to_string(),
                started_at_ms: now,
                heartbeat_at_ms: now,
                planned_shutdown_reason: None,
                shutdown_completed_at_ms: None,
            });
            push_event(state, &self.session_id, "process_started", version, now);
        });
        self.append_event_line("process_started", version);
    }

    fn update_file(&self, update: impl FnOnce(&mut LifecycleFile, u64)) {
        let now = now_ms();
        self.update_file_at(now, |state| update(state, now));
    }

    fn update_file_at(&self, _now: u64, update: impl FnOnce(&mut LifecycleFile)) {
        let mut state = read_lifecycle(&self.path);
        if state.schema.trim().is_empty() {
            state.schema = SCHEMA.to_string();
        }
        update(&mut state);
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(&state) {
            let _ = fs::write(&self.path, text);
        }
    }

    fn append_event_line(&self, event: &str, detail: &str) {
        if let Some(parent) = self.events_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let line = json!({
            "at_ms": now_ms(),
            "session_id": self.session_id.as_str(),
            "event": event,
            "detail": truncate(detail, 300),
        })
        .to_string();
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)
        {
            let _ = writeln!(file, "{line}");
        }
    }
}

pub(crate) fn spawn_heartbeat(tracker: NodeLifecycleTracker) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            tracker.record_heartbeat();
        }
    });
}

pub(crate) async fn runtime_report(
    runtime: &super::NodeRuntime,
    connected: bool,
    logged_in: bool,
    last_event: &str,
) -> NodeLifecycleReport {
    let active_task_count = runtime
        .active_cli_prompts
        .views_without_approvals()
        .await
        .len();
    let sidecar_session_count = runtime
        .cli_sidecars
        .latest_sessions(20)
        .map(|sessions| sessions.len())
        .unwrap_or(0);
    runtime.lifecycle.report(LifecycleInputs {
        connected,
        logged_in,
        last_event,
        active_task_count,
        sidecar_session_count,
    })
}

fn default_lifecycle_path() -> PathBuf {
    super::state_path().with_file_name("lifecycle-state.json")
}

fn read_lifecycle(path: &Path) -> LifecycleFile {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| LifecycleFile {
            schema: SCHEMA.to_string(),
            ..Default::default()
        })
}

fn previous_exit_from_session(session: &LifecycleSessionState, now: u64) -> Option<PreviousExit> {
    let heartbeat_age_ms = now.saturating_sub(session.heartbeat_at_ms);
    if let Some(completed_at) = session.shutdown_completed_at_ms {
        return Some(PreviousExit {
            session_id: session.session_id.clone(),
            kind: planned_kind(session.planned_shutdown_reason.as_deref()).to_string(),
            reason: planned_reason(session.planned_shutdown_reason.as_deref()).to_string(),
            started_at_ms: Some(session.started_at_ms),
            ended_at_ms: Some(completed_at),
            last_heartbeat_at_ms: Some(session.heartbeat_at_ms),
            heartbeat_age_ms: Some(heartbeat_age_ms),
        });
    }

    if let Some(reason) = session.planned_shutdown_reason.as_deref() {
        return Some(PreviousExit {
            session_id: session.session_id.clone(),
            kind: planned_kind(Some(reason)).to_string(),
            reason: planned_reason(Some(reason)).to_string(),
            started_at_ms: Some(session.started_at_ms),
            ended_at_ms: None,
            last_heartbeat_at_ms: Some(session.heartbeat_at_ms),
            heartbeat_age_ms: Some(heartbeat_age_ms),
        });
    }

    Some(PreviousExit {
        session_id: session.session_id.clone(),
        kind: "unexpected_exit".to_string(),
        reason: "上次 Win 端没有写入正常退出记录，可能是闪退、被任务管理器结束、系统重启或断电。"
            .to_string(),
        started_at_ms: Some(session.started_at_ms),
        ended_at_ms: None,
        last_heartbeat_at_ms: Some(session.heartbeat_at_ms),
        heartbeat_age_ms: Some(heartbeat_age_ms),
    })
}

fn classify_current_state(
    connected: bool,
    logged_in: bool,
    heartbeat_age_ms: Option<u64>,
    previous_kind: &str,
) -> (&'static str, &'static str) {
    if heartbeat_age_ms.is_some_and(|age| age > HEARTBEAT_STALE_MS) {
        return ("stale_heartbeat", "danger");
    }
    if !logged_in {
        return ("needs_login", "warning");
    }
    if connected {
        if previous_kind == "unexpected_exit" {
            return ("recovered_after_unexpected_exit", "warning");
        }
        return ("healthy", "ok");
    }
    ("cloud_disconnected", "warning")
}

fn recommended_action(
    state: &str,
    previous_kind: &str,
    active_task_count: usize,
    sidecar_session_count: usize,
) -> &'static str {
    if state == "stale_heartbeat" {
        return "restart_client";
    }
    if active_task_count > 0 || sidecar_session_count > 0 {
        return "review_task_recovery";
    }
    if previous_kind == "unexpected_exit" {
        return "review_previous_session";
    }
    match state {
        "needs_login" => "login",
        "cloud_disconnected" => "wait_or_reconnect",
        _ => "none",
    }
}

fn lifecycle_summary(
    state: &str,
    last_event: &str,
    previous_kind: &str,
    active_task_count: usize,
    sidecar_session_count: usize,
) -> String {
    if state == "stale_heartbeat" {
        return "本机管理接口可响应，但生命周期心跳已过期，疑似卡住或后台任务调度停止。"
            .to_string();
    }
    if active_task_count > 0 || sidecar_session_count > 0 {
        return format!(
            "检测到 {active_task_count} 个运行中任务、{sidecar_session_count} 个 sidecar 会话，重连后应先查看恢复面板。"
        );
    }
    if previous_kind == "unexpected_exit" {
        return "上次 Win 端可能异常退出；本轮已重新启动，建议查看任务日志确认是否需要恢复。"
            .to_string();
    }
    match state {
        "healthy" => "Win 端在线，云端连接正常。".to_string(),
        "recovered_after_unexpected_exit" => {
            "Win 端已重连，但上一轮会话异常结束，建议确认未完成任务。".to_string()
        }
        "needs_login" => "Win 端已启动，但还没有绑定一龙账号。".to_string(),
        "cloud_disconnected" => {
            let suffix = non_empty(last_event).unwrap_or_else(|| "等待云端连接恢复".to_string());
            format!("Win 端本机进程正常，云端连接暂未就绪：{suffix}")
        }
        _ => "生命周期状态暂时无法判断。".to_string(),
    }
}

fn planned_kind(reason: Option<&str>) -> &'static str {
    match reason.unwrap_or_default() {
        "update" | "auto_update" | "client_update" => "planned_update",
        "uninstall" => "planned_uninstall",
        "restart" => "planned_restart",
        "user_interrupt" | "user_exit" => "user_closed",
        _ => "planned_shutdown",
    }
}

fn planned_reason(reason: Option<&str>) -> &'static str {
    match planned_kind(reason) {
        "planned_update" => "上次 Win 端进入计划内升级/重启流程。",
        "planned_uninstall" => "上次 Win 端进入用户触发的卸载流程。",
        "planned_restart" => "上次 Win 端进入计划内重启流程。",
        "user_closed" => "上次 Win 端收到用户关闭或中断信号。",
        _ => "上次 Win 端进入计划内关闭流程。",
    }
}

fn push_event(state: &mut LifecycleFile, session_id: &str, event: &str, detail: &str, at_ms: u64) {
    state.recent_events.push(LifecycleEvent {
        at_ms,
        session_id: session_id.to_string(),
        event: event.to_string(),
        detail: truncate(detail, 300),
    });
    if state.recent_events.len() > MAX_RECENT_EVENTS {
        let excess = state.recent_events.len() - MAX_RECENT_EVENTS;
        state.recent_events.drain(0..excess);
    }
}

fn clean_reason(reason: &str) -> String {
    truncate(reason.trim(), 80)
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        value.chars().take(max_chars).collect()
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "node_agent_lifecycle_tests.rs"]
mod node_agent_lifecycle_tests;
