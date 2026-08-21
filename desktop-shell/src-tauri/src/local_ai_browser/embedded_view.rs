use serde::Deserialize;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, State, Webview,
};

use crate::{internal_browser::raise_webview, MAIN_WINDOW_LABEL};

use super::{
    ensure_runtime_session, ensure_session_webview, provider, resolve_owner_fingerprint,
    window_label, LocalAiBrowserRuntime, LocalAiWebSessionState,
};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddedWebviewBounds {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl EmbeddedWebviewBounds {
    pub(crate) fn validate(self) -> Result<Self, String> {
        let values = [self.x, self.y, self.width, self.height];
        if values.iter().any(|value| !value.is_finite()) {
            return Err("内部网页标签区域无效。".to_string());
        }
        if self.x < 0.0
            || self.y < 0.0
            || self.width < 320.0
            || self.height < 220.0
            || self.width > 16_384.0
            || self.height > 16_384.0
        {
            return Err("内部网页标签区域超出安全范围。".to_string());
        }
        Ok(self)
    }

    pub(crate) fn position(self) -> LogicalPosition<f64> {
        LogicalPosition::new(self.x, self.y)
    }

    pub(crate) fn size(self) -> LogicalSize<f64> {
        LogicalSize::new(self.width, self.height)
    }
}

#[tauri::command]
pub(crate) async fn present_local_ai_web_session_embedded(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
    bounds: EmbeddedWebviewBounds,
    content_only: Option<bool>,
) -> Result<LocalAiWebSessionState, String> {
    let provider = provider(&provider_id)?;
    let fingerprint = resolve_owner_fingerprint(&app, provider, &owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = window_label(provider, &fingerprint);
    ensure_runtime_session(&app, runtime.inner(), provider, &fingerprint, &label)?;
    let content_only = content_only.unwrap_or(false);
    if content_only {
        let state = runtime
            .snapshot(&label)
            .ok_or_else(|| format!("{} 本地会话状态不可用。", provider.display_name))?;
        if !answer_surface_ready(&state) {
            return Err("官网回答区域尚未就绪，已继续显示本机缓存。".to_string());
        }
    }
    present(&app, &label, bounds, content_only)?;
    runtime.mark_window_status(&label, "ready");
    runtime.mark_window_visible(&label, true);
    runtime
        .snapshot(&label)
        .ok_or_else(|| format!("{} 本地会话状态不可用。", provider.display_name))
}

#[tauri::command]
pub(crate) async fn hide_local_ai_web_session_embedded(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
) -> Result<LocalAiWebSessionState, String> {
    let provider = provider(&provider_id)?;
    let fingerprint = resolve_owner_fingerprint(&app, provider, &owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = window_label(provider, &fingerprint);
    ensure_runtime_session(&app, runtime.inner(), provider, &fingerprint, &label)?;
    hide(&app, &label)?;
    runtime.mark_window_visible(&label, false);
    runtime
        .snapshot(&label)
        .ok_or_else(|| format!("{} 本地会话状态不可用。", provider.display_name))
}

pub(crate) fn park(webview: &Webview) -> Result<(), String> {
    let parent_size = webview.window().inner_size().map_err(display_error)?;
    let (parked_x, parked_y) = parked_position(parent_size.width, parent_size.height);
    webview.hide().map_err(display_error)?;
    webview
        .set_position(PhysicalPosition::new(parked_x, parked_y))
        .map_err(display_error)?;
    webview.show().map_err(display_error)
}

// 主窗口最小化或调整过程中偶发的瞬时尺寸；小于这个阈值一律视为不可信。
const PARK_MIN_TRUSTED_SIZE: u32 = 100;
// 停放坐标额外留出的安全边距，避免四舍五入或多显示器缩放导致贴边穿帮。
const PARK_MARGIN: i32 = 64;
// 尺寸不可信或换算溢出时使用的固定兜底坐标，足够远离任何真实屏幕范围。
const PARK_FALLBACK_OFFSET: i32 = 20_000;

// 窗口最小化、DPI 切换瞬间或尺寸溢出时不能信任这次测量，否则会把停放坐标
// 算回 (0,0) 附近，让本该隐藏的完整官方页面重新盖住原生界面。
fn parked_position(width: u32, height: u32) -> (i32, i32) {
    if width < PARK_MIN_TRUSTED_SIZE || height < PARK_MIN_TRUSTED_SIZE {
        return (PARK_FALLBACK_OFFSET, PARK_FALLBACK_OFFSET);
    }
    let x = i32::try_from(width)
        .unwrap_or(PARK_FALLBACK_OFFSET)
        .saturating_add(PARK_MARGIN);
    let y = i32::try_from(height)
        .unwrap_or(PARK_FALLBACK_OFFSET)
        .saturating_add(PARK_MARGIN);
    (x, y)
}

pub(crate) fn present(
    app: &AppHandle,
    webview_label: &str,
    bounds: EmbeddedWebviewBounds,
    content_only: bool,
) -> Result<(), String> {
    let bounds = bounds.validate()?;
    let webview = app
        .get_webview(webview_label)
        .ok_or_else(|| "官方网页尚未创建。".to_string())?;
    let main_window = app
        .get_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "一龙主窗口不可用。".to_string())?;
    webview.hide().map_err(display_error)?;
    if webview.window().label() != MAIN_WINDOW_LABEL {
        webview.reparent(&main_window).map_err(display_error)?;
    }
    if let Some(popout) = app.get_window(webview_label) {
        popout.hide().map_err(display_error)?;
    }
    set_content_surface_mode(&webview, content_only)?;
    webview
        .set_position(bounds.position())
        .map_err(display_error)?;
    webview.set_size(bounds.size()).map_err(display_error)?;
    webview.show().map_err(display_error)?;
    raise_webview(&webview)?;
    if content_only {
        Ok(())
    } else {
        webview.set_focus().map_err(display_error)
    }
}

pub(crate) fn hide(app: &AppHandle, webview_label: &str) -> Result<(), String> {
    if let Some(webview) = app.get_webview(webview_label) {
        set_content_surface_mode(&webview, false)?;
        park(&webview)?;
    }
    if let Some(popout) = app.get_window(webview_label) {
        popout.hide().map_err(display_error)?;
    }
    let main_webview = app
        .get_webview(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "一龙主工作台 WebView 不可用。".to_string())?;
    main_webview.show().map_err(display_error)?;
    raise_webview(&main_webview)?;
    main_webview.set_focus().map_err(display_error)
}

fn set_content_surface_mode(webview: &Webview, enabled: bool) -> Result<(), String> {
    let script = if enabled {
        answer_surface_script()
    } else {
        restore_surface_script()
    };
    webview.eval(script).map_err(display_error)
}

fn answer_surface_ready(state: &LocalAiWebSessionState) -> bool {
    if state.loading
        || state.semantic_cache_status != "live"
        || state.window_status == "blocked"
        || state.window_status == "error"
    {
        return false;
    }
    let Some(event) = state.semantic_event.as_ref() else {
        return false;
    };
    if event
        .get("streaming")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    semantic_event_has_completed_assistant(event)
}

fn semantic_event_has_completed_assistant(event: &serde_json::Value) -> bool {
    event
        .get("messages")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|messages| {
            messages.iter().any(|message| {
                message.get("role").and_then(serde_json::Value::as_str) == Some("assistant")
                    && message.get("state").and_then(serde_json::Value::as_str) != Some("streaming")
            })
        })
}

fn answer_surface_script() -> &'static str {
    r#"
(function () {
  'use strict';
  var styleId = 'elon-official-answer-surface-style';
  var backdropId = 'elon-official-answer-surface-backdrop';
  var providerAttribute = 'data-elon-official-answer-provider';
  document.documentElement.removeAttribute('data-elon-official-answer-surface');
  document.documentElement.removeAttribute(providerAttribute);
  document.querySelectorAll('[data-elon-official-answer-root]').forEach(function (node) {
    node.removeAttribute('data-elon-official-answer-root');
  });
  var oldStyle = document.getElementById(styleId);
  if (oldStyle) oldStyle.remove();
  var oldBackdrop = document.getElementById(backdropId);
  if (oldBackdrop) oldBackdrop.remove();

  var root = document.querySelector('main, [role="main"]');
  if (!(root instanceof HTMLElement)) return;

  var provider = /(^|\.)chatgpt\.com$/i.test(window.location.hostname)
    ? 'chatgpt'
    : 'google-ai-mode';

  var style = document.createElement('style');
  style.id = styleId;
  style.textContent = [
    'html[data-elon-official-answer-surface], html[data-elon-official-answer-surface] body { overflow: hidden !important; }',
    'html[data-elon-official-answer-provider="google-ai-mode"], html[data-elon-official-answer-provider="google-ai-mode"] body { background: #202124 !important; }',
    'html[data-elon-official-answer-provider="chatgpt"], html[data-elon-official-answer-provider="chatgpt"] body { background: var(--main-surface-primary, #212121) !important; }',
    '#elon-official-answer-surface-backdrop { position: fixed !important; inset: 0 !important; z-index: 2147483000 !important; background: #202124 !important; }',
    'html[data-elon-official-answer-provider="chatgpt"] #elon-official-answer-surface-backdrop { background: var(--main-surface-primary, #212121) !important; }',
    '[data-elon-official-answer-root] { position: fixed !important; inset: 0 !important; z-index: 2147483001 !important; box-sizing: border-box !important; width: 100vw !important; max-width: none !important; height: 100vh !important; max-height: none !important; margin: 0 !important; overflow: auto !important; overscroll-behavior: contain !important; border-radius: 0 !important; }',
    'html[data-elon-official-answer-provider="google-ai-mode"] [data-elon-official-answer-root] { background: #202124 !important; }',
    'html[data-elon-official-answer-provider="chatgpt"] [data-elon-official-answer-root] { background: var(--main-surface-primary, #212121) !important; scrollbar-gutter: stable; }',
    'html[data-elon-official-answer-provider="chatgpt"] [data-elon-official-answer-root] [data-message-author-role] { box-sizing: border-box !important; width: min(100%, 48rem) !important; max-width: 48rem !important; margin-inline: auto !important; }',
    '[data-elon-official-answer-root] form:has(textarea), [data-elon-official-answer-root] form:has([contenteditable]) { opacity: 0 !important; pointer-events: none !important; }',
    '[data-elon-official-answer-root] textarea, [data-elon-official-answer-root] [contenteditable="true"], [data-elon-official-answer-root] [contenteditable="plaintext-only"] { opacity: 0 !important; pointer-events: none !important; }'
  ].join('\n');
  document.head.appendChild(style);

  var backdrop = document.createElement('div');
  backdrop.id = backdropId;
  backdrop.setAttribute('aria-hidden', 'true');
  document.body.appendChild(backdrop);
  root.setAttribute('data-elon-official-answer-root', 'true');
  document.documentElement.setAttribute(providerAttribute, provider);
  document.documentElement.setAttribute('data-elon-official-answer-surface', 'true');

  window.requestAnimationFrame(function () {
    var candidates = Array.from(root.querySelectorAll(
      '[data-testid^="conversation-turn-"][data-message-author-role="assistant"], [data-message-author-role="assistant"], [data-sfc-cp][data-hveid], article, [role="article"]'
    ));
    var target = candidates.reverse().find(function (node) {
      var rect = node.getBoundingClientRect();
      return rect.width > 240 && rect.height > 40;
    });
    if (target && typeof target.scrollIntoView === 'function') {
      target.scrollIntoView({ block: 'center', inline: 'nearest' });
    }
  });
})();
"#
}

fn restore_surface_script() -> &'static str {
    r#"
(function () {
  'use strict';
  document.documentElement.removeAttribute('data-elon-official-answer-surface');
  document.documentElement.removeAttribute('data-elon-official-answer-provider');
  document.querySelectorAll('[data-elon-official-answer-root]').forEach(function (node) {
    node.removeAttribute('data-elon-official-answer-root');
  });
  var style = document.getElementById('elon-official-answer-surface-style');
  if (style) style.remove();
  var backdrop = document.getElementById('elon-official-answer-surface-backdrop');
  if (backdrop) backdrop.remove();
})();
"#
}

pub(crate) fn park_if_background(
    app: &AppHandle,
    runtime: &LocalAiBrowserRuntime,
    webview_label: &str,
) -> Result<(), String> {
    if runtime
        .snapshot(webview_label)
        .is_some_and(|state| !state.window_visible)
    {
        hide(app, webview_label)?;
    }
    Ok(())
}

/// 停放坐标只在停放那一刻按当前主窗口尺寸算一次；主窗口后续被拖大、还原或
/// 最大化后必须重新停放，否则旧坐标可能落回新的可见区域内，重新盖住原生界面。
pub(crate) fn reflow_parked_sessions(app: &AppHandle) {
    let runtime = app.state::<LocalAiBrowserRuntime>();
    for label in runtime.parked_session_labels() {
        let _ = hide(app, &label);
    }
}

pub(crate) fn restore_popout(app: &AppHandle, webview_label: &str) -> Result<(), String> {
    let webview = app
        .get_webview(webview_label)
        .ok_or_else(|| "官方网页尚未创建。".to_string())?;
    let popout = app
        .get_window(webview_label)
        .ok_or_else(|| "官方网页窗口已关闭，请重新打开。".to_string())?;
    webview.hide().map_err(display_error)?;
    if webview.window().label() != webview_label {
        webview.reparent(&popout).map_err(display_error)?;
    }
    webview
        .set_position(PhysicalPosition::new(0, 0))
        .map_err(display_error)?;
    webview
        .set_size(popout.inner_size().map_err(display_error)?)
        .map_err(display_error)?;
    webview.show().map_err(display_error)?;
    popout.unminimize().map_err(display_error)?;
    popout.show().map_err(display_error)?;
    popout.set_focus().map_err(display_error)
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parked_position_uses_measured_size_with_a_safety_margin() {
        assert_eq!(parked_position(1280, 800), (1280 + PARK_MARGIN, 800 + PARK_MARGIN));
    }

    #[test]
    fn parked_position_falls_back_to_a_fixed_offset_for_untrusted_sizes() {
        assert_eq!(parked_position(0, 0), (PARK_FALLBACK_OFFSET, PARK_FALLBACK_OFFSET));
        assert_eq!(
            parked_position(PARK_MIN_TRUSTED_SIZE - 1, 800),
            (PARK_FALLBACK_OFFSET, PARK_FALLBACK_OFFSET)
        );
        assert_eq!(
            parked_position(1280, PARK_MIN_TRUSTED_SIZE - 1),
            (PARK_FALLBACK_OFFSET, PARK_FALLBACK_OFFSET)
        );
    }

    #[test]
    fn embedded_bounds_reject_invalid_or_tiny_surfaces() {
        assert!(EmbeddedWebviewBounds {
            x: 0.0,
            y: 58.0,
            width: 900.0,
            height: 640.0,
        }
        .validate()
        .is_ok());
        assert!(EmbeddedWebviewBounds {
            x: -1.0,
            y: 0.0,
            width: 900.0,
            height: 640.0,
        }
        .validate()
        .is_err());
        assert!(EmbeddedWebviewBounds {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn answer_surface_requires_a_completed_assistant_message() {
        assert!(semantic_event_has_completed_assistant(&json!({
            "messages": [
                {"role":"user","state":"completed"},
                {"role":"assistant","state":"completed"}
            ]
        })));
        assert!(!semantic_event_has_completed_assistant(&json!({
            "messages": [{"role":"assistant","state":"streaming"}]
        })));
        assert!(!semantic_event_has_completed_assistant(&json!({
            "messages": [{"role":"user","state":"completed"}]
        })));
    }

    #[test]
    fn answer_surface_script_is_pc_scoped_and_reversible() {
        let apply = answer_surface_script();
        let restore = restore_surface_script();
        assert!(apply.contains("main, [role=\"main\"]"));
        assert!(apply.contains("data-elon-official-answer-root"));
        assert!(apply.contains("data-elon-official-answer-provider"));
        assert!(apply.contains(r"chatgpt\.com"));
        assert!(apply.contains("max-width: 48rem"));
        assert!(apply.contains("form:has(textarea)"));
        assert!(restore.contains("style.remove()"));
        assert!(restore.contains("backdrop.remove()"));
    }
}
