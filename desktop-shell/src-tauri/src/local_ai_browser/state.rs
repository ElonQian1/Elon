use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::Value;
use tauri::Url;

#[derive(Clone, Default)]
pub struct LocalAiBrowserRuntime {
    sessions: Arc<Mutex<HashMap<String, SessionRecord>>>,
}

#[derive(Clone)]
struct SessionRecord {
    provider_id: String,
    window_label: String,
    window_status: String,
    current_url: String,
    current_host: String,
    loading: bool,
    renderer_status: String,
    last_error: Option<String>,
    semantic_event: Option<Value>,
    command_result: Option<Value>,
    updated_at_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiWebSessionState {
    pub provider_id: String,
    pub window_label: String,
    pub window_status: String,
    pub current_url: String,
    pub current_host: String,
    pub loading: bool,
    pub renderer_status: String,
    pub last_error: Option<String>,
    pub semantic_event: Option<Value>,
    pub command_result: Option<Value>,
    pub updated_at_ms: u64,
}

impl LocalAiBrowserRuntime {
    pub fn ensure_session(&self, label: &str, provider_id: &str, renderer_status: &str) {
        let mut sessions = self.sessions();
        sessions
            .entry(label.to_string())
            .or_insert_with(|| SessionRecord {
                provider_id: provider_id.to_string(),
                window_label: label.to_string(),
                window_status: "opening".to_string(),
                current_url: String::new(),
                current_host: String::new(),
                loading: true,
                renderer_status: renderer_status.to_string(),
                last_error: None,
                semantic_event: None,
                command_result: None,
                updated_at_ms: now_ms(),
            });
    }

    pub fn mark_opening(&self, label: &str) {
        self.update(label, |record| {
            record.window_status = "opening".to_string();
            record.loading = true;
            record.last_error = None;
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
                record.last_error = None;
            } else {
                record.window_status = "blocked".to_string();
                record.loading = false;
                record.last_error = Some(
                    blocked_message
                        .unwrap_or("页面尝试离开允许的本地 AI 网页域名，已由一龙拦截。")
                        .to_string(),
                );
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

    pub fn observe_url(&self, label: &str, url: &Url) {
        let safe_url = safe_visible_url(url);
        let host = url.host_str().unwrap_or_default().to_string();
        self.update(label, |record| {
            record.current_url = safe_url;
            record.current_host = host;
        });
    }

    pub fn mark_window_status(&self, label: &str, status: &str) {
        self.update(label, |record| {
            record.window_status = status.to_string();
            if matches!(status, "closed" | "error") {
                record.loading = false;
            }
        });
    }

    pub fn record_error(&self, label: &str, detail: impl Into<String>) {
        let detail = truncate(detail.into(), 240);
        self.update(label, |record| {
            record.window_status = "error".to_string();
            record.loading = false;
            record.last_error = Some(detail);
        });
    }

    pub fn mark_command_pending(&self, label: &str) {
        self.update(label, |record| {
            record.command_result = None;
        });
    }

    pub fn record_adapter_event(&self, label: &str, kind: &str, payload: Value) {
        self.update(label, |record| match kind {
            "adapter_ready" | "message_snapshot" | "conversation_snapshot" => {
                record.renderer_status = "active".to_string();
                record.semantic_event = Some(payload);
                record.last_error = None;
            }
            "command_result" => record.command_result = Some(payload),
            "browser_diagnostic" => {
                let detail = payload
                    .get("detail")
                    .and_then(Value::as_str)
                    .unwrap_or("ChatGPT 页面暂未完成加载。")
                    .to_string();
                record.last_error = Some(truncate(detail, 240));
            }
            _ => {}
        });
    }

    pub fn snapshot(&self, label: &str) -> Option<LocalAiWebSessionState> {
        self.sessions().get(label).cloned().map(Into::into)
    }

    fn update(&self, label: &str, update: impl FnOnce(&mut SessionRecord)) {
        let mut sessions = self.sessions();
        let Some(record) = sessions.get_mut(label) else {
            return;
        };
        update(record);
        record.updated_at_ms = now_ms();
    }

    fn sessions(&self) -> MutexGuard<'_, HashMap<String, SessionRecord>> {
        self.sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

impl From<SessionRecord> for LocalAiWebSessionState {
    fn from(record: SessionRecord) -> Self {
        Self {
            provider_id: record.provider_id,
            window_label: record.window_label,
            window_status: record.window_status,
            current_url: record.current_url,
            current_host: record.current_host,
            loading: record.loading,
            renderer_status: record.renderer_status,
            last_error: record.last_error,
            semantic_event: record.semantic_event,
            command_result: record.command_result,
            updated_at_ms: record.updated_at_ms,
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
