use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tauri::{AppHandle, Manager, State, WebviewWindow};

use super::{ensure_session_webview, native_window, owner_fingerprint, provider};

#[derive(Clone, Default)]
pub struct LocalAiNativeWindowRuntime {
    windows: Arc<Mutex<HashMap<String, NativeWindowRecord>>>,
}

#[derive(Clone)]
struct NativeWindowRecord {
    provider_id: String,
    window_label: String,
    phase: String,
    focused: bool,
    page_ready: bool,
    root_exists: bool,
    root_child_count: u32,
    last_error_code: Option<String>,
    updated_at_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiNativeWindowState {
    pub provider_id: String,
    pub window_label: String,
    pub phase: String,
    pub focused: bool,
    pub page_ready: bool,
    pub root_exists: bool,
    pub root_child_count: u32,
    pub last_error_code: Option<String>,
    pub retryable: bool,
    pub updated_at_ms: u64,
}

#[tauri::command]
pub async fn get_local_ai_native_window_state(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiNativeWindowRuntime>,
    provider_id: String,
    owner_key: String,
) -> Result<LocalAiNativeWindowState, String> {
    let provider = provider(&provider_id)?;
    let fingerprint = owner_fingerprint(&owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = native_window::native_window_label(provider, &fingerprint);
    if app.get_webview_window(&label).is_none() {
        if runtime.snapshot(&label).is_none() {
            return Err(format!("尚未创建 {} 一龙聊天窗。", provider.display_name));
        }
        runtime.mark_closed(&label, provider.id);
    }
    runtime
        .snapshot(&label)
        .ok_or_else(|| format!("尚未创建 {} 一龙聊天窗。", provider.display_name))
}

impl LocalAiNativeWindowRuntime {
    pub fn mark_creating(&self, label: &str, provider_id: &str) {
        self.windows().insert(
            label.to_string(),
            NativeWindowRecord {
                provider_id: provider_id.to_string(),
                window_label: label.to_string(),
                phase: "creating".to_string(),
                focused: false,
                page_ready: false,
                root_exists: false,
                root_child_count: 0,
                last_error_code: None,
                updated_at_ms: now_ms(),
            },
        );
    }

    pub fn mark_created(&self, label: &str) {
        self.update(label, |record| {
            if record.phase == "creating" {
                record.phase = "loading".to_string();
                record.last_error_code = None;
            }
        });
    }

    pub fn mark_recovering(&self, label: &str) {
        self.update(label, |record| {
            if matches!(record.phase.as_str(), "error" | "closed") {
                record.phase = "loading".to_string();
                record.page_ready = false;
                record.last_error_code = None;
            }
        });
    }

    pub fn mark_page_started(&self, label: &str) {
        self.update(label, |record| {
            record.phase = "loading".to_string();
            record.page_ready = false;
            record.last_error_code = None;
        });
    }

    pub fn mark_page_finished(&self, label: &str) {
        self.update(label, |record| {
            if !matches!(record.phase.as_str(), "ready" | "error") {
                record.phase = "loaded".to_string();
            }
        });
    }

    pub fn mark_health(
        &self,
        label: &str,
        phase: &str,
        root_exists: bool,
        root_child_count: u32,
    ) {
        self.update(label, |record| {
            record.root_exists = root_exists;
            record.root_child_count = root_child_count;
            match phase {
                "load" | "settled" if root_exists && root_child_count > 0 => {
                    record.phase = "ready".to_string();
                    record.page_ready = true;
                    record.last_error_code = None;
                }
                "settled" => {
                    record.phase = "error".to_string();
                    record.page_ready = false;
                    record.last_error_code = Some("root_empty".to_string());
                }
                "window_error" | "promise_rejection" => {
                    record.phase = "error".to_string();
                    record.page_ready = false;
                    record.last_error_code = Some("page_runtime_error".to_string());
                }
                _ => {}
            }
        });
    }

    pub fn mark_focus(&self, label: &str, focused: bool) {
        self.update(label, |record| record.focused = focused);
    }

    pub fn mark_error(&self, label: &str, code: &str) {
        self.update(label, |record| {
            record.phase = "error".to_string();
            record.page_ready = false;
            record.last_error_code = Some(code.to_string());
        });
    }

    pub fn mark_closed(&self, label: &str, provider_id: &str) {
        let mut windows = self.windows();
        let record = windows
            .entry(label.to_string())
            .or_insert_with(|| NativeWindowRecord {
                provider_id: provider_id.to_string(),
                window_label: label.to_string(),
                phase: "closed".to_string(),
                focused: false,
                page_ready: false,
                root_exists: false,
                root_child_count: 0,
                last_error_code: None,
                updated_at_ms: now_ms(),
            });
        record.phase = "closed".to_string();
        record.focused = false;
        record.page_ready = false;
        record.updated_at_ms = now_ms();
    }

    pub fn snapshot(&self, label: &str) -> Option<LocalAiNativeWindowState> {
        self.windows().get(label).cloned().map(Into::into)
    }

    fn update(&self, label: &str, update: impl FnOnce(&mut NativeWindowRecord)) {
        let mut windows = self.windows();
        let Some(record) = windows.get_mut(label) else {
            return;
        };
        update(record);
        record.updated_at_ms = now_ms();
    }

    fn windows(&self) -> MutexGuard<'_, HashMap<String, NativeWindowRecord>> {
        self.windows
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

impl From<NativeWindowRecord> for LocalAiNativeWindowState {
    fn from(record: NativeWindowRecord) -> Self {
        let retryable = matches!(record.phase.as_str(), "error" | "closed");
        Self {
            provider_id: record.provider_id,
            window_label: record.window_label,
            phase: record.phase,
            focused: record.focused,
            page_ready: record.page_ready,
            root_exists: record.root_exists,
            root_child_count: record.root_child_count,
            last_error_code: record.last_error_code,
            retryable,
            updated_at_ms: record.updated_at_ms,
        }
    }
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

    #[test]
    fn settled_root_is_required_before_window_becomes_ready() {
        let runtime = LocalAiNativeWindowRuntime::default();
        runtime.mark_creating("window", "chatgpt");
        runtime.mark_created("window");
        runtime.mark_health("window", "settled", true, 0);
        let failed = runtime.snapshot("window").unwrap();
        assert_eq!(failed.phase, "error");
        assert_eq!(failed.last_error_code.as_deref(), Some("root_empty"));
        assert!(failed.retryable);

        runtime.mark_recovering("window");
        runtime.mark_health("window", "settled", true, 1);
        let ready = runtime.snapshot("window").unwrap();
        assert_eq!(ready.phase, "ready");
        assert!(ready.page_ready);
        assert!(!ready.retryable);
    }

    #[test]
    fn late_page_finished_event_does_not_regress_ready_state() {
        let runtime = LocalAiNativeWindowRuntime::default();
        runtime.mark_creating("window", "chatgpt");
        runtime.mark_health("window", "load", true, 1);
        runtime.mark_page_finished("window");

        let ready = runtime.snapshot("window").unwrap();
        assert_eq!(ready.phase, "ready");
        assert!(ready.page_ready);
    }

    #[test]
    fn late_created_event_does_not_regress_ready_state() {
        let runtime = LocalAiNativeWindowRuntime::default();
        runtime.mark_creating("window", "chatgpt");
        runtime.mark_health("window", "load", true, 1);
        runtime.mark_created("window");

        assert_eq!(runtime.snapshot("window").unwrap().phase, "ready");
    }
}
