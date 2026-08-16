use serde_json::{json, Value};
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::local_ai_browser::LocalAiBrowserRuntime;

const PROVIDERS: [&str; 2] = ["chatgpt", "google-ai-mode"];

pub(super) fn validate_provider_id(provider_id: Option<&str>) -> Result<&str, String> {
    let provider_id = provider_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "AI 官方窗口动作必须提供 provider_id。".to_string())?;
    PROVIDERS
        .contains(&provider_id)
        .then_some(provider_id)
        .ok_or_else(|| "provider_id 不在 AI 官方窗口白名单。".to_string())
}

pub(super) fn list(app: &AppHandle, web_runtime: &LocalAiBrowserRuntime) -> Value {
    let windows = PROVIDERS
        .iter()
        .map(|provider_id| state(app, web_runtime, provider_id))
        .collect::<Vec<_>>();
    json!({
        "schema": "elon.tauri_ai_window_list.v1",
        "windows": windows,
        "privacy": privacy(),
    })
}

pub(super) fn capture(
    app: &AppHandle,
    web_runtime: &LocalAiBrowserRuntime,
    provider_id: &str,
) -> Value {
    json!({
        "schema": "elon.tauri_ai_window_capture.v1",
        "window": state(app, web_runtime, provider_id),
        "privacy": privacy(),
    })
}

pub(super) fn focus(
    app: &AppHandle,
    web_runtime: &LocalAiBrowserRuntime,
    provider_id: &str,
) -> Result<Value, String> {
    let window = official_window(app, provider_id)
        .ok_or_else(|| "目标 AI 官方网页会话尚未创建或已经关闭。".to_string())?;
    window.show().map_err(display_error)?;
    if window.is_minimized().unwrap_or(false) {
        window.unminimize().map_err(display_error)?;
    }
    window.set_focus().map_err(display_error)?;
    Ok(capture(app, web_runtime, provider_id))
}

fn state(app: &AppHandle, web_runtime: &LocalAiBrowserRuntime, provider_id: &str) -> Value {
    let official_session = web_runtime.diagnostic_for_provider(provider_id);
    let window = official_window(app, provider_id);
    view(provider_id, window.as_ref(), official_session)
}

fn official_window(app: &AppHandle, provider_id: &str) -> Option<WebviewWindow> {
    let prefix = format!("local-ai-{provider_id}-");
    app.webview_windows()
        .into_iter()
        .find_map(|(label, window)| label.starts_with(&prefix).then_some(window))
}

fn view(
    provider_id: &str,
    window: Option<&WebviewWindow>,
    official_session: Option<Value>,
) -> Value {
    let open = window.is_some();
    let status = official_session
        .as_ref()
        .and_then(|session| session.get("window_status"))
        .and_then(Value::as_str);
    let loading = official_session
        .as_ref()
        .and_then(|session| session.get("loading"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let page_ready = open
        && official_session
            .as_ref()
            .and_then(|session| session.get("semantic_snapshot_ready"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let last_error_code = official_session
        .as_ref()
        .and_then(|session| session.get("last_error_code"))
        .cloned()
        .unwrap_or(Value::Null);
    let updated_at_ms = official_session
        .as_ref()
        .and_then(|session| session.get("updated_at_ms"))
        .and_then(Value::as_u64)
        .unwrap_or_default();
    json!({
        "provider_id": provider_id,
        "phase": official_phase(open, status, loading, page_ready),
        "open": open,
        "focused": window.and_then(|window| window.is_focused().ok()).unwrap_or(false),
        "page_ready": page_ready,
        "root_exists": page_ready,
        "root_child_count": usize::from(page_ready),
        "last_error_code": last_error_code,
        "retryable": !open || matches!(status, Some("blocked" | "error" | "closed")),
        "updated_at_ms": updated_at_ms,
        "official_session": official_session,
    })
}

fn official_phase(
    open: bool,
    status: Option<&str>,
    loading: bool,
    page_ready: bool,
) -> &'static str {
    if !open {
        return if status.is_some() {
            "closed"
        } else {
            "not_created"
        };
    }
    match status {
        Some("opening") => "creating",
        Some("loading") => "loading",
        Some("ready" | "minimized") => "ready",
        Some("blocked" | "error") => "error",
        Some("closed") => "closed",
        _ if loading => "loading",
        _ if page_ready => "ready",
        _ => "loading",
    }
}

fn privacy() -> Value {
    json!({
        "window_labels": false,
        "owner_fingerprints": false,
        "urls": false,
        "page_text": false,
        "cookies": false,
        "tokens": false,
    })
}

fn display_error(error: impl std::fmt::Display) -> String {
    format!("AI 官方网页窗口控制失败：{error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_ids_are_fixed_and_do_not_accept_window_labels() {
        assert_eq!(validate_provider_id(Some("chatgpt")).unwrap(), "chatgpt");
        assert_eq!(
            validate_provider_id(Some("google-ai-mode")).unwrap(),
            "google-ai-mode"
        );
        assert!(validate_provider_id(Some("arbitrary-window-label-secret")).is_err());
        assert!(validate_provider_id(Some("gemini")).is_err());
    }

    #[test]
    fn official_phase_uses_the_production_webview_lifecycle() {
        assert_eq!(official_phase(false, None, false, false), "not_created");
        assert_eq!(official_phase(false, Some("ready"), false, true), "closed");
        assert_eq!(
            official_phase(true, Some("opening"), true, false),
            "creating"
        );
        assert_eq!(official_phase(true, Some("ready"), false, true), "ready");
        assert_eq!(official_phase(true, Some("blocked"), false, false), "error");
    }
}
