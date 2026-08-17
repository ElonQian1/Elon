use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder},
    AppHandle, Manager, State, Webview, WebviewUrl,
};

use crate::{
    external_navigation, local_ai_browser::embedded_view::EmbeddedWebviewBounds, MAIN_WINDOW_LABEL,
};

const INTERNAL_BROWSER_LABEL: &str = "internal-browser-source";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InternalBrowserTabState {
    tab_id: &'static str,
    title: String,
    current_url: String,
    current_host: String,
    loading: bool,
    loaded: bool,
    visible: bool,
    last_error: Option<String>,
}

#[derive(Clone, Default)]
pub(crate) struct InternalBrowserRuntime {
    state: Arc<Mutex<Option<InternalBrowserTabState>>>,
}

impl InternalBrowserRuntime {
    fn replace(&self, state: InternalBrowserTabState) {
        *self.state.lock().unwrap() = Some(state);
    }

    fn update(&self, update: impl FnOnce(&mut InternalBrowserTabState)) {
        if let Some(state) = self.state.lock().unwrap().as_mut() {
            update(state);
        }
    }

    fn snapshot(&self) -> Result<InternalBrowserTabState, String> {
        self.state
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "内部网页标签尚未打开。".to_string())
    }

    fn clear(&self) {
        *self.state.lock().unwrap() = None;
    }
}

#[tauri::command]
pub async fn open_internal_browser_tab(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, InternalBrowserRuntime>,
    url: String,
    title: Option<String>,
    bounds: EmbeddedWebviewBounds,
) -> Result<InternalBrowserTabState, String> {
    ensure_main_webview(&webview)?;
    let url = parse_external_url(&url)?;
    let bounds = bounds.validate()?;
    let title = safe_title(title.as_deref(), url.host_str().unwrap_or("网页"));
    runtime.replace(state_for(&url, title, true, true));

    if let Some(tab) = app.get_webview(INTERNAL_BROWSER_LABEL) {
        tab.set_position(bounds.position()).map_err(display_error)?;
        tab.set_size(bounds.size()).map_err(display_error)?;
        tab.navigate(url).map_err(display_error)?;
        tab.show().map_err(display_error)?;
        raise_webview(&tab)?;
        tab.set_focus().map_err(display_error)?;
        return runtime.snapshot();
    }

    let navigation_runtime = runtime.inner().clone();
    let page_runtime = runtime.inner().clone();
    let title_runtime = runtime.inner().clone();
    let builder = WebviewBuilder::new(INTERNAL_BROWSER_LABEL, WebviewUrl::External(url.clone()))
        .incognito(true)
        .enable_clipboard_access()
        .on_navigation(|next| external_navigation::validate_external_url(next).is_ok())
        .on_new_window(|next, _features| {
            let _ = external_navigation::open_in_system_browser(&next);
            NewWindowResponse::Deny
        })
        .on_page_load(move |_webview, payload| {
            let current_url = payload.url().as_str().to_string();
            let current_host = payload.url().host_str().unwrap_or_default().to_string();
            let loading = payload.event() == PageLoadEvent::Started;
            let loaded = payload.event() == PageLoadEvent::Finished;
            let failed = payload.url().scheme() == "edge-error";
            let last_error = failed.then(|| "页面加载失败，建议改用系统浏览器。".to_string());
            page_runtime.update(|state| {
                if !failed {
                    state.current_url = current_url;
                    state.current_host = current_host;
                }
                state.loading = loading;
                state.loaded = loaded;
                state.last_error = last_error;
            });
        })
        .on_document_title_changed(move |_webview, title| {
            let title = safe_title(Some(&title), "网页");
            title_runtime.update(|state| state.title = title);
        });
    let main_window = app
        .get_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "一龙主窗口不可用。".to_string())?;
    let tab = main_window
        .add_child(builder, bounds.position(), bounds.size())
        .map_err(display_error)?;
    navigation_runtime.update(|state| state.visible = true);
    tab.show().map_err(display_error)?;
    raise_webview(&tab)?;
    tab.set_focus().map_err(display_error)?;
    runtime.snapshot()
}

#[tauri::command]
pub fn resize_internal_browser_tab(
    webview: Webview,
    app: AppHandle,
    bounds: EmbeddedWebviewBounds,
) -> Result<(), String> {
    ensure_main_webview(&webview)?;
    let bounds = bounds.validate()?;
    let tab = app
        .get_webview(INTERNAL_BROWSER_LABEL)
        .ok_or_else(|| "内部网页标签尚未打开。".to_string())?;
    tab.set_position(bounds.position()).map_err(display_error)?;
    tab.set_size(bounds.size()).map_err(display_error)
}

#[tauri::command]
pub async fn control_internal_browser_tab(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, InternalBrowserRuntime>,
    action: String,
) -> Result<Option<InternalBrowserTabState>, String> {
    ensure_main_webview(&webview)?;
    let tab = app
        .get_webview(INTERNAL_BROWSER_LABEL)
        .ok_or_else(|| "内部网页标签尚未打开。".to_string())?;
    match action.as_str() {
        "back" => tab.eval("history.back();").map_err(display_error)?,
        "forward" => tab.eval("history.forward();").map_err(display_error)?,
        "reload" => tab.reload().map_err(display_error)?,
        "show" => {
            tab.show().map_err(display_error)?;
            raise_webview(&tab)?;
            tab.set_focus().map_err(display_error)?;
            runtime.update(|state| state.visible = true);
        }
        "hide" => {
            tab.hide().map_err(display_error)?;
            runtime.update(|state| state.visible = false);
        }
        "external" => {
            let current = runtime.snapshot()?.current_url;
            external_navigation::open_in_system_browser(&parse_external_url(&current)?)?;
        }
        "close" => {
            tab.close().map_err(display_error)?;
            runtime.clear();
            return Ok(None);
        }
        _ => return Err("不支持的内部网页标签控制动作。".to_string()),
    }
    runtime.snapshot().map(Some)
}

#[tauri::command]
pub fn get_internal_browser_tab_state(
    webview: Webview,
    runtime: State<'_, InternalBrowserRuntime>,
) -> Result<InternalBrowserTabState, String> {
    ensure_main_webview(&webview)?;
    runtime.snapshot()
}

fn parse_external_url(value: &str) -> Result<tauri::Url, String> {
    let url = value
        .parse::<tauri::Url>()
        .map_err(|_| "内部网页链接格式无效。".to_string())?;
    external_navigation::validate_external_url(&url)?;
    Ok(url)
}

fn state_for(
    url: &tauri::Url,
    title: String,
    loading: bool,
    visible: bool,
) -> InternalBrowserTabState {
    InternalBrowserTabState {
        tab_id: "source",
        title,
        current_url: url.as_str().to_string(),
        current_host: url.host_str().unwrap_or_default().to_string(),
        loading,
        loaded: false,
        visible,
        last_error: None,
    }
}

#[cfg(windows)]
pub(crate) fn raise_webview(webview: &Webview) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOP, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER,
        SWP_NOSIZE,
    };

    webview
        .with_webview(|platform| unsafe {
            let controller = platform.controller();
            let mut host = Default::default();
            if controller.ParentWindow(&mut host).is_ok() {
                let _ = SetWindowPos(
                    host.0 as _,
                    HWND_TOP,
                    0,
                    0,
                    0,
                    0,
                    SWP_ASYNCWINDOWPOS
                        | SWP_NOACTIVATE
                        | SWP_NOMOVE
                        | SWP_NOOWNERZORDER
                        | SWP_NOSIZE,
                );
            }
        })
        .map_err(display_error)
}

#[cfg(not(windows))]
pub(crate) fn raise_webview(_webview: &Webview) -> Result<(), String> {
    Ok(())
}

fn safe_title(value: Option<&str>, fallback: &str) -> String {
    let title = value
        .unwrap_or_default()
        .chars()
        .filter(|character| !character.is_control())
        .take(120)
        .collect::<String>()
        .trim()
        .to_string();
    if title.is_empty() {
        fallback.to_string()
    } else {
        title
    }
}

fn ensure_main_webview(webview: &Webview) -> Result<(), String> {
    if webview.label() == MAIN_WINDOW_LABEL {
        Ok(())
    } else {
        Err("内部网页标签只允许一龙主窗口控制。".to_string())
    }
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_tabs_only_accept_safe_https_urls() {
        assert!(parse_external_url("https://example.com/weather?q=taipei").is_ok());
        assert!(parse_external_url("http://example.com/").is_err());
        assert!(parse_external_url("file:///C:/Windows/win.ini").is_err());
        assert!(parse_external_url("https://user:secret@example.com/").is_err());
    }

    #[test]
    fn tab_titles_are_bounded_and_control_characters_are_removed() {
        assert_eq!(safe_title(Some(" Weather\nToday "), "网页"), "WeatherToday");
        assert_eq!(safe_title(Some(""), "example.com"), "example.com");
        assert_eq!(safe_title(Some(&"x".repeat(200)), "网页").len(), 120);
    }
}
