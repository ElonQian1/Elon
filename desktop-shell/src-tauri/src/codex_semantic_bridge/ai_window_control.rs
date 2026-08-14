use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::local_ai_browser::{LocalAiNativeWindowRuntime, LocalAiNativeWindowState};

const PROVIDERS: [&str; 2] = ["chatgpt", "google-ai-mode"];

pub(super) fn validate_provider_id(provider_id: Option<&str>) -> Result<&str, String> {
    let provider_id = provider_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "AI 子窗口动作必须提供 provider_id。".to_string())?;
    PROVIDERS
        .contains(&provider_id)
        .then_some(provider_id)
        .ok_or_else(|| "provider_id 不在 AI 子窗口白名单。".to_string())
}

pub(super) fn list(app: &AppHandle, runtime: &LocalAiNativeWindowRuntime) -> Value {
    let windows = PROVIDERS
        .iter()
        .map(|provider_id| state(app, runtime, provider_id))
        .collect::<Vec<_>>();
    json!({
        "schema": "elon.tauri_ai_window_list.v1",
        "windows": windows,
        "privacy": privacy(),
    })
}

pub(super) fn capture(
    app: &AppHandle,
    runtime: &LocalAiNativeWindowRuntime,
    provider_id: &str,
) -> Value {
    json!({
        "schema": "elon.tauri_ai_window_capture.v1",
        "window": state(app, runtime, provider_id),
        "privacy": privacy(),
    })
}

pub(super) fn focus(
    app: &AppHandle,
    runtime: &LocalAiNativeWindowRuntime,
    provider_id: &str,
) -> Result<Value, String> {
    let snapshot = current_snapshot(app, runtime, provider_id)
        .ok_or_else(|| "目标一龙 AI 子窗口尚未创建。".to_string())?;
    let window = app
        .get_webview_window(&snapshot.window_label)
        .ok_or_else(|| "目标一龙 AI 子窗口当前已关闭。".to_string())?;
    window.show().map_err(display_error)?;
    if window.is_minimized().unwrap_or(false) {
        window.unminimize().map_err(display_error)?;
    }
    window.set_focus().map_err(display_error)?;
    runtime.mark_focus(&snapshot.window_label, true);
    Ok(capture(app, runtime, provider_id))
}

fn state(app: &AppHandle, runtime: &LocalAiNativeWindowRuntime, provider_id: &str) -> Value {
    let Some(snapshot) = current_snapshot(app, runtime, provider_id) else {
        return json!({
            "provider_id": provider_id,
            "phase": "not_created",
            "open": false,
            "focused": false,
            "page_ready": false,
            "root_exists": false,
            "root_child_count": 0,
            "last_error_code": null,
            "retryable": true,
            "updated_at_ms": 0,
        });
    };
    view(app, snapshot)
}

fn current_snapshot(
    app: &AppHandle,
    runtime: &LocalAiNativeWindowRuntime,
    provider_id: &str,
) -> Option<LocalAiNativeWindowState> {
    let states = runtime.states_for_provider(provider_id);
    states
        .iter()
        .find(|state| app.get_webview_window(&state.window_label).is_some())
        .cloned()
        .or_else(|| states.into_iter().next())
}

fn view(app: &AppHandle, snapshot: LocalAiNativeWindowState) -> Value {
    let open = app.get_webview_window(&snapshot.window_label).is_some();
    json!({
        "provider_id": snapshot.provider_id,
        "phase": if open { snapshot.phase.as_str() } else { "closed" },
        "open": open,
        "focused": open && snapshot.focused,
        "page_ready": open && snapshot.page_ready,
        "root_exists": snapshot.root_exists,
        "root_child_count": snapshot.root_child_count,
        "last_error_code": snapshot.last_error_code,
        "retryable": snapshot.retryable || !open,
        "updated_at_ms": snapshot.updated_at_ms,
    })
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
    format!("一龙 AI 子窗口控制失败：{error}")
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
        assert!(validate_provider_id(Some("local-ai-native-chatgpt-secret")).is_err());
        assert!(validate_provider_id(Some("gemini")).is_err());
    }
}
