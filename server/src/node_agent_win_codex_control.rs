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
pub(crate) use api::{routes, timeline_payload};

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

    pub(crate) fn enqueue_action(
        &self,
        trace_id: &str,
        kind: &str,
        route: Option<&str>,
        requested_by: &str,
    ) -> Result<WinControlAction, String> {
        let kind = validate_action_kind(kind)?;
        let route = validate_action_route(kind, route)?;
        let now = now_ms();
        let action = WinControlAction {
            action_id: format!("win_act_{}", uuid::Uuid::new_v4().simple()),
            trace_id: clean_identifier(trace_id, "win_action"),
            kind: kind.to_string(),
            route,
            requested_by: clean_identifier(requested_by, "local_admin"),
            requested_at_ms: now,
            expires_at_ms: now.saturating_add(ACTION_TTL_MS),
            status: "queued".to_string(),
            receipt: None,
        };
        let mut state = lock(&self.inner);
        expire_actions(&mut state, now);
        state.actions.push_back(action.clone());
        while state.actions.len() > MAX_ACTIONS {
            state.actions.pop_front();
        }
        drop(state);
        self.record(
            &action.trace_id,
            "control",
            "info",
            "action.queued",
            &format!("已排队 Win 语义动作 {}", action.kind),
            json!({"action_id": action.action_id, "kind": action.kind, "route": action.route}),
        );
        Ok(action)
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
            "actions": allowed_actions(),
            "routes": allowed_route_roots(),
            "sources": ["frontend", "rust", "cli", "network", "tauri", "control"],
            "security": {
                "arbitrary_javascript": false,
                "arbitrary_tauri_command": false,
                "arbitrary_url": false,
                "cookies_exported": false,
                "request_bodies_logged": false,
                "prompt_bodies_logged": false,
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
