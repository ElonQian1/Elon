use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::Value;
use tauri::Url;

use super::{conversation_directory, semantic_context, snapshot_cache};

#[path = "state/cache.rs"]
mod cache;
#[path = "state/context.rs"]
mod context;

#[derive(Clone, Default)]
pub struct LocalAiBrowserRuntime {
    sessions: Arc<Mutex<HashMap<String, SessionRecord>>>,
}

#[derive(Clone)]
struct SessionRecord {
    provider_id: String,
    window_label: String,
    window_status: String,
    window_visible: bool,
    current_url: String,
    current_host: String,
    loading: bool,
    renderer_status: String,
    last_error: Option<String>,
    last_error_code: Option<String>,
    semantic_event: Option<Value>,
    navigation_event: Option<Value>,
    composer_event: Option<Value>,
    feature_event: Option<Value>,
    ui_manifest_event: Option<Value>,
    command_result: Option<Value>,
    last_event_kind: String,
    last_command_action: String,
    last_command_request_id: Option<String>,
    last_command_ok: Option<bool>,
    message_count: usize,
    assistant_message_count: usize,
    streaming: bool,
    semantic_live: bool,
    navigation_live: bool,
    conversation_snapshots: Vec<snapshot_cache::StoredConversationSnapshot>,
    active_restorable_url: Option<String>,
    active_conversation_id: Option<String>,
    semantic_conversation_id: Option<String>,
    active_page_context_key: Option<String>,
    semantic_page_context_key: Option<String>,
    pending_context_action: String,
    pending_context_since_ms: u64,
    pending_send_prompt: Option<String>,
    preserve_conversation_on_navigation: bool,
    cache_path: Option<PathBuf>,
    cache_updated_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiWebSessionState {
    pub provider_id: String,
    pub window_label: String,
    pub window_status: String,
    pub window_visible: bool,
    pub current_url: String,
    pub current_host: String,
    pub loading: bool,
    pub renderer_status: String,
    pub last_error: Option<String>,
    pub last_error_code: Option<String>,
    pub semantic_event: Option<Value>,
    pub navigation_event: Option<Value>,
    pub composer_event: Option<Value>,
    pub feature_event: Option<Value>,
    pub ui_manifest_event: Option<Value>,
    pub command_result: Option<Value>,
    pub diagnostics: Value,
    pub cache_status: String,
    pub semantic_cache_status: String,
    pub navigation_cache_status: String,
    pub local_conversations: Vec<LocalAiCachedConversation>,
    pub active_conversation_id: Option<String>,
    pub context_ready: bool,
    pub context_status: String,
    pub cache_updated_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiCachedConversation {
    pub id: String,
    pub title: String,
    pub active: bool,
    pub updated_at_ms: u64,
}

impl LocalAiBrowserRuntime {
    pub fn cached_restorable_url(&self, label: &str) -> Option<String> {
        self.sessions().get(label).and_then(|record| {
            record.active_restorable_url.clone().or_else(|| {
                record
                    .semantic_event
                    .as_ref()
                    .and_then(|event| event.get("url"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            })
        })
    }

    pub fn activate_cached_conversation(&self, label: &str, id: &str) -> Option<String> {
        let mut sessions = self.sessions();
        let record = sessions.get_mut(label)?;
        let cached = record
            .conversation_snapshots
            .iter()
            .find(|entry| entry.id == id)
            .cloned()?;
        record.semantic_event = Some(cached.semantic_event);
        record.active_restorable_url = Some(cached.restorable_url.clone());
        record.semantic_conversation_id = Some(cached.id.clone());
        record.semantic_page_context_key =
            semantic_context::page_context_key(&record.provider_id, &cached.restorable_url);
        record.begin_cached_conversation(cached.id, &cached.restorable_url);
        record.semantic_live = false;
        record.streaming = false;
        record.last_event_kind = "cached_conversation_activated".to_string();
        record.updated_at_ms = now_ms();
        Some(cached.restorable_url)
    }

    pub fn ensure_session(&self, label: &str, provider_id: &str, renderer_status: &str) {
        self.ensure_session_record(label, provider_id, renderer_status, None);
    }

    pub fn ensure_session_with_cache(
        &self,
        label: &str,
        provider_id: &str,
        renderer_status: &str,
        cache_path: PathBuf,
    ) {
        self.ensure_session_record(label, provider_id, renderer_status, Some(cache_path));
    }

    fn ensure_session_record(
        &self,
        label: &str,
        provider_id: &str,
        renderer_status: &str,
        cache_path: Option<PathBuf>,
    ) {
        if self.sessions().contains_key(label) {
            return;
        }
        let cached = cache_path
            .as_deref()
            .and_then(|path| snapshot_cache::load(path, provider_id).ok().flatten());
        let mut sessions = self.sessions();
        sessions.entry(label.to_string()).or_insert_with(|| {
            let (semantic_event, navigation_event, conversation_snapshots, cache_updated_at_ms) =
                cached
                    .map(|snapshot| {
                        (
                            snapshot.semantic_event,
                            snapshot.navigation_event,
                            snapshot.conversation_snapshots,
                            snapshot.updated_at_ms,
                        )
                    })
                    .unwrap_or_default();
            let active_restorable_url = conversation_snapshots
                .first()
                .map(|entry| entry.restorable_url.clone());
            let active_conversation_id =
                conversation_snapshots.first().map(|entry| entry.id.clone());
            let active_page_context_key = active_restorable_url
                .as_deref()
                .and_then(|url| semantic_context::page_context_key(provider_id, url));
            SessionRecord {
                provider_id: provider_id.to_string(),
                window_label: label.to_string(),
                window_status: "opening".to_string(),
                window_visible: false,
                current_url: String::new(),
                current_host: String::new(),
                loading: true,
                renderer_status: renderer_status.to_string(),
                last_error: None,
                last_error_code: None,
                semantic_event,
                navigation_event,
                composer_event: None,
                feature_event: None,
                ui_manifest_event: None,
                command_result: None,
                last_event_kind: "session_created".to_string(),
                last_command_action: String::new(),
                last_command_request_id: None,
                last_command_ok: None,
                message_count: 0,
                assistant_message_count: 0,
                streaming: false,
                semantic_live: false,
                navigation_live: false,
                conversation_snapshots,
                active_restorable_url,
                active_conversation_id: active_conversation_id.clone(),
                semantic_conversation_id: active_conversation_id,
                active_page_context_key: active_page_context_key.clone(),
                semantic_page_context_key: active_page_context_key,
                pending_context_action: String::new(),
                pending_context_since_ms: 0,
                pending_send_prompt: None,
                preserve_conversation_on_navigation: false,
                cache_path,
                cache_updated_at_ms,
                updated_at_ms: now_ms(),
            }
        });
    }

    pub fn mark_opening(&self, label: &str, window_visible: bool) {
        self.update(label, |record| {
            record.window_status = "opening".to_string();
            record.window_visible = window_visible;
            record.loading = true;
            record.renderer_status = "connecting".to_string();
            record.last_error = None;
            record.last_error_code = None;
            record.command_result = None;
            record.mark_snapshot_cached();
        });
    }

    /// 当前停放在后台（未关闭、也没有对用户显示）的会话标签，供主窗口尺寸变化后
    /// 重新计算停放坐标，避免旧坐标落回放大/还原后的可见区域内。
    pub fn parked_session_labels(&self) -> Vec<String> {
        self.sessions()
            .iter()
            .filter(|(_, record)| !record.window_visible && record.window_status != "closed")
            .map(|(label, _)| label.clone())
            .collect()
    }

    pub fn mark_navigation(
        &self,
        label: &str,
        url: &Url,
        allowed: bool,
        blocked_message: Option<&str>,
    ) {
        let safe_url = safe_visible_url(url);
        let raw_url = url.as_str().to_string();
        let host = url.host_str().unwrap_or_default().to_string();
        self.update(label, |record| {
            record.current_url = safe_url;
            record.current_host = host;
            if allowed {
                record.active_restorable_url =
                    snapshot_cache::normalize_restorable_url(&record.provider_id, &raw_url);
                record.mark_context_navigation(&raw_url);
                record.window_status = "loading".to_string();
                record.loading = true;
                record.renderer_status = "connecting".to_string();
                record.last_error = None;
                record.last_error_code = None;
                record.command_result = None;
                record.mark_snapshot_cached();
            } else {
                record.window_status = "blocked".to_string();
                record.loading = false;
                record.last_error = Some(
                    blocked_message
                        .unwrap_or("页面尝试离开允许的本地 AI 网页域名，已由一龙拦截。")
                        .to_string(),
                );
                record.last_error_code = Some("navigation_blocked".to_string());
            }
        });
    }

    pub fn mark_page_finished(&self, label: &str, url: &Url) {
        let safe_url = safe_visible_url(url);
        let host = url.host_str().unwrap_or_default().to_string();
        self.update(label, |record| {
            record.current_url = safe_url;
            record.current_host = host;
            record.window_status = "ready".to_string();
            record.loading = false;
        });
    }

    pub fn mark_window_status(&self, label: &str, status: &str) {
        self.update(label, |record| {
            record.window_status = status.to_string();
            if matches!(status, "closed" | "error") {
                record.loading = false;
                record.window_visible = false;
                record.mark_snapshot_cached();
            }
        });
    }

    pub fn mark_window_visible(&self, label: &str, visible: bool) {
        self.update(label, |record| {
            record.window_visible = visible;
        });
    }

    pub fn record_error(&self, label: &str, detail: impl Into<String>) {
        let detail = truncate(detail.into(), 240);
        self.update(label, |record| {
            record.window_status = "error".to_string();
            record.loading = false;
            record.last_error = Some(detail);
            record.last_error_code = Some("host_error".to_string());
        });
    }

    pub fn mark_command_pending(&self, label: &str, action: &str, request_id: Option<&str>) {
        self.mark_command_pending_with_value(label, action, request_id, None);
    }

    pub fn mark_command_pending_with_value(
        &self,
        label: &str,
        action: &str,
        request_id: Option<&str>,
        value: Option<&str>,
    ) {
        self.update(label, |record| {
            record.command_result = None;
            record.last_event_kind = "command_pending".to_string();
            record.last_command_action = truncate(action.to_string(), 48);
            record.last_command_request_id =
                request_id.map(|value| truncate(value.to_string(), 36));
            record.last_command_ok = None;
            record.begin_context_command(action, value);
        });
    }

    pub fn record_adapter_event(&self, label: &str, kind: &str, payload: Value) {
        self.record_adapter_event_with_context(label, kind, payload, None);
    }

    pub fn record_adapter_event_with_context(
        &self,
        label: &str,
        kind: &str,
        payload: Value,
        page_context_key: Option<&str>,
    ) {
        self.record_adapter_event_with_context_and_url(
            label,
            kind,
            payload,
            page_context_key,
            None,
        );
    }

    pub fn record_adapter_event_with_context_and_url(
        &self,
        label: &str,
        kind: &str,
        payload: Value,
        page_context_key: Option<&str>,
        restorable_url: Option<&str>,
    ) {
        self.update(label, |record| {
            record.last_event_kind = truncate(kind.to_string(), 48);
            match kind {
                "adapter_ready" => {
                    record.renderer_status = "active".to_string();
                    record.last_error = None;
                    record.last_error_code = None;
                }
                "message_snapshot" => {
                    record.renderer_status = "active".to_string();
                    if !record.apply_message_snapshot(payload, page_context_key) {
                        return;
                    }
                    if let Some(url) = restorable_url.and_then(|url| {
                        snapshot_cache::normalize_restorable_url(&record.provider_id, url)
                    }) {
                        record.active_restorable_url = Some(url);
                    }
                    let snapshot = record.semantic_event.as_ref();
                    record.message_count = snapshot
                        .and_then(|event| event.get("messages"))
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len);
                    record.assistant_message_count = snapshot
                        .and_then(|event| event.get("messages"))
                        .and_then(Value::as_array)
                        .map_or(0, |messages| {
                            messages
                                .iter()
                                .filter(|message| {
                                    message.get("role").and_then(Value::as_str) == Some("assistant")
                                })
                                .count()
                        });
                    record.streaming = snapshot
                        .and_then(|event| event.get("streaming"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    record.semantic_live = true;
                    record.cache_updated_at_ms = now_ms();
                    record.last_error = None;
                    record.last_error_code = None;
                    record.remember_current_conversation();
                }
                "conversation_snapshot" => {
                    record.renderer_status = "active".to_string();
                    record.navigation_event = Some(conversation_directory::merge(
                        record.navigation_event.as_ref(),
                        payload,
                    ));
                    record.navigation_live = true;
                    record.cache_updated_at_ms = now_ms();
                    record.last_error = None;
                    record.last_error_code = None;
                }
                "composer_controls_snapshot" => record.composer_event = Some(payload),
                "navigation_snapshot" => record.feature_event = Some(payload),
                "ui_manifest_snapshot" => record.ui_manifest_event = Some(payload),
                "command_result" => {
                    record.finish_context_command(&payload);
                    record.last_command_action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .map_or_else(String::new, |value| truncate(value.to_string(), 48));
                    record.last_command_request_id = payload
                        .get("requestId")
                        .and_then(Value::as_str)
                        .map(|value| truncate(value.to_string(), 36));
                    record.last_command_ok = payload.get("ok").and_then(Value::as_bool);
                    record.command_result = Some(payload);
                }
                "browser_diagnostic" => {
                    let detail = payload
                        .get("detail")
                        .and_then(Value::as_str)
                        .unwrap_or("ChatGPT 页面暂未完成加载。")
                        .to_string();
                    record.last_error = Some(truncate(detail, 240));
                    record.last_error_code = payload
                        .get("kind")
                        .and_then(Value::as_str)
                        .map(|kind| truncate(kind.to_string(), 48));
                }
                _ => {}
            }
        });
        if matches!(kind, "message_snapshot" | "conversation_snapshot") {
            self.persist_snapshot(label);
        }
    }

    pub fn clear_snapshots(&self, label: &str) {
        let cache_path = {
            let mut sessions = self.sessions();
            let Some(record) = sessions.get_mut(label) else {
                return;
            };
            record.semantic_event = None;
            record.navigation_event = None;
            record.composer_event = None;
            record.feature_event = None;
            record.ui_manifest_event = None;
            record.command_result = None;
            record.last_event_kind = "session_cleared".to_string();
            record.last_command_action.clear();
            record.last_command_request_id = None;
            record.last_command_ok = None;
            record.message_count = 0;
            record.assistant_message_count = 0;
            record.streaming = false;
            record.semantic_live = false;
            record.navigation_live = false;
            record.conversation_snapshots.clear();
            record.active_restorable_url = None;
            record.reset_context();
            record.cache_updated_at_ms = 0;
            record.updated_at_ms = now_ms();
            record.cache_path.clone()
        };
        if let Some(path) = cache_path {
            snapshot_cache::clear(&path);
        }
    }

    pub fn snapshot(&self, label: &str) -> Option<LocalAiWebSessionState> {
        let mut sessions = self.sessions();
        let record = sessions.get_mut(label)?;
        record.expire_stale_pending_context();
        Some(record.clone().into())
    }

    #[cfg(test)]
    pub(crate) fn backdate_pending_context_for_test(&self, label: &str, ms_ago: u64) {
        self.update(label, |record| {
            record.pending_context_since_ms = now_ms().saturating_sub(ms_ago);
        });
    }

    pub fn require_bound_context(&self, label: &str) -> Result<(), String> {
        let mut sessions = self.sessions();
        let record = sessions
            .get_mut(label)
            .ok_or_else(|| "本地 AI 官方会话尚未创建。".to_string())?;
        record.expire_stale_pending_context();
        if record.context_ready() {
            return Ok(());
        }
        let detail = match record.context_binding_status() {
            "restoring" => "官方会话正在恢复，请等待当前上下文确认后再发送。",
            "cached" => "当前只显示本地缓存，官方会话尚未恢复，暂不能发送。",
            "empty" => "官方页面尚未返回可用会话，请等待页面加载完成。",
            _ => "当前消息快照与官方页面不属于同一会话，已阻止发送。",
        };
        Err(detail.to_string())
    }

    pub fn diagnostic_for_provider(&self, provider_id: &str) -> Option<Value> {
        let sessions = self.sessions();
        let record = sessions
            .values()
            .filter(|record| record.provider_id == provider_id)
            .max_by_key(|record| record.updated_at_ms)?;
        let snapshot = record.semantic_event.as_ref();
        let navigation = record.navigation_event.as_ref();
        let conversations = navigation
            .and_then(|event| event.get("conversations"))
            .and_then(Value::as_array);
        let collection = navigation
            .and_then(|event| event.get("collection"))
            .and_then(Value::as_object);
        Some(serde_json::json!({
            "present": true,
            "window_status": record.window_status,
            "window_visible": record.window_visible,
            "loading": record.loading,
            "adapter_connected": record.renderer_status == "active",
            "semantic_snapshot_ready": snapshot.is_some_and(|event| event.get("type").and_then(Value::as_str) == Some("message_snapshot")),
            "composer_ready": record.semantic_live && snapshot.and_then(|event| event.get("composerReady")).and_then(Value::as_bool).unwrap_or(false),
            "context_ready": record.context_ready(),
            "context_status": record.context_binding_status(),
            "context_transition_pending": !record.pending_context_action.is_empty(),
            "page_kind": snapshot.and_then(|event| event.get("pageKind")).and_then(Value::as_str).unwrap_or("unknown"),
            "cache_status": record.cache_status(),
            "semantic_cache_status": record.event_cache_status(snapshot.is_some(), record.semantic_live),
            "navigation_cache_status": record.event_cache_status(navigation.is_some(), record.navigation_live),
            "navigation_snapshot_ready": navigation.is_some_and(|event| event.get("type").and_then(Value::as_str) == Some("conversation_snapshot")),
            "navigation_live": record.navigation_live,
            "directory_complete": collection.and_then(|value| value.get("complete")).and_then(Value::as_bool).unwrap_or(false),
            "directory_observed_count": collection.and_then(|value| value.get("observedCount")).and_then(Value::as_u64).unwrap_or(0),
            "directory_available_count": collection.and_then(|value| value.get("availableCount")).and_then(Value::as_u64).unwrap_or_else(|| conversations.map_or(0, |items| items.len() as u64)),
            "conversation_count": conversations.map_or(0, |items| items.len()),
            "project_count": navigation.and_then(|event| event.get("projects")).and_then(Value::as_array).map_or(0, Vec::len),
            "pinned_count": conversations.into_iter().flatten().filter(|item| item.get("pinned").and_then(Value::as_bool).unwrap_or(false)).count(),
            "local_conversation_count": record.conversation_snapshots.len(),
            "active_conversation": record.active_conversation_id.is_some(),
            "last_error_code": record.last_error_code,
            "last_event_kind": record.last_event_kind,
            "last_command_action": record.last_command_action,
            "last_command_ok": record.last_command_ok,
            "message_count": record.message_count,
            "assistant_message_count": record.assistant_message_count,
            "streaming": record.streaming,
            "updated_at_ms": record.updated_at_ms,
        }))
    }

    fn update(&self, label: &str, update: impl FnOnce(&mut SessionRecord)) {
        let mut sessions = self.sessions();
        let Some(record) = sessions.get_mut(label) else {
            return;
        };
        update(record);
        record.updated_at_ms = now_ms();
    }

    fn persist_snapshot(&self, label: &str) {
        let snapshot = {
            let sessions = self.sessions();
            sessions.get(label).and_then(|record| {
                record.cache_path.clone().map(|path| {
                    (
                        path,
                        record.provider_id.clone(),
                        record.semantic_event.clone(),
                        record.navigation_event.clone(),
                        record.conversation_snapshots.clone(),
                        record.cache_updated_at_ms,
                    )
                })
            })
        };
        let Some((
            path,
            provider_id,
            semantic_event,
            navigation_event,
            conversation_snapshots,
            updated_at_ms,
        )) = snapshot
        else {
            return;
        };
        let _ = snapshot_cache::store(
            &path,
            &provider_id,
            semantic_event.as_ref(),
            navigation_event.as_ref(),
            &conversation_snapshots,
            updated_at_ms,
        );
    }

    fn sessions(&self) -> MutexGuard<'_, HashMap<String, SessionRecord>> {
        self.sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

impl From<SessionRecord> for LocalAiWebSessionState {
    fn from(record: SessionRecord) -> Self {
        let cache_status = record.cache_status().to_string();
        let semantic_cache_status = record
            .event_cache_status(record.semantic_event.is_some(), record.semantic_live)
            .to_string();
        let navigation_cache_status = record
            .event_cache_status(record.navigation_event.is_some(), record.navigation_live)
            .to_string();
        let diagnostics = diagnostic_summary(&record);
        let context_ready = record.context_ready();
        let context_status = record.context_binding_status().to_string();
        let local_conversations = record
            .conversation_snapshots
            .iter()
            .map(|entry| LocalAiCachedConversation {
                id: entry.id.clone(),
                title: entry.title.clone(),
                active: record.active_conversation_id.as_deref() == Some(entry.id.as_str()),
                updated_at_ms: entry.updated_at_ms,
            })
            .collect();
        Self {
            provider_id: record.provider_id,
            window_label: record.window_label,
            window_status: record.window_status,
            window_visible: record.window_visible,
            current_url: record.current_url,
            current_host: record.current_host,
            loading: record.loading,
            renderer_status: record.renderer_status,
            last_error: record.last_error,
            last_error_code: record.last_error_code,
            semantic_event: record.semantic_event,
            navigation_event: record.navigation_event,
            composer_event: record.composer_event,
            feature_event: record.feature_event,
            ui_manifest_event: record.ui_manifest_event,
            command_result: record.command_result,
            diagnostics,
            cache_status,
            semantic_cache_status,
            navigation_cache_status,
            local_conversations,
            active_conversation_id: record.active_conversation_id,
            context_ready,
            context_status,
            cache_updated_at_ms: record.cache_updated_at_ms,
            updated_at_ms: record.updated_at_ms,
        }
    }
}

fn diagnostic_summary(record: &SessionRecord) -> Value {
    serde_json::json!({
        "lastEventKind": record.last_event_kind,
        "lastCommandAction": record.last_command_action,
        "lastCommandRequestId": record.last_command_request_id,
        "lastCommandOk": record.last_command_ok,
        "messageCount": record.message_count,
        "assistantMessageCount": record.assistant_message_count,
        "streaming": record.streaming,
        "updatedAtMs": record.updated_at_ms,
    })
}

fn safe_visible_url(url: &Url) -> String {
    if url.scheme() == "about" {
        return "about:blank".to_string();
    }
    let Some(host) = url.host_str() else {
        return String::new();
    };
    format!("{}://{}{}", url.scheme(), host, url.path())
}

fn truncate(mut value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }
    value = value.chars().take(max_chars).collect();
    value
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "state/tests.rs"]
mod tests;
