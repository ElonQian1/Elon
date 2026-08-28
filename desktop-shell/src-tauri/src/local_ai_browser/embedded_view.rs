use serde::Deserialize;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, State, Url, Webview,
};

use crate::{internal_browser::raise_webview, MAIN_WINDOW_LABEL};

use super::{
    ensure_runtime_session, ensure_session_webview, provider, resolve_owner_fingerprint,
    reconnect_adapter, window_label, LocalAiBrowserRuntime, LocalAiWebSessionState,
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
) -> Result<LocalAiWebSessionState, String> {
    let provider = provider(&provider_id)?;
    let fingerprint = resolve_owner_fingerprint(&app, provider, &owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = window_label(provider, &fingerprint);
    ensure_runtime_session(&app, runtime.inner(), provider, &fingerprint, &label)?;
    let page = app
        .get_webview(&label)
        .ok_or_else(|| "官方网页尚未创建。".to_string())?;
    present(&app, &label, bounds)?;
    runtime.mark_window_status(&label, "ready");
    runtime.mark_window_visible(&label, true);
    // Android resumes the page adapter and immediately snapshots the current
    // document whenever the official WebView returns to the foreground. Do the
    // same for WebView2 so a parked page cannot expose an old bridge or old
    // conversation after the native tab becomes visible again.
    reconnect_adapter(provider, &page);
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
    let (parked_x, parked_y) = parked_position();
    webview.hide().map_err(display_error)?;
    webview
        .set_position(PhysicalPosition::new(parked_x, parked_y))
        .map_err(display_error)?;
    webview.show().map_err(display_error)
}

pub(crate) fn reload_after_stop(webview: &Webview) -> Result<(), String> {
    // 与 APK WebView 的新会话恢复一致：先终止可能仍占用旧会话的导航，
    // 再重载首页。忽略 stop 的瞬时错误，真正的 reload 结果仍严格返回。
    let _ = webview.eval("window.stop();");
    webview.reload().map_err(display_error)
}

pub(crate) fn navigate_after_stop(webview: &Webview, url: Url) -> Result<(), String> {
    let _ = webview.eval("window.stop();");
    webview.navigate(url).map_err(display_error)
}

// 嵌入区域的最大允许尺寸是 16_384；使用更远的固定坐标，窗口最大化、DPI
// 切换和随后 resize 都不会把后台官网页重新带回可见区域。
const PARK_OFFSET: i32 = 20_000;

fn parked_position() -> (i32, i32) {
    (PARK_OFFSET, PARK_OFFSET)
}

pub(crate) fn present(
    app: &AppHandle,
    webview_label: &str,
    bounds: EmbeddedWebviewBounds,
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
    webview
        .set_position(bounds.position())
        .map_err(display_error)?;
    webview.set_size(bounds.size()).map_err(display_error)?;
    webview.show().map_err(display_error)?;
    raise_webview(&webview)?;
    webview.set_focus().map_err(display_error)
}

pub(crate) fn hide(app: &AppHandle, webview_label: &str) -> Result<(), String> {
    if let Some(webview) = app.get_webview(webview_label) {
        if webview.window().label() == MAIN_WINDOW_LABEL {
            // Do not reparent a live WebView2 while handling a frontend command.
            // Tauri's Windows reparent dispatcher waits synchronously for the UI
            // event loop; moving an actively streaming provider page into the
            // hidden popout can therefore deadlock the whole desktop window.
            park(&webview)?;
        } else if let Some(popout) = app.get_window(webview_label) {
            // A page already hosted by the popout can stay there. Keeping its
            // complete viewport visible inside a hidden native host preserves the
            // provider DOM and background controller without another reparent.
            popout.hide().map_err(display_error)?;
            webview.hide().map_err(display_error)?;
            webview
                .set_position(PhysicalPosition::new(0, 0))
                .map_err(display_error)?;
            webview
                .set_size(popout.inner_size().map_err(display_error)?)
                .map_err(display_error)?;
            webview.show().map_err(display_error)?;
        } else {
            park(&webview)?;
        }
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

    #[test]
    fn parked_position_stays_beyond_the_largest_embedded_surface() {
        assert_eq!(parked_position(), (PARK_OFFSET, PARK_OFFSET));
        assert!(PARK_OFFSET > 16_384);
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
}
