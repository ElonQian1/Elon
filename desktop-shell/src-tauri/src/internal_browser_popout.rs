//! Moving a reading tab between the workbench window and its own top-level window.
use tauri::{AppHandle, Manager, PhysicalPosition, Webview, WindowBuilder, WindowEvent};

use super::{
    display_error, raise_webview, tab_id_from_label, InternalBrowserRuntime, MAIN_WINDOW_LABEL,
};
use crate::local_ai_browser::embedded_view::park;

fn window_label(webview_label: &str) -> String {
    format!("{webview_label}-window")
}

pub(super) fn detach(app: &AppHandle, tab: &Webview, title: &str) -> Result<(), String> {
    let webview_label = tab.label().to_string();
    let label = window_label(&webview_label);
    let window = match app.get_window(&label) {
        Some(window) => window,
        None => {
            let window = WindowBuilder::new(app, &label)
                .title(format!("{title} · 一龙阅读"))
                .inner_size(1080.0, 760.0)
                .min_inner_size(480.0, 360.0)
                .center()
                .build()
                .map_err(display_error)?;
            let handle = app.clone();
            let owned = webview_label.clone();
            let own_label = label.clone();
            window.on_window_event(move |event| match event {
                WindowEvent::CloseRequested { .. } => {
                    // Only a tab still living in this window is closed with it; a docked tab
                    // has already been moved back to the workbench.
                    let hosted_here = handle
                        .get_webview(&owned)
                        .is_some_and(|view| view.window().label() == own_label);
                    if hosted_here {
                        if let Some(tab_id) = tab_id_from_label(&owned) {
                            handle.state::<InternalBrowserRuntime>().remove(tab_id);
                        }
                        if let Some(view) = handle.get_webview(&owned) {
                            let _ = view.close();
                        }
                    }
                }
                WindowEvent::Resized(size) => {
                    if let Some(view) = handle.get_webview(&owned) {
                        if view.window().label() == own_label {
                            let _ = view.set_size(*size);
                        }
                    }
                }
                _ => {}
            });
            window
        }
    };
    tab.hide().map_err(display_error)?;
    if tab.window().label() != label {
        tab.reparent(&window).map_err(display_error)?;
    }
    tab.set_position(PhysicalPosition::new(0, 0))
        .map_err(display_error)?;
    tab.set_size(window.inner_size().map_err(display_error)?)
        .map_err(display_error)?;
    tab.show().map_err(display_error)?;
    window
        .set_title(&format!("{title} · 一龙阅读"))
        .map_err(display_error)?;
    window.show().map_err(display_error)?;
    window.set_focus().map_err(display_error)
}

pub(super) fn attach(app: &AppHandle, tab: &Webview) -> Result<(), String> {
    let main_window = app
        .get_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "一龙主窗口不可用。".to_string())?;
    tab.hide().map_err(display_error)?;
    if tab.window().label() != MAIN_WINDOW_LABEL {
        tab.reparent(&main_window).map_err(display_error)?;
    }
    // The frontend shows and positions the tab once its dock area is laid out.
    park(tab)?;
    close_window(app, tab.label());
    main_window.set_focus().map_err(display_error)
}

pub(super) fn focus(app: &AppHandle, webview_label: &str) -> Result<(), String> {
    let window = app
        .get_window(&window_label(webview_label))
        .ok_or_else(|| "阅读窗口已关闭。".to_string())?;
    window.unminimize().map_err(display_error)?;
    window.show().map_err(display_error)?;
    if let Some(tab) = app.get_webview(webview_label) {
        raise_webview(&tab)?;
    }
    window.set_focus().map_err(display_error)
}

pub(super) fn close_window(app: &AppHandle, webview_label: &str) {
    if let Some(window) = app.get_window(&window_label(webview_label)) {
        let _ = window.close();
    }
}
