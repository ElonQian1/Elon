//! Local, allowlisted Win/Tauri semantic control and bounded diagnostic timeline.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{
    collections::{HashSet, VecDeque},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "node_agent_win_codex_control_api.rs"]
mod api;
pub(crate) use api::{routes, tauri_diagnostic_snapshot, timeline_payload};
#[path = "node_agent_win_codex_control/action_queue.rs"]
mod action_queue;
#[path = "node_agent_win_codex_control/ai_session_diagnostic.rs"]
mod ai_session_diagnostic;

const MAX_EVENTS: usize = 2_000;
const MAX_ACTIONS: usize = 256;
const ACTION_TTL_MS: u128 = 120_000;
const HOST_HEARTBEAT_TTL_MS: u128 = 20_000;
const MAX_SUMMARY_CHARS: usize = 600;
const MAX_FIELD_STRING_CHARS: usize = 800;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct WinControlEvent {
    pub seq: u64,
    pub event_id: String,
    pub trace_id: String,
    pub source: String,
    pub level: String,
    pub kind: String,
    pub summary: String,
    pub at_ms: u128,
    pub fields: Value,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct WinControlAction {
    pub action_id: String,
    pub trace_id: String,
    pub kind: String,
    pub route: Option<String>,
    pub provider_id: Option<String>,
    pub target_release_identity: Option<String>,
    pub requested_by: String,
    pub requested_at_ms: u128,
    pub expires_at_ms: u128,
    pub status: String,
    pub receipt: Option<WinControlReceipt>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct WinControlReceipt {
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub window_state: Option<Value>,
    #[serde(default)]
    pub at_ms: Option<u128>,
}

#[derive(Default)]
struct WinControlState {
    next_seq: u64,
    events: VecDeque<WinControlEvent>,
    actions: VecDeque<WinControlAction>,
    last_frontend_heartbeat_ms: Option<u128>,
    last_tauri_heartbeat_ms: Option<u128>,
}

#[derive(Default)]
pub(crate) struct WinCodexControlHub {
    inner: Mutex<WinControlState>,
}

impl WinCodexControlHub {
    pub(crate) fn record(
        &self,
        trace_id: &str,
        source: &str,
        level: &str,
        kind: &str,
        summary: &str,
        fields: Value,
    ) -> WinControlEvent {
        let now = now_ms();
        let source = normalize_source(source).to_string();
        let mut state = lock(&self.inner);
        state.next_seq = state.next_seq.saturating_add(1);
        if source == "frontend" {
            state.last_frontend_heartbeat_ms = Some(now);
        } else if source == "tauri" {
            state.last_tauri_heartbeat_ms = Some(now);
        }
        let event = WinControlEvent {
            seq: state.next_seq,
            event_id: format!("win_evt_{}", uuid::Uuid::new_v4().simple()),
            trace_id: clean_identifier(trace_id, "win"),
            source,
            level: normalize_level(level).to_string(),
            kind: clean_kind(kind),
            summary: redact_text(summary),
            at_ms: now,
            fields: sanitize_value(fields, 0),
        };
        state.events.push_back(event.clone());
        while state.events.len() > MAX_EVENTS {
            state.events.pop_front();
        }
        event
    }

    pub(crate) fn enqueue_action_with_target(
        &self,
        trace_id: &str,
        kind: &str,
        route: Option<&str>,
        provider_id: Option<&str>,
        target_release_identity: Option<&str>,
        requested_by: &str,
    ) -> Result<WinControlAction, String> {
        action_queue::enqueue(
            self,
            trace_id,
            kind,
            route,
            provider_id,
            target_release_identity,
            requested_by,
            &crate::node_agent_release_identity::current(),
        )
    }

    pub(crate) fn pending_actions(&self, limit: usize) -> Vec<WinControlAction> {
        let now = now_ms();
        let mut state = lock(&self.inner);
        expire_actions(&mut state, now);
        state
            .actions
            .iter()
            .filter(|action| action.status == "queued" && action.expires_at_ms >= now)
            .take(limit.clamp(1, 50))
            .cloned()
            .collect()
    }

    pub(crate) fn claim_action(&self, action_id: &str) -> Result<WinControlAction, String> {
        let now = now_ms();
        let mut state = lock(&self.inner);
        expire_actions(&mut state, now);
        let Some(action) = state
            .actions
            .iter_mut()
            .find(|action| action.action_id == action_id)
        else {
            return Err("Win 语义动作不存在或已超出保留窗口。".to_string());
        };
        if action.status == "executing" {
            return Ok(action.clone());
        }
        if action.status != "queued" {
            return Err(format!(
                "Win 语义动作当前状态为 {}，不能领取。",
                action.status
            ));
        }
        action.status = "executing".to_string();
        let claimed = action.clone();
        drop(state);
        self.record(
            &claimed.trace_id,
            "control",
            "info",
            "action.claimed",
            &format!("Tauri 桥已领取 Win 语义动作 {}", claimed.kind),
            json!({"action_id": claimed.action_id, "kind": claimed.kind}),
        );
        Ok(claimed)
    }

    pub(crate) fn action(&self, action_id: &str) -> Result<WinControlAction, String> {
        let now = now_ms();
        let mut state = lock(&self.inner);
        expire_actions(&mut state, now);
        state
            .actions
            .iter()
            .find(|action| action.action_id == action_id.trim())
            .cloned()
            .ok_or_else(|| "Win 语义动作不存在或已超出保留窗口。".to_string())
    }

    pub(crate) fn record_receipt(
        &self,
        action_id: &str,
        receipt: WinControlReceipt,
    ) -> Result<WinControlAction, String> {
        validate_receipt(&receipt)?;
        let now = now_ms();
        let mut state = lock(&self.inner);
        expire_actions(&mut state, now);
        let Some(action) = state
            .actions
            .iter_mut()
            .find(|action| action.action_id == action_id)
        else {
            return Err("Win 语义动作不存在或已超出保留窗口。".to_string());
        };
        if action.status == "expired" {
            return Err("Win 语义动作已经过期，拒绝迟到回执。".to_string());
        }
        if let Some(existing) = action.receipt.as_ref() {
            if existing.status == receipt.status {
                return Ok(action.clone());
            }
            return Err("Win 语义动作已有不同终态回执。".to_string());
        }
        if action.status != "executing" {
            return Err("Win 语义动作尚未由 Tauri 桥领取，拒绝伪造回执。".to_string());
        }
        let mut receipt = receipt;
        receipt.at_ms = Some(receipt.at_ms.unwrap_or(now));
        receipt.message = receipt.message.as_deref().map(redact_text);
        receipt.route = receipt
            .route
            .as_deref()
            .and_then(|route| validate_route(route).ok());
        receipt.window_state = receipt
            .window_state
            .take()
            .map(|value| sanitize_receipt_state(&action.kind, action.provider_id.as_deref(), value))
            .transpose()?;
        if receipt.status == "succeeded"
            && matches!(
                action.kind.as_str(),
                "capture_state" | "list_ai_windows" | "capture_ai_window_state" | "focus_ai_window"
            )
            && receipt.window_state.is_none()
        {
            return Err("成功的窗口状态动作必须携带脱敏状态回执。".to_string());
        }
        action.status = receipt.status.clone();
        action.receipt = Some(receipt);
        let completed = action.clone();
        drop(state);
        self.record(
            &completed.trace_id,
            "control",
            if completed.status == "succeeded" { "info" } else { "warn" },
            "action.receipt",
            &format!("Win 语义动作 {} 返回 {}", completed.kind, completed.status),
            json!({"action_id": completed.action_id, "kind": completed.kind, "status": completed.status}),
        );
        Ok(completed)
    }

    pub(crate) fn events(
        &self,
        since: u64,
        limit: usize,
        sources: &HashSet<String>,
    ) -> Vec<WinControlEvent> {
        lock(&self.inner)
            .events
            .iter()
            .filter(|event| event.seq > since)
            .filter(|event| sources.is_empty() || sources.contains(&event.source))
            .rev()
            .take(limit.clamp(1, 500))
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub(crate) fn capabilities(&self) -> Value {
        let now = now_ms();
        let state = lock(&self.inner);
        json!({
            "schema": "elon.win_codex_control.v1",
            "release_identity": crate::node_agent_release_identity::current(),
            "actions": allowed_actions(),
            "ai_window_providers": ["chatgpt", "google-ai-mode"],
            "routes": allowed_route_roots(),
            "sources": ["frontend", "rust", "cli", "network", "tauri", "control"],
            "security": {
                "arbitrary_javascript": false,
                "arbitrary_tauri_command": false,
                "arbitrary_url": false,
                "cookies_exported": false,
                "request_bodies_logged": false,
                "prompt_bodies_logged": false,
                "arbitrary_update_target": false,
                "update_restart_requires_exact_release": true,
            },
            "frontend_available": heartbeat_live(state.last_frontend_heartbeat_ms, now),
            "tauri_available": heartbeat_live(state.last_tauri_heartbeat_ms, now),
            "retention": {"events": MAX_EVENTS, "actions": MAX_ACTIONS, "action_ttl_ms": ACTION_TTL_MS},
        })
    }
}

fn sanitize_value(value: Value, depth: usize) -> Value {
    if depth >= 4 {
        return Value::String("[已截断]".to_string());
    }
    match value {
        Value::String(value) => Value::String(redact_text(&value)),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .take(32)
                .map(|value| sanitize_value(value, depth + 1))
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .take(48)
                .map(|(key, value)| {
                    let safe = if sensitive_key(&key) {
                        Value::String("[REDACTED]".to_string())
                    } else {
                        sanitize_value(value, depth + 1)
                    };
                    (truncate_chars(&key, 80), safe)
                })
                .collect::<Map<_, _>>(),
        ),
        other => other,
    }
}

fn redact_text(value: &str) -> String {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if [
        "authorization",
        "cookie",
        "password",
        "passwd",
        "api_key",
        "apikey",
        "access_token",
        "refresh_token",
        "bearer ",
        "client_secret",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return "[已脱敏：摘要包含敏感字段标记]".to_string();
    }
    truncate_chars(trimmed, MAX_SUMMARY_CHARS.min(MAX_FIELD_STRING_CHARS))
}

fn sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "cookie",
        "authorization",
        "api_key",
        "prompt",
        "body",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn validate_action_kind(kind: &str) -> Result<&'static str, String> {
    allowed_actions()
        .iter()
        .copied()
        .find(|allowed| *allowed == kind.trim())
        .ok_or_else(|| "不支持的 Win 语义动作。".to_string())
}

fn validate_action_route(kind: &str, route: Option<&str>) -> Result<Option<String>, String> {
    if kind == "navigate" {
        return route
            .ok_or_else(|| "navigate 动作必须提供相对路由。".to_string())
            .and_then(validate_route)
            .map(Some);
    }
    if route.is_some_and(|value| !value.trim().is_empty()) {
        return Err("只有 navigate 动作允许提供 route。".to_string());
    }
    Ok(None)
}

fn validate_action_provider(
    kind: &str,
    provider_id: Option<&str>,
) -> Result<Option<String>, String> {
    let requires_provider = matches!(kind, "capture_ai_window_state" | "focus_ai_window");
    let provider_id = provider_id.map(str::trim).filter(|value| !value.is_empty());
    if requires_provider {
        let provider_id =
            provider_id.ok_or_else(|| "AI 子窗口动作必须提供 provider_id。".to_string())?;
        return matches!(provider_id, "chatgpt" | "google-ai-mode")
            .then(|| Some(provider_id.to_string()))
            .ok_or_else(|| "provider_id 不在 AI 子窗口白名单。".to_string());
    }
    if provider_id.is_some() {
        return Err("只有 AI 子窗口定向动作允许 provider_id。".to_string());
    }
    Ok(None)
}

fn validate_action_target(
    kind: &str,
    target_release_identity: Option<&str>,
    requested_by: &str,
) -> Result<Option<String>, String> {
    let target = target_release_identity
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if kind == "update_and_restart" {
        if requested_by.trim() != "codex_mcp" {
            return Err("更新重启动作只允许项目绑定的 Codex MCP 发起。".to_string());
        }
        return target
            .ok_or_else(|| "update_and_restart 必须提供精确 target_release_identity。".to_string())
            .and_then(validate_release_identity)
            .map(Some);
    }
    if target.is_some() {
        return Err("只有 update_and_restart 允许 target_release_identity。".to_string());
    }
    Ok(None)
}

fn validate_release_identity(value: &str) -> Result<String, String> {
    let (version, git_sha) = value
        .rsplit_once('+')
        .ok_or_else(|| "target_release_identity 必须是 version+git_sha。".to_string())?;
    let version_ok = !version.is_empty()
        && version.len() <= 48
        && version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'));
    let git_sha_ok =
        (40..=64).contains(&git_sha.len()) && git_sha.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !version_ok || !git_sha_ok {
        return Err("target_release_identity 不是合法的精确 Win 发布身份。".to_string());
    }
    Ok(format!("{}+{}", version, git_sha.to_ascii_lowercase()))
}

fn validate_route(route: &str) -> Result<String, String> {
    let route = route.trim();
    if route.is_empty()
        || route.len() > 180
        || !route.starts_with('/')
        || route.starts_with("//")
        || route.contains('?')
        || route.contains('#')
        || route.contains(':')
        || route.contains("..")
        || route
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '/' | '-' | '_' | '.')))
    {
        return Err("Win 路由必须是无 query/hash 的安全相对路径。".to_string());
    }
    if !allowed_route_roots()
        .iter()
        .any(|root| route == *root || route.starts_with(&format!("{root}/")))
    {
        return Err("Win 路由不在语义控制白名单。".to_string());
    }
    Ok(route.to_string())
}

fn validate_receipt(receipt: &WinControlReceipt) -> Result<(), String> {
    if !matches!(
        receipt.status.trim(),
        "succeeded" | "failed" | "host_unavailable" | "rejected"
    ) {
        return Err("动作回执 status 无效。".to_string());
    }
    Ok(())
}

fn sanitize_receipt_state(
    action_kind: &str,
    provider_id: Option<&str>,
    value: Value,
) -> Result<Value, String> {
    if serde_json::to_vec(&value)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX)
        > 16 * 1024
    {
        return Err("窗口状态回执超过 16 KiB。".to_string());
    }
    let schema = value.get("schema").and_then(Value::as_str);
    match (action_kind, schema) {
        ("list_ai_windows", Some("elon.tauri_ai_window_list.v1")) => {
            let sanitized = value
                .get("windows")
                .and_then(Value::as_array)
                .ok_or_else(|| "AI 窗口列表回执缺少 windows。".to_string())?
                .iter()
                .map(sanitize_ai_window)
                .collect::<Result<Vec<_>, _>>()?;
            let windows = ["chatgpt", "google-ai-mode"]
                .iter()
                .map(|expected| {
                    let matches = sanitized
                        .iter()
                        .filter(|window| {
                            window.get("provider_id").and_then(Value::as_str) == Some(*expected)
                        })
                        .collect::<Vec<_>>();
                    (matches.len() == 1)
                        .then(|| matches[0].clone())
                        .ok_or_else(|| "AI 窗口列表必须精确包含两个固定 provider。".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"schema": schema, "windows": windows, "privacy": receipt_privacy()}))
        }
        (
            "capture_ai_window_state" | "focus_ai_window",
            Some("elon.tauri_ai_window_capture.v1"),
        ) => {
            let window = sanitize_ai_window(
                value
                    .get("window")
                    .ok_or_else(|| "AI 窗口状态回执缺少 window。".to_string())?,
            )?;
            let expected =
                provider_id.ok_or_else(|| "AI 窗口动作缺少 provider_id。".to_string())?;
            if window.get("provider_id").and_then(Value::as_str) != Some(expected) {
                return Err("AI 窗口状态回执与请求 provider_id 不一致。".to_string());
            }
            Ok(json!({"schema": schema, "window": window, "privacy": receipt_privacy()}))
        }
        ("capture_state", None) => Ok(json!({
            "role": "main",
            "visible": optional_bool(&value, "visible"),
            "focused": optional_bool(&value, "focused"),
            "maximized": optional_bool(&value, "maximized"),
            "minimized": optional_bool(&value, "minimized"),
            "devtools_open": optional_bool(&value, "devtools_open"),
            "window_roles": value.get("window_roles").and_then(Value::as_array).into_iter().flatten()
                .filter_map(Value::as_str).filter(|role| matches!(*role, "main" | "ai:chatgpt" | "ai:google-ai-mode" | "official:chatgpt" | "official:google-ai-mode" | "other"))
                .take(8).collect::<Vec<_>>(),
        })),
        _ => Err("窗口状态回执与动作类型或 schema 不匹配。".to_string()),
    }
}

fn sanitize_ai_window(value: &Value) -> Result<Value, String> {
    let provider_id = value
        .get("provider_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !matches!(provider_id, "chatgpt" | "google-ai-mode") {
        return Err("AI 窗口状态包含无效 provider_id。".to_string());
    }
    let phase = value
        .get("phase")
        .and_then(Value::as_str)
        .unwrap_or("error");
    if !matches!(
        phase,
        "not_created" | "creating" | "loading" | "loaded" | "ready" | "error" | "closed"
    ) {
        return Err("AI 窗口状态包含无效 phase。".to_string());
    }
    let error_code = value
        .get("last_error_code")
        .and_then(Value::as_str)
        .filter(|code| {
            matches!(
                *code,
                "root_empty"
                    | "page_runtime_error"
                    | "webview_navigation_error"
                    | "webview_create_failed"
            )
        });
    let official_session = ai_session_diagnostic::sanitize(value.get("official_session"))?;
    Ok(json!({
        "provider_id": provider_id,
        "phase": phase,
        "open": value.get("open").and_then(Value::as_bool).unwrap_or(false),
        "focused": value.get("focused").and_then(Value::as_bool).unwrap_or(false),
        "page_ready": value.get("page_ready").and_then(Value::as_bool).unwrap_or(false),
        "root_exists": value.get("root_exists").and_then(Value::as_bool).unwrap_or(false),
        "root_child_count": value.get("root_child_count").and_then(Value::as_u64).unwrap_or(0).min(10_000),
        "last_error_code": error_code,
        "retryable": value.get("retryable").and_then(Value::as_bool).unwrap_or(false),
        "updated_at_ms": value.get("updated_at_ms").and_then(Value::as_u64).unwrap_or(0),
        "official_session": official_session,
    }))
}

fn optional_bool(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn receipt_privacy() -> Value {
    json!({
        "window_labels": false,
        "owner_fingerprints": false,
        "urls": false,
        "page_text": false,
        "cookies": false,
        "tokens": false,
    })
}

fn expire_actions(state: &mut WinControlState, now: u128) {
    for action in &mut state.actions {
        if matches!(action.status.as_str(), "queued" | "executing") && action.expires_at_ms < now {
            action.status = "expired".to_string();
        }
    }
}

fn allowed_actions() -> &'static [&'static str] {
    &[
        "show_window",
        "focus_window",
        "navigate",
        "reload_page",
        "open_devtools",
        "close_devtools",
        "capture_state",
        "list_ai_windows",
        "capture_ai_window_state",
        "focus_ai_window",
        "update_and_restart",
    ]
}

fn allowed_route_roots() -> &'static [&'static str] {
    &[
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
    ]
}

fn is_client_source(source: &str) -> bool {
    matches!(source.trim(), "frontend" | "network" | "tauri")
}

fn normalize_source(source: &str) -> &'static str {
    match source.trim() {
        "frontend" => "frontend",
        "network" => "network",
        "tauri" => "tauri",
        "cli" => "cli",
        "control" => "control",
        _ => "rust",
    }
}

fn normalize_level(level: &str) -> &'static str {
    match level.trim().to_ascii_lowercase().as_str() {
        "debug" => "debug",
        "warn" | "warning" => "warn",
        "error" | "fatal" => "error",
        _ => "info",
    }
}

fn clean_identifier(value: &str, fallback: &str) -> String {
    let value = value.trim();
    let cleaned = value
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

fn truncate_chars(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    value.chars().take(limit).collect::<String>() + "…"
}

fn heartbeat_live(value: Option<u128>, now: u128) -> bool {
    value.is_some_and(|value| now.saturating_sub(value) <= HOST_HEARTBEAT_TTL_MS)
}

fn u128_to_u64(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
#[path = "node_agent_win_codex_control_tests.rs"]
mod tests;
