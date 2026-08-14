use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::Value;
use tauri::Url;

use super::snapshot_cache;

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
    command_result: Option<Value>,
    semantic_live: bool,
    navigation_live: bool,
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
    pub command_result: Option<Value>,
    pub cache_status: String,
    pub semantic_cache_status: String,
    pub navigation_cache_status: String,
    pub cache_updated_at_ms: u64,
    pub updated_at_ms: u64,
}

impl LocalAiBrowserRuntime {
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
            let (semantic_event, navigation_event, cache_updated_at_ms) = cached
                .map(|snapshot| {
                    (
                        snapshot.semantic_event,
                        snapshot.navigation_event,
                        snapshot.updated_at_ms,
                    )
                })
                .unwrap_or_default();
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
                command_result: None,
                semantic_live: false,
                navigation_live: false,
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

    pub fn mark_navigation(
        &self,
        label: &str,
        url: &Url,
        allowed: bool,
        blocked_message: Option<&str>,
    ) {
        let safe_url = safe_visible_url(url);
        let host = url.host_str().unwrap_or_default().to_string();
        self.update(label, |record| {
            record.current_url = safe_url;
            record.current_host = host;
            if allowed {
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

    pub fn mark_command_pending(&self, label: &str) {
        self.update(label, |record| {
            record.command_result = None;
        });
    }

    pub fn record_adapter_event(&self, label: &str, kind: &str, payload: Value) {
        self.update(label, |record| match kind {
            "adapter_ready" => {
                record.renderer_status = "active".to_string();
                record.last_error = None;
                record.last_error_code = None;
            }
            "message_snapshot" => {
                record.renderer_status = "active".to_string();
                record.semantic_event = Some(payload);
                record.semantic_live = true;
                record.cache_updated_at_ms = now_ms();
                record.last_error = None;
                record.last_error_code = None;
            }
            "conversation_snapshot" => {
                record.renderer_status = "active".to_string();
                record.navigation_event = Some(payload);
                record.navigation_live = true;
                record.cache_updated_at_ms = now_ms();
                record.last_error = None;
                record.last_error_code = None;
            }
            "command_result" => record.command_result = Some(payload),
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
            record.command_result = None;
            record.semantic_live = false;
            record.navigation_live = false;
            record.cache_updated_at_ms = 0;
            record.updated_at_ms = now_ms();
            record.cache_path.clone()
        };
        if let Some(path) = cache_path {
            snapshot_cache::clear(&path);
        }
    }

    pub fn snapshot(&self, label: &str) -> Option<LocalAiWebSessionState> {
        self.sessions().get(label).cloned().map(Into::into)
    }

    pub fn diagnostic_for_provider(&self, provider_id: &str) -> Option<Value> {
        let sessions = self.sessions();
        let record = sessions
            .values()
            .filter(|record| record.provider_id == provider_id)
            .max_by_key(|record| record.updated_at_ms)?;
        let snapshot = record.semantic_event.as_ref();
        Some(serde_json::json!({
            "window_status": record.window_status,
            "window_visible": record.window_visible,
            "loading": record.loading,
            "adapter_connected": record.renderer_status == "active",
            "semantic_snapshot_ready": snapshot.is_some_and(|event| event.get("type").and_then(Value::as_str) == Some("message_snapshot")),
            "composer_ready": record.semantic_live && snapshot.and_then(|event| event.get("composerReady")).and_then(Value::as_bool).unwrap_or(false),
            "page_kind": snapshot.and_then(|event| event.get("pageKind")).and_then(Value::as_str).unwrap_or("unknown"),
            "cache_status": record.cache_status(),
            "last_error_code": record.last_error_code,
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
                        record.cache_updated_at_ms,
                    )
                })
            })
        };
        let Some((path, provider_id, semantic_event, navigation_event, updated_at_ms)) = snapshot
        else {
            return;
        };
        let _ = snapshot_cache::store(
            &path,
            &provider_id,
            semantic_event.as_ref(),
            navigation_event.as_ref(),
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
            command_result: record.command_result,
            cache_status,
            semantic_cache_status,
            navigation_cache_status,
            cache_updated_at_ms: record.cache_updated_at_ms,
            updated_at_ms: record.updated_at_ms,
        }
    }
}

impl SessionRecord {
    fn mark_snapshot_cached(&mut self) {
        if self.semantic_event.is_some() {
            self.semantic_live = false;
        }
        if self.navigation_event.is_some() {
            self.navigation_live = false;
        }
    }

    fn cache_status(&self) -> &'static str {
        let has_semantic = self.semantic_event.is_some();
        let has_navigation = self.navigation_event.is_some();
        if !has_semantic && !has_navigation {
            return "empty";
        }
        if (!has_semantic || self.semantic_live) && (!has_navigation || self.navigation_live) {
            "live"
        } else {
            "cached"
        }
    }

    fn event_cache_status(&self, present: bool, live: bool) -> &'static str {
        if !present {
            "empty"
        } else if live {
            "live"
        } else {
            "cached"
        }
    }
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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_and_navigation_snapshots_do_not_overwrite_each_other() {
        let runtime = LocalAiBrowserRuntime::default();
        runtime.ensure_session("session", "chatgpt", "reserved");
        runtime.record_adapter_event(
            "session",
            "message_snapshot",
            json!({"type": "message_snapshot", "messages": [{"id": "answer"}]}),
        );
        runtime.record_adapter_event(
            "session",
            "conversation_snapshot",
            json!({"type": "conversation_snapshot", "projects": [{"id": "project"}]}),
        );

        let snapshot = runtime.snapshot("session").unwrap();
        assert_eq!(snapshot.semantic_event.unwrap()["type"], "message_snapshot");
        assert_eq!(
            snapshot.navigation_event.unwrap()["type"],
            "conversation_snapshot"
        );
    }

    #[test]
    fn background_opening_never_reports_a_visible_official_window() {
        let runtime = LocalAiBrowserRuntime::default();
        runtime.ensure_session("session", "chatgpt", "connecting");
        runtime.mark_opening("session", false);

        let background = runtime.snapshot("session").unwrap();
        assert_eq!(background.window_status, "opening");
        assert!(!background.window_visible);

        runtime.mark_opening("session", true);
        assert!(runtime.snapshot("session").unwrap().window_visible);
    }

    #[test]
    fn provider_diagnostic_exposes_readiness_without_identity_or_page_content() {
        let runtime = LocalAiBrowserRuntime::default();
        runtime.ensure_session("local-ai-chatgpt-owner-secret", "chatgpt", "connecting");
        runtime.record_adapter_event(
            "local-ai-chatgpt-owner-secret",
            "message_snapshot",
            json!({
                "type": "message_snapshot",
                "composerReady": true,
                "pageKind": "home",
                "draft": "private prompt",
                "messages": [{"content": "private answer"}],
            }),
        );
        runtime.record_adapter_event(
            "local-ai-chatgpt-owner-secret",
            "browser_diagnostic",
            json!({
                "kind": "adapter_bootstrap_failed",
                "detail": "private exception detail",
            }),
        );

        let diagnostic = runtime.diagnostic_for_provider("chatgpt").unwrap();
        assert_eq!(diagnostic["adapter_connected"], true);
        assert_eq!(diagnostic["semantic_snapshot_ready"], true);
        assert_eq!(diagnostic["composer_ready"], true);
        assert_eq!(diagnostic["last_error_code"], "adapter_bootstrap_failed");
        let encoded = diagnostic.to_string();
        assert!(!encoded.contains("owner-secret"));
        assert!(!encoded.contains("private prompt"));
        assert!(!encoded.contains("private answer"));
        assert!(!encoded.contains("private exception detail"));
    }

    #[test]
    fn navigation_keeps_the_snapshot_visible_but_marks_it_cached() {
        let runtime = LocalAiBrowserRuntime::default();
        runtime.ensure_session("session", "chatgpt", "reserved");
        runtime.record_adapter_event(
            "session",
            "message_snapshot",
            json!({"type": "message_snapshot", "messages": []}),
        );
        assert_eq!(runtime.snapshot("session").unwrap().cache_status, "live");

        runtime.mark_navigation(
            "session",
            &Url::parse("https://chatgpt.com/c/example").unwrap(),
            true,
            None,
        );
        let cached = runtime.snapshot("session").unwrap();
        assert_eq!(cached.cache_status, "cached");
        assert!(cached.semantic_event.is_some());

        runtime.clear_snapshots("session");
        let cleared = runtime.snapshot("session").unwrap();
        assert_eq!(cleared.cache_status, "empty");
        assert!(cleared.semantic_event.is_none());
    }
}
