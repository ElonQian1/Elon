use serde::Deserialize;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, State, Webview};

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
) -> Result<LocalAiWebSessionState, String> {
    let provider = provider(&provider_id)?;
    let fingerprint = resolve_owner_fingerprint(&app, provider, &owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = window_label(provider, &fingerprint);
    ensure_runtime_session(&app, runtime.inner(), provider, &fingerprint, &label)?;
    present(&app, &label, bounds)?;
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
        if let Some(popout) = app.get_window(webview_label) {
            // A large child WebView parked just outside the main window can become
            // visible again after maximize/DPI changes or WebView2 coordinate
            // clamping. Keep the provider page alive inside its hidden native host
            // instead; present() reparents the same WebView back into the tab area.
            popout.hide().map_err(display_error)?;
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
    fn parked_position_uses_measured_size_with_a_safety_margin() {
        assert_eq!(
            parked_position(1280, 800),
            (1280 + PARK_MARGIN, 800 + PARK_MARGIN)
        );
    }

    #[test]
    fn parked_position_falls_back_to_a_fixed_offset_for_untrusted_sizes() {
        assert_eq!(
            parked_position(0, 0),
            (PARK_FALLBACK_OFFSET, PARK_FALLBACK_OFFSET)
        );
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
}
