//! Reading tabs: several isolated child WebViews the main workbench can show, park, pop out
//! into their own window or dock back. Hidden tabs are parked off-screen instead of hidden so
//! pages keep running and the shared read-back adapter still observes them.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::Serialize;
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder},
    AppHandle, Manager, State, Webview, WebviewUrl,
};

use crate::{
    external_navigation,
    local_ai_browser::embedded_view::{park, EmbeddedWebviewBounds},
    MAIN_WINDOW_LABEL,
};

#[path = "internal_browser_popout.rs"]
mod popout;
#[path = "internal_browser_read_preview.rs"]
mod read_preview;

const LABEL_PREFIX: &str = "internal-browser-";
const DEFAULT_TAB_ID: &str = "source";
const MAX_TABS: usize = 8;
const PROFILE_DIR: &str = "reading-tabs-profile";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InternalBrowserTabState {
    tab_id: String,
    title: String,
    current_url: String,
    current_host: String,
    loading: bool,
    loaded: bool,
    visible: bool,
    /// `main` = child of the workbench window; `popout` = its own top-level window.
    hosted: &'static str,
    last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_preview: Option<serde_json::Value>,
}

#[derive(Clone, Default)]
pub(crate) struct InternalBrowserRuntime {
    tabs: Arc<Mutex<HashMap<String, InternalBrowserTabState>>>,
}

impl InternalBrowserRuntime {
    fn replace(&self, state: InternalBrowserTabState) {
        self.tabs
            .lock()
            .unwrap()
            .insert(state.tab_id.clone(), state);
    }

    fn update(&self, tab_id: &str, update: impl FnOnce(&mut InternalBrowserTabState)) {
        if let Some(state) = self.tabs.lock().unwrap().get_mut(tab_id) {
            update(state);
        }
    }

    fn snapshot(&self, tab_id: &str) -> Result<InternalBrowserTabState, String> {
        self.tabs
            .lock()
            .unwrap()
            .get(tab_id)
            .cloned()
            .ok_or_else(|| "内部网页标签尚未打开。".to_string())
    }

    pub(crate) fn remove(&self, tab_id: &str) {
        self.tabs.lock().unwrap().remove(tab_id);
    }

    fn list(&self) -> Vec<InternalBrowserTabState> {
        let mut tabs: Vec<_> = self.tabs.lock().unwrap().values().cloned().collect();
        tabs.sort_by(|a, b| a.tab_id.cmp(&b.tab_id));
        tabs
    }

    fn len(&self) -> usize {
        self.tabs.lock().unwrap().len()
    }
}

fn tab_id_or_default(value: Option<String>) -> Result<String, String> {
    let id = value.unwrap_or_else(|| DEFAULT_TAB_ID.to_string());
    let valid = (1..=40).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    valid
        .then_some(id)
        .ok_or_else(|| "内部网页标签编号无效。".to_string())
}

pub(crate) fn webview_label(tab_id: &str) -> String {
    format!("{LABEL_PREFIX}{tab_id}")
}

pub(crate) fn tab_id_from_label(label: &str) -> Option<&str> {
    label.strip_prefix(LABEL_PREFIX)
}

#[tauri::command]
pub async fn open_internal_browser_tab(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, InternalBrowserRuntime>,
    url: String,
    title: Option<String>,
    bounds: EmbeddedWebviewBounds,
    tab_id: Option<String>,
) -> Result<InternalBrowserTabState, String> {
    ensure_main_webview(&webview)?;
    let tab_id = tab_id_or_default(tab_id)?;
    let url = parse_external_url(&url)?;
    let bounds = bounds.validate()?;
    let title = safe_title(title.as_deref(), url.host_str().unwrap_or("网页"));
    let label = webview_label(&tab_id);

    if let Some(tab) = app.get_webview(&label) {
        let hosted = runtime
            .snapshot(&tab_id)
            .map(|s| s.hosted)
            .unwrap_or("main");
        runtime.replace(state_for(&tab_id, &url, title, hosted));
        tab.navigate(url).map_err(display_error)?;
        if hosted == "popout" {
            popout::focus(&app, &label)?;
        } else {
            present(&tab, bounds)?;
        }
        return runtime.snapshot(&tab_id);
    }
    if runtime.len() >= MAX_TABS {
        return Err(format!(
            "最多同时打开 {MAX_TABS} 个阅读标签，请先关闭一些。"
        ));
    }
    runtime.replace(state_for(&tab_id, &url, title, "main"));

    let page_runtime = runtime.inner().clone();
    let page_tab = tab_id.clone();
    let title_runtime = runtime.inner().clone();
    let title_tab = tab_id.clone();
    // One persistent profile shared by reading tabs so sites needing a login stay usable.
    let profile = app
        .path()
        .app_local_data_dir()
        .map_err(display_error)?
        .join(PROFILE_DIR);
    let builder = WebviewBuilder::new(&label, WebviewUrl::External(url.clone()))
        .data_directory(profile)
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
            page_runtime.update(&page_tab, |state| {
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
            title_runtime.update(&title_tab, |state| state.title = title);
        });
    let main_window = app
        .get_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "一龙主窗口不可用。".to_string())?;
    let tab = main_window
        .add_child(builder, bounds.position(), bounds.size())
        .map_err(|error| {
            runtime.remove(&tab_id);
            display_error(error)
        })?;
    present(&tab, bounds)?;
    runtime.snapshot(&tab_id)
}

fn present(tab: &Webview, bounds: EmbeddedWebviewBounds) -> Result<(), String> {
    tab.set_position(bounds.position()).map_err(display_error)?;
    tab.set_size(bounds.size()).map_err(display_error)?;
    tab.show().map_err(display_error)?;
    raise_webview(tab)?;
    tab.set_focus().map_err(display_error)
}

#[tauri::command]
pub fn resize_internal_browser_tab(
    webview: Webview,
    app: AppHandle,
    runtime: State<'_, InternalBrowserRuntime>,
    bounds: EmbeddedWebviewBounds,
    tab_id: Option<String>,
) -> Result<(), String> {
    ensure_main_webview(&webview)?;
    let tab_id = tab_id_or_default(tab_id)?;
    let bounds = bounds.validate()?;
    let state = runtime.snapshot(&tab_id)?;
    // A popped-out or parked tab keeps its own geometry; only a visible docked tab follows.
    if state.hosted != "main" || !state.visible {
        return Ok(());
    }
    let tab = app
        .get_webview(&webview_label(&tab_id))
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
    tab_id: Option<String>,
    bounds: Option<EmbeddedWebviewBounds>,
) -> Result<Option<InternalBrowserTabState>, String> {
    ensure_main_webview(&webview)?;
    let tab_id = tab_id_or_default(tab_id)?;
    let label = webview_label(&tab_id);
    let tab = app
        .get_webview(&label)
        .ok_or_else(|| "内部网页标签尚未打开。".to_string())?;
    match action.as_str() {
        "back" => tab.eval("history.back();").map_err(display_error)?,
        "forward" => tab.eval("history.forward();").map_err(display_error)?,
        "reload" => tab.reload().map_err(display_error)?,
        "show" => {
            if runtime.snapshot(&tab_id)?.hosted == "popout" {
                popout::focus(&app, &label)?;
            } else {
                // Without bounds the caller resizes right after; the tab stays parked until then.
                match bounds {
                    Some(bounds) => present(&tab, bounds.validate()?)?,
                    None => {
                        tab.show().map_err(display_error)?;
                        raise_webview(&tab)?;
                    }
                }
                runtime.update(&tab_id, |state| state.visible = true);
            }
        }
        "hide" => {
            if runtime.snapshot(&tab_id)?.hosted == "main" {
                park(&tab)?;
                runtime.update(&tab_id, |state| state.visible = false);
            }
        }
        "popout" => {
            popout::detach(&app, &tab, &runtime.snapshot(&tab_id)?.title)?;
            runtime.update(&tab_id, |state| {
                state.hosted = "popout";
                state.visible = true;
            });
        }
        "dock" => {
            popout::attach(&app, &tab)?;
            runtime.update(&tab_id, |state| {
                state.hosted = "main";
                state.visible = false;
            });
        }
        "external" => {
            let current = runtime.snapshot(&tab_id)?.current_url;
            external_navigation::open_in_system_browser(&parse_external_url(&current)?)?;
        }
        "close" => {
            close_tab(&app, &runtime, &tab_id);
            return Ok(None);
        }
        _ => return Err("不支持的内部网页标签控制动作。".to_string()),
    }
    runtime.snapshot(&tab_id).map(Some)
}

pub(crate) fn close_tab(app: &AppHandle, runtime: &InternalBrowserRuntime, tab_id: &str) {
    let label = webview_label(tab_id);
    if let Some(tab) = app.get_webview(&label) {
        let _ = tab.close();
    }
    popout::close_window(app, &label);
    runtime.remove(tab_id);
}

#[tauri::command]
pub async fn get_internal_browser_tab_state(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, InternalBrowserRuntime>,
    tab_id: Option<String>,
    original_url: Option<String>,
) -> Result<InternalBrowserTabState, String> {
    ensure_main_webview(&webview)?;
    let tab_id = tab_id_or_default(tab_id)?;
    let mut state = runtime.snapshot(&tab_id)?;
    if let Some(original) = original_url {
        // Parked tabs are still rendered off-screen, so read-back works while hidden.
        if state.loaded && state.last_error.is_none() {
            if let Some(tab) = app.get_webview(&webview_label(&tab_id)) {
                state.read_preview = read_preview::read(&tab, &original).await;
                let current = runtime.snapshot(&tab_id)?;
                if current.current_url != state.current_url || current.loading {
                    state.read_preview = None;
                }
            }
        }
    }
    Ok(state)
}

#[tauri::command]
pub fn list_internal_browser_tabs(
    webview: Webview,
    runtime: State<'_, InternalBrowserRuntime>,
) -> Result<Vec<InternalBrowserTabState>, String> {
    ensure_main_webview(&webview)?;
    Ok(runtime.list())
}

fn parse_external_url(value: &str) -> Result<tauri::Url, String> {
    let url = value
        .parse::<tauri::Url>()
        .map_err(|_| "内部网页链接格式无效。".to_string())?;
    external_navigation::validate_external_url(&url)?;
    Ok(url)
}

fn state_for(
    tab_id: &str,
    url: &tauri::Url,
    title: String,
    hosted: &'static str,
) -> InternalBrowserTabState {
    InternalBrowserTabState {
        tab_id: tab_id.to_string(),
        title,
        current_url: url.as_str().to_string(),
        current_host: url.host_str().unwrap_or_default().to_string(),
        loading: true,
        loaded: false,
        visible: true,
        hosted,
        last_error: None,
        read_preview: None,
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

    #[test]
    fn tab_ids_default_to_source_and_reject_label_injection() {
        assert_eq!(tab_id_or_default(None).unwrap(), "source");
        assert_eq!(tab_id_or_default(Some("read-3".into())).unwrap(), "read-3");
        assert!(tab_id_or_default(Some("Main".into())).is_err());
        assert!(tab_id_or_default(Some("../main".into())).is_err());
        assert!(tab_id_or_default(Some(String::new())).is_err());
        assert_eq!(tab_id_from_label(&webview_label("read-3")), Some("read-3"));
    }

    #[test]
    fn runtime_tracks_tabs_independently() {
        let runtime = InternalBrowserRuntime::default();
        let url: tauri::Url = "https://example.com/a".parse().unwrap();
        runtime.replace(state_for("a", &url, "A".into(), "main"));
        runtime.replace(state_for("b", &url, "B".into(), "main"));
        runtime.update("a", |state| state.visible = false);
        assert!(!runtime.snapshot("a").unwrap().visible);
        assert!(runtime.snapshot("b").unwrap().visible);
        assert_eq!(runtime.list().len(), 2);
        runtime.remove("a");
        assert!(runtime.snapshot("a").is_err());
    }
}
