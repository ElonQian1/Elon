//! Allowlisted semantic actions for Codex-driven Win client debugging.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{State, WebviewWindow};

const MAX_NATIVE_EVENTS: usize = 600;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SemanticAction {
    pub action_id: String,
    #[serde(default)]
    pub trace_id: String,
    pub kind: String,
    #[serde(default)]
    pub route: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct NativeEvent {
    seq: u64,
    event_id: String,
    trace_id: String,
    level: String,
    kind: String,
    summary: String,
    at_ms: u128,
    fields: Value,
}

#[derive(Default)]
struct BridgeState {
    next_seq: u64,
    events: VecDeque<NativeEvent>,
}

#[derive(Default)]
pub(crate) struct CodexSemanticBridge {
    state: Mutex<BridgeState>,
}

impl CodexSemanticBridge {
    fn record(
        &self,
        trace_id: &str,
        level: &str,
        kind: &str,
        summary: &str,
        fields: Value,
    ) -> NativeEvent {
        let mut state = lock(&self.state);
        state.next_seq = state.next_seq.saturating_add(1);
        let event = NativeEvent {
            seq: state.next_seq,
            event_id: format!("tauri_evt_{}_{}", now_ms(), state.next_seq),
            trace_id: clean_identifier(trace_id, "tauri"),
            level: normalize_level(level).to_string(),
            kind: clean_kind(kind),
            summary: truncate(summary.trim(), 500),
            at_ms: now_ms(),
            fields,
        };
        state.events.push_back(event.clone());
        while state.events.len() > MAX_NATIVE_EVENTS {
            state.events.pop_front();
        }
        event
    }

    fn read(&self, after: u64, limit: usize) -> Vec<NativeEvent> {
        lock(&self.state)
            .events
            .iter()
            .filter(|event| event.seq > after)
            .take(limit.clamp(1, 200))
            .cloned()
            .collect()
    }
}

#[tauri::command]
pub(crate) fn codex_win_capabilities(
    window: WebviewWindow,
    bridge: State<'_, CodexSemanticBridge>,
) -> Value {
    bridge.record(
        "tauri_heartbeat",
        "debug",
        "bridge.heartbeat",
        "Tauri 语义桥在线",
        json!({"window_label": window.label()}),
    );
    json!({
        "schema":"elon.tauri_codex_bridge.v1",
        "available":true,
        "window_label":window.label(),
        "actions":["show_window","focus_window","navigate","reload_page","open_devtools","close_devtools","capture_state"],
        "devtools_supported":cfg!(debug_assertions),
        "arbitrary_javascript":false,
        "arbitrary_command":false,
        "arbitrary_url":false,
    })
}

#[tauri::command]
pub(crate) fn codex_execute_semantic_action(
    window: WebviewWindow,
    bridge: State<'_, CodexSemanticBridge>,
    action: SemanticAction,
) -> Result<Value, String> {
    validate_action(&action)?;
    let result = execute(&window, &action);
    let captured_state = (action.kind == "capture_state").then(|| window_state(&window));
    let (status, level, message) = match &result {
        Ok(message) => ("succeeded", "info", message.clone()),
        Err(error) => ("failed", "error", error.clone()),
    };
    bridge.record(
        &action.trace_id,
        level,
        "action.executed",
        &format!("{}: {}", action.kind, status),
        json!({"action_id": action.action_id, "kind": action.kind, "status": status, "route": action.route, "window_state": captured_state}),
    );
    result.map(|message| {
        json!({
            "schema":"elon.tauri_codex_action_receipt.v1",
            "action_id":action.action_id,
            "trace_id":clean_identifier(&action.trace_id, "tauri"),
            "status":"succeeded",
            "message":message,
            "route":action.route,
            "window_state":captured_state,
            "at_ms":now_ms(),
        })
    })
}

#[tauri::command]
pub(crate) fn codex_read_native_events(
    bridge: State<'_, CodexSemanticBridge>,
    after: Option<u64>,
    limit: Option<usize>,
) -> Value {
    let events = bridge.read(after.unwrap_or(0), limit.unwrap_or(100));
    let next_after = events
        .last()
        .map(|event| event.seq)
        .unwrap_or(after.unwrap_or(0));
    json!({
        "schema":"elon.tauri_codex_events.v1",
        "events":events,
        "next_after":next_after,
    })
}

fn execute(window: &WebviewWindow, action: &SemanticAction) -> Result<String, String> {
    match action.kind.as_str() {
        "show_window" => {
            window.show().map_err(display_error)?;
            Ok("主窗口已显示".to_string())
        }
        "focus_window" => {
            window.show().map_err(display_error)?;
            window.set_focus().map_err(display_error)?;
            Ok("主窗口已显示并聚焦".to_string())
        }
        "navigate" => {
            let route = action.route.as_deref().ok_or("navigate 缺少 route")?;
            let encoded = serde_json::to_string(route).map_err(display_error)?;
            window
                .eval(&format!(
                    "window.history.pushState({{}}, '', {encoded}); window.dispatchEvent(new PopStateEvent('popstate'));"
                ))
                .map_err(display_error)?;
            Ok(format!("已导航到 {route}"))
        }
        "reload_page" => {
            window
                .eval("window.setTimeout(function () { window.location.reload(); }, 500);")
                .map_err(display_error)?;
            Ok("页面刷新将在回执写回后触发".to_string())
        }
        "open_devtools" => devtools(window, true),
        "close_devtools" => devtools(window, false),
        "capture_state" => Ok("已捕获非秘密窗口状态".to_string()),
        _ => Err("不支持的 Tauri 语义动作".to_string()),
    }
}

fn window_state(window: &WebviewWindow) -> Value {
    json!({
        "label": window.label(),
        "visible": window.is_visible().ok(),
        "focused": window.is_focused().ok(),
        "maximized": window.is_maximized().ok(),
        "minimized": window.is_minimized().ok(),
        "devtools_open": devtools_open(window),
    })
}

#[cfg(debug_assertions)]
fn devtools_open(window: &WebviewWindow) -> Option<bool> {
    Some(window.is_devtools_open())
}

#[cfg(not(debug_assertions))]
fn devtools_open(_window: &WebviewWindow) -> Option<bool> {
    None
}

#[cfg(debug_assertions)]
fn devtools(window: &WebviewWindow, open: bool) -> Result<String, String> {
    if open {
        window.open_devtools();
        Ok("DevTools 已打开".to_string())
    } else {
        window.close_devtools();
        Ok("DevTools 已关闭".to_string())
    }
}

#[cfg(not(debug_assertions))]
fn devtools(_window: &WebviewWindow, _open: bool) -> Result<String, String> {
    Err("当前生产壳未启用 DevTools；能力已明确标记为不可用。".to_string())
}

fn validate_action(action: &SemanticAction) -> Result<(), String> {
    if action.action_id.trim().is_empty() || action.action_id.len() > 100 {
        return Err("action_id 无效".to_string());
    }
    if !matches!(
        action.kind.as_str(),
        "show_window"
            | "focus_window"
            | "navigate"
            | "reload_page"
            | "open_devtools"
            | "close_devtools"
            | "capture_state"
    ) {
        return Err("动作不在 Tauri 白名单".to_string());
    }
    if action.kind == "navigate" {
        validate_route(action.route.as_deref().ok_or("navigate 缺少 route")?)?;
    } else if action
        .route
        .as_deref()
        .is_some_and(|route| !route.trim().is_empty())
    {
        return Err("只有 navigate 允许 route".to_string());
    }
    Ok(())
}

fn validate_route(route: &str) -> Result<(), String> {
    let route = route.trim();
    let roots = [
        "/ai",
        "/workspace",
        "/projects",
        "/git-worktrees",
        "/ui-tuner",
        "/local-tasks",
        "/codex-control",
        "/node",
        "/doctor",
        "/account",
        "/user-browser",
    ];
    let safe_chars = route
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '-' | '_' | '.'));
    if route.is_empty()
        || route.len() > 180
        || !route.starts_with('/')
        || route.starts_with("//")
        || route.contains("..")
        || !safe_chars
        || !roots
            .iter()
            .any(|root| route == *root || route.starts_with(&format!("{root}/")))
    {
        return Err("route 不在 Tauri 相对路由白名单".to_string());
    }
    Ok(())
}

fn clean_identifier(value: &str, fallback: &str) -> String {
    let cleaned = value
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
        .take(160)
        .collect::<String>();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn clean_kind(value: &str) -> String {
    let cleaned = value
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
        .take(100)
        .collect::<String>();
    if cleaned.is_empty() {
        "event".to_string()
    } else {
        cleaned
    }
}

fn normalize_level(value: &str) -> &'static str {
    match value.trim() {
        "debug" => "debug",
        "warn" => "warn",
        "error" => "error",
        _ => "info",
    }
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        value.to_string()
    } else {
        value.chars().take(limit).collect::<String>() + "…"
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_are_relative_and_allowlisted() {
        assert!(validate_route("/codex-control").is_ok());
        assert!(validate_route("/projects/demo").is_ok());
        assert!(validate_route("https://example.com").is_err());
        assert!(validate_route("/unknown").is_err());
        assert!(validate_route("/projects/../account").is_err());
    }

    #[test]
    fn arbitrary_commands_never_enter_the_bridge() {
        let action = SemanticAction {
            action_id: "a".to_string(),
            trace_id: "t".to_string(),
            kind: "eval_javascript".to_string(),
            route: None,
        };
        assert!(validate_action(&action).is_err());
    }
}
