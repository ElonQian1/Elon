//! Allowlisted semantic actions for Codex-driven Win client debugging.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State, WebviewWindow};

#[path = "codex_semantic_bridge/ai_window_control.rs"]
mod ai_window_control;

use crate::local_ai_browser::LocalAiNativeWindowRuntime;

const MAX_NATIVE_EVENTS: usize = 600;
const MAX_PERSISTED_EVENTS: usize = 64;
const MAX_PERSISTED_HEARTBEATS: usize = 4;
static PERSIST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SemanticAction {
    pub action_id: String,
    #[serde(default)]
    pub trace_id: String,
    pub kind: String,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub provider_id: Option<String>,
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
        // Every native bridge event must reach the durable redacted snapshot. Keeping
        // persistence inside record() prevents command handlers from silently creating
        // an in-memory-only timeline after a desktop restart.
        drop(state);
        self.persist();
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

    fn persist(&self) {
        let events = lock(&self.state).events.clone();
        std::thread::spawn(move || {
            let _guard = lock(&PERSIST_LOCK);
            persist_events(&events);
        });
    }
}

fn persist_events(events: &VecDeque<NativeEvent>) {
    let Some(path) = diagnostic_snapshot_path() else {
        return;
    };
    let latest_seq = events.back().map(|event| event.seq).unwrap_or_default();
    if existing_snapshot_seq(&path, std::process::id()) >= latest_seq {
        return;
    }
    let payload = json!({
        "schema": "elon.tauri_native_diagnostics.v1",
        "generated_at_ms": now_ms(),
        "latest_seq": latest_seq,
        "desktop_pid": std::process::id(),
        "privacy": {
            "cookies": false,
            "tokens": false,
            "request_bodies": false,
            "prompt_bodies": false,
            "page_text": false
        },
        "events": diagnostic_snapshot_events(events),
    });
    let Ok(bytes) = serde_json::to_vec(&payload) else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    if fs::write(&temporary, bytes).is_ok() {
        let _ = fs::remove_file(&path);
        let _ = fs::rename(temporary, path);
    }
}

fn diagnostic_snapshot_events(events: &VecDeque<NativeEvent>) -> Vec<NativeEvent> {
    let mut heartbeat_count = 0usize;
    events
        .iter()
        .rev()
        .filter(|event| {
            if event.kind != "bridge.heartbeat" {
                return true;
            }
            heartbeat_count = heartbeat_count.saturating_add(1);
            heartbeat_count <= MAX_PERSISTED_HEARTBEATS
        })
        .take(MAX_PERSISTED_EVENTS)
        .cloned()
        .collect()
}

fn existing_snapshot_seq(path: &PathBuf, desktop_pid: u32) -> u64 {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(|value| {
            value.get("desktop_pid").and_then(Value::as_u64) == Some(u64::from(desktop_pid))
        })
        .and_then(|value| value.get("latest_seq").and_then(Value::as_u64))
        .unwrap_or_default()
}

fn diagnostic_snapshot_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|root| {
        PathBuf::from(root)
            .join("Elon")
            .join("desktop-diagnostics-v1")
            .join("native-events.json")
    })
}

pub(crate) fn record_app_event(
    app: &AppHandle,
    trace_id: &str,
    level: &str,
    kind: &str,
    summary: &str,
    fields: Value,
) {
    let bridge = app.state::<CodexSemanticBridge>();
    bridge.record(trace_id, level, kind, summary, fields);
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
        "actions":["show_window","focus_window","navigate","reload_page","open_devtools","close_devtools","capture_state","list_ai_windows","capture_ai_window_state","focus_ai_window"],
        "ai_window_providers":["chatgpt","google-ai-mode"],
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
    ai_windows: State<'_, LocalAiNativeWindowRuntime>,
    action: SemanticAction,
) -> Result<Value, String> {
    validate_action(&action)?;
    let result = execute(&window, ai_windows.inner(), &action);
    let captured_state = result.as_ref().ok().and_then(|result| result.state.clone());
    let (status, level) = match &result {
        Ok(_) => ("succeeded", "info"),
        Err(_) => ("failed", "error"),
    };
    bridge.record(
        &action.trace_id,
        level,
        "action.executed",
        &format!("{}: {}", action.kind, status),
        json!({"action_id": action.action_id, "kind": action.kind, "status": status, "route": action.route, "window_state": captured_state.clone()}),
    );
    result.map(|result| {
        json!({
            "schema":"elon.tauri_codex_action_receipt.v1",
            "action_id":action.action_id,
            "trace_id":clean_identifier(&action.trace_id, "tauri"),
            "status":"succeeded",
            "message":result.message,
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

struct SemanticActionResult {
    message: String,
    state: Option<Value>,
}

fn outcome(
    message: impl Into<String>,
    state: Option<Value>,
) -> Result<SemanticActionResult, String> {
    Ok(SemanticActionResult {
        message: message.into(),
        state,
    })
}

fn execute(
    window: &WebviewWindow,
    ai_windows: &LocalAiNativeWindowRuntime,
    action: &SemanticAction,
) -> Result<SemanticActionResult, String> {
    match action.kind.as_str() {
        "show_window" => {
            window.show().map_err(display_error)?;
            outcome("主窗口已显示", None)
        }
        "focus_window" => {
            window.show().map_err(display_error)?;
            window.set_focus().map_err(display_error)?;
            outcome("主窗口已显示并聚焦", None)
        }
        "navigate" => {
            let route = action.route.as_deref().ok_or("navigate 缺少 route")?;
            let browser_route = pc_browser_route(route)?;
            let encoded = serde_json::to_string(&browser_route).map_err(display_error)?;
            window
                .eval(&format!(
                    "window.history.pushState({{}}, '', {encoded}); window.dispatchEvent(new PopStateEvent('popstate'));"
                ))
                .map_err(display_error)?;
            outcome(format!("已导航到 {route}"), None)
        }
        "reload_page" => {
            window
                .eval("window.setTimeout(function () { window.location.reload(); }, 500);")
                .map_err(display_error)?;
            outcome("页面刷新将在回执写回后触发", None)
        }
        "open_devtools" => devtools(window, true).and_then(|message| outcome(message, None)),
        "close_devtools" => devtools(window, false).and_then(|message| outcome(message, None)),
        "capture_state" => outcome("已捕获非秘密窗口状态", Some(window_state(window))),
        "list_ai_windows" => outcome(
            "已列出一龙 AI 逻辑子窗口",
            Some(ai_window_control::list(window.app_handle(), ai_windows)),
        ),
        "capture_ai_window_state" => {
            let provider_id =
                ai_window_control::validate_provider_id(action.provider_id.as_deref())?;
            outcome(
                "已捕获一龙 AI 子窗口脱敏状态",
                Some(ai_window_control::capture(
                    window.app_handle(),
                    ai_windows,
                    provider_id,
                )),
            )
        }
        "focus_ai_window" => {
            let provider_id =
                ai_window_control::validate_provider_id(action.provider_id.as_deref())?;
            let state = ai_window_control::focus(window.app_handle(), ai_windows, provider_id)?;
            outcome("一龙 AI 子窗口已聚焦", Some(state))
        }
        _ => Err("不支持的 Tauri 语义动作".to_string()),
    }
}

fn window_state(window: &WebviewWindow) -> Value {
    let mut window_roles = window
        .app_handle()
        .webview_windows()
        .keys()
        .map(|label| semantic_window_role(label))
        .collect::<Vec<_>>();
    window_roles.sort_unstable();
    window_roles.dedup();
    json!({
        "label": window.label(),
        "visible": window.is_visible().ok(),
        "focused": window.is_focused().ok(),
        "maximized": window.is_maximized().ok(),
        "minimized": window.is_minimized().ok(),
        "devtools_open": devtools_open(window),
        "window_roles": window_roles,
    })
}

fn semantic_window_role(label: &str) -> &'static str {
    if label == "main" {
        "main"
    } else if label.starts_with("local-ai-native-chatgpt-") {
        "ai:chatgpt"
    } else if label.starts_with("local-ai-native-google-ai-mode-") {
        "ai:google-ai-mode"
    } else if label.starts_with("local-ai-web-chatgpt-") {
        "official:chatgpt"
    } else if label.starts_with("local-ai-web-google-ai-mode-") {
        "official:google-ai-mode"
    } else {
        "other"
    }
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
            | "list_ai_windows"
            | "capture_ai_window_state"
            | "focus_ai_window"
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
    let provider_action = matches!(
        action.kind.as_str(),
        "capture_ai_window_state" | "focus_ai_window"
    );
    if provider_action {
        ai_window_control::validate_provider_id(action.provider_id.as_deref())?;
    } else if action
        .provider_id
        .as_deref()
        .is_some_and(|provider_id| !provider_id.trim().is_empty())
    {
        return Err("只有 AI 子窗口定向动作允许 provider_id".to_string());
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

fn pc_browser_route(route: &str) -> Result<String, String> {
    validate_route(route)?;
    Ok(format!("/pc{}", route.trim()))
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
    fn semantic_navigation_preserves_the_pc_browser_basename() {
        assert_eq!(
            pc_browser_route("/user-browser").unwrap(),
            "/pc/user-browser"
        );
        assert_eq!(
            pc_browser_route("/projects/demo").unwrap(),
            "/pc/projects/demo"
        );
        assert!(pc_browser_route("/pc/user-browser").is_err());
    }

    #[test]
    fn arbitrary_commands_never_enter_the_bridge() {
        let action = SemanticAction {
            action_id: "a".to_string(),
            trace_id: "t".to_string(),
            kind: "eval_javascript".to_string(),
            route: None,
            provider_id: None,
        };
        assert!(validate_action(&action).is_err());
    }

    #[test]
    fn existing_snapshot_sequence_is_scoped_to_the_current_desktop_process() {
        let path = std::env::temp_dir().join(format!(
            "elon_native_diagnostics_{}.json",
            std::process::id()
        ));
        std::fs::write(
            &path,
            serde_json::to_vec(&json!({"desktop_pid": 41, "latest_seq": 99})).unwrap(),
        )
        .unwrap();
        assert_eq!(existing_snapshot_seq(&path, 41), 99);
        assert_eq!(existing_snapshot_seq(&path, 42), 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn diagnostic_snapshot_keeps_window_events_when_heartbeats_are_noisy() {
        let event = |seq: u64, kind: &str| NativeEvent {
            seq,
            event_id: format!("event-{seq}"),
            trace_id: "trace".to_string(),
            level: "debug".to_string(),
            kind: kind.to_string(),
            summary: kind.to_string(),
            at_ms: u128::from(seq),
            fields: json!({}),
        };
        let mut events = VecDeque::new();
        events.push_back(event(1, "native_window.created"));
        for seq in 2..=100 {
            events.push_back(event(seq, "bridge.heartbeat"));
        }

        let snapshot = diagnostic_snapshot_events(&events);
        assert_eq!(
            snapshot
                .iter()
                .filter(|item| item.kind == "bridge.heartbeat")
                .count(),
            MAX_PERSISTED_HEARTBEATS
        );
        assert!(snapshot
            .iter()
            .any(|item| item.kind == "native_window.created"));
    }
}
