use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent},
    AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use crate::codex_semantic_bridge;

use super::{
    display_error, ensure_main_webview, owner_fingerprint, provider, restore_window,
    ProviderDefinition, LOCAL_AI_NATIVE_WINDOW_PREFIX,
};

const NATIVE_CHAT_PATH: &str = "/pc/user-browser/native";
const NAVIGATION_ERROR_SCRIPT: &str = r#"
(function () {
  var root = document.getElementById('__elon_native_window_status__');
  if (root) root.remove();
  document.documentElement.innerHTML = '<head><meta charset="utf-8"><title>一龙聊天窗诊断</title></head>'
    + '<body style="margin:0;background:#080b0a;color:#e9f7f1;display:grid;place-items:center;min-height:100vh;font-family:Segoe UI,Microsoft YaHei,sans-serif">'
    + '<section style="width:min(520px,calc(100vw - 48px));border:1px solid #7a3c45;border-radius:16px;padding:28px;background:#181113">'
    + '<small style="color:#ff8695;letter-spacing:.12em">ELON-NATIVE-NAVIGATION-FAILED</small>'
    + '<h1 style="font-size:22px;margin:12px 0 8px">一龙聊天页面导航失败</h1>'
    + '<p style="color:#c8afb3;line-height:1.7;margin:0">窗口已保留，没有闪退。请在 Codex 控制台查看 native_window.navigation_failed 事件，然后关闭本窗口重试。</p>'
    + '</section></body>';
})();
"#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiNativeChatWindow {
    provider_id: &'static str,
    window_label: String,
    status: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalAiNativeWindowHealth {
    phase: String,
    ready_state: String,
    root_exists: bool,
    root_child_count: u32,
    route: String,
}

pub(super) fn publish_health(
    webview: WebviewWindow,
    report: LocalAiNativeWindowHealth,
) -> Result<(), String> {
    if !webview.label().starts_with(LOCAL_AI_NATIVE_WINDOW_PREFIX) {
        return Err("子窗口健康状态只允许一龙原生 AI 窗口上报。".to_string());
    }
    if report.route != NATIVE_CHAT_PATH
        || !matches!(
            report.phase.as_str(),
            "script_started"
                | "dom_content_loaded"
                | "load"
                | "settled"
                | "window_error"
                | "promise_rejection"
        )
        || !matches!(
            report.ready_state.as_str(),
            "loading" | "interactive" | "complete"
        )
        || report.root_child_count > 10_000
    {
        return Err("子窗口健康状态格式无效。".to_string());
    }
    let is_error = matches!(report.phase.as_str(), "window_error" | "promise_rejection")
        || (report.phase == "settled" && (!report.root_exists || report.root_child_count == 0));
    codex_semantic_bridge::record_app_event(
        webview.app_handle(),
        webview.label(),
        if is_error { "error" } else { "debug" },
        "native_window.page_health",
        if is_error {
            "一龙 AI 子窗口页面未正常就绪"
        } else {
            "一龙 AI 子窗口页面健康状态已更新"
        },
        json!({
            "window_label": webview.label(),
            "phase": report.phase,
            "ready_state": report.ready_state,
            "root_exists": report.root_exists,
            "root_child_count": report.root_child_count,
            "route": report.route,
        }),
    );
    Ok(())
}

pub(super) async fn open(
    app: AppHandle,
    webview: WebviewWindow,
    provider_id: String,
    owner_key: String,
) -> Result<LocalAiNativeChatWindow, String> {
    ensure_main_webview(&webview)?;
    let provider = provider(&provider_id)?;
    let fingerprint = owner_fingerprint(&owner_key)?;
    let label = native_window_label(provider, &fingerprint);

    if let Some(window) = app.get_webview_window(&label) {
        restore_window(&window)?;
        record(
            &app,
            &label,
            "info",
            "native_window.focused",
            "一龙 AI 子窗口已恢复",
            json!({}),
        );
        return Ok(response(provider, label, "focused"));
    }

    let url = native_chat_url(&webview, provider)?;
    let bootstrap_url = "about:blank"
        .parse()
        .map_err(|error| format!("一龙聊天窗启动页无效：{error}"))?;
    let origin = PageOrigin::from_url(&url)?;
    let navigation_origin = origin.clone();
    let popup_origin = origin;
    let navigation_app = app.clone();
    let navigation_label = label.clone();
    let page_app = app.clone();
    let page_label = label.clone();
    let target_url = url.clone();
    let event_app = app.clone();
    let event_label = label.clone();
    record(
        &app,
        &label,
        "info",
        "native_window.creating",
        "正在创建一龙 AI 子窗口",
        json!({
        "target": safe_url(&url),
        }),
    );
    let window = WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(bootstrap_url))
        .title(format!("{} · 一龙聊天", provider.display_name))
        .inner_size(940.0, 760.0)
        .min_inner_size(720.0, 560.0)
        .center()
        .parent(&webview)
        .map_err(display_error)?
        .focused(true)
        .enable_clipboard_access()
        .initialization_script(include_str!("native_window_probe.js"))
        .on_navigation(move |candidate| {
            let allowed = navigation_origin.allows(candidate);
            record(
                &navigation_app,
                &navigation_label,
                if allowed { "debug" } else { "warn" },
                if allowed {
                    "native_window.navigation_allowed"
                } else {
                    "native_window.navigation_blocked"
                },
                if allowed {
                    "一龙 AI 子窗口导航已允许"
                } else {
                    "一龙 AI 子窗口导航已阻止"
                },
                json!({"url": safe_url(candidate)}),
            );
            allowed
        })
        .on_new_window(move |candidate, _features| {
            if popup_origin.allows(&candidate) {
                NewWindowResponse::Allow
            } else {
                NewWindowResponse::Deny
            }
        })
        .on_page_load(move |window, payload| {
            let page_url = payload.url();
            record(
                &page_app,
                &page_label,
                "debug",
                match payload.event() {
                    PageLoadEvent::Started => "native_window.page_started",
                    PageLoadEvent::Finished => "native_window.page_finished",
                },
                "一龙 AI 子窗口页面事件",
                json!({"url": safe_url(page_url)}),
            );
            if matches!(payload.event(), PageLoadEvent::Finished)
                && page_url.as_str() == "about:blank"
            {
                match window.navigate(target_url.clone()) {
                    Ok(()) => record(
                        &page_app,
                        &page_label,
                        "info",
                        "native_window.navigation_dispatched",
                        "一龙 AI 子窗口已开始导航",
                        json!({"url": safe_url(&target_url)}),
                    ),
                    Err(_error) => {
                        record(
                            &page_app,
                            &page_label,
                            "error",
                            "native_window.navigation_failed",
                            "一龙 AI 子窗口导航失败，窗口已保留",
                            json!({"error_code": "host_navigation_failed"}),
                        );
                        show_navigation_error(&window);
                    }
                }
            } else if matches!(payload.event(), PageLoadEvent::Finished)
                && page_url.scheme() == "edge-error"
            {
                record(
                    &page_app,
                    &page_label,
                    "error",
                    "native_window.navigation_failed",
                    "一龙 AI 子窗口加载失败，窗口已保留",
                    json!({"error_code": "webview_navigation_error"}),
                );
                show_navigation_error(&window);
            }
        })
        .build()
        .map_err(|error| {
            record(
                &app,
                &label,
                "error",
                "native_window.create_failed",
                "一龙 AI 子窗口创建失败",
                json!({"error_code": "webview_create_failed"}),
            );
            display_error(error)
        })?;
    window.on_window_event(move |event| match event {
        WindowEvent::Focused(focused) => {
            record(
                &event_app,
                &event_label,
                "debug",
                "native_window.focus_changed",
                if *focused {
                    "一龙 AI 子窗口已获得焦点"
                } else {
                    "一龙 AI 子窗口已失去焦点"
                },
                json!({"focused": focused}),
            );
        }
        WindowEvent::CloseRequested { .. } => record(
            &event_app,
            &event_label,
            "info",
            "native_window.close_requested",
            "一龙 AI 子窗口收到关闭请求",
            json!({}),
        ),
        WindowEvent::Destroyed => record(
            &event_app,
            &event_label,
            "info",
            "native_window.destroyed",
            "一龙 AI 子窗口已关闭",
            json!({}),
        ),
        _ => {}
    });
    restore_window(&window)?;
    record(
        &app,
        &label,
        "info",
        "native_window.created",
        "一龙 AI 子窗口已创建",
        json!({
            "parent_label": webview.label(),
            "window_role": "managed_child",
        }),
    );
    Ok(response(provider, label, "created"))
}

fn record(
    app: &AppHandle,
    label: &str,
    level: &str,
    kind: &str,
    summary: &str,
    fields: serde_json::Value,
) {
    codex_semantic_bridge::record_app_event(app, label, level, kind, summary, fields);
}

fn safe_url(url: &Url) -> String {
    if url.as_str() == "about:blank" || url.scheme() == "edge-error" {
        return format!("{}:{}", url.scheme(), url.path());
    }
    format!("{}{}", url.origin().ascii_serialization(), url.path())
}

fn show_navigation_error(window: &WebviewWindow) {
    let _ = window.eval(NAVIGATION_ERROR_SCRIPT);
}

pub(super) fn native_window_label(provider: &ProviderDefinition, fingerprint: &str) -> String {
    format!(
        "{LOCAL_AI_NATIVE_WINDOW_PREFIX}{}-{fingerprint}",
        provider.id
    )
}

fn native_chat_url(webview: &WebviewWindow, provider: &ProviderDefinition) -> Result<Url, String> {
    let mut url = webview.url().map_err(display_error)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("当前一龙页面地址不能创建原生聊天窗口。".to_string());
    }
    url.set_path(NATIVE_CHAT_PATH);
    url.set_query(None);
    url.set_fragment(None);
    url.query_pairs_mut().append_pair("provider", provider.id);
    Ok(url)
}

fn response(
    provider: &ProviderDefinition,
    window_label: String,
    status: &'static str,
) -> LocalAiNativeChatWindow {
    LocalAiNativeChatWindow {
        provider_id: provider.id,
        window_label,
        status,
    }
}

#[derive(Clone)]
struct PageOrigin {
    scheme: String,
    host: String,
    port: Option<u16>,
}

impl PageOrigin {
    fn from_url(url: &Url) -> Result<Self, String> {
        Ok(Self {
            scheme: url.scheme().to_string(),
            host: url
                .host_str()
                .ok_or_else(|| "一龙页面缺少主机名。".to_string())?
                .to_ascii_lowercase(),
            port: url.port_or_known_default(),
        })
    }

    fn allows(&self, url: &Url) -> bool {
        url.as_str() == "about:blank"
            || url.scheme() == "edge-error"
            || (url.scheme() == self.scheme
                && url
                    .host_str()
                    .is_some_and(|host| host.eq_ignore_ascii_case(&self.host))
                && url.port_or_known_default() == self.port
                && (url.path() == NATIVE_CHAT_PATH || url.path().starts_with("/pc/assets/")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_page_origin_allows_only_bootstrap_target_assets_and_webview_errors() {
        let target: Url = "http://127.0.0.1:7799/pc/user-browser/native?provider=chatgpt"
            .parse()
            .unwrap();
        let origin = PageOrigin::from_url(&target).unwrap();
        assert!(origin.allows(&"about:blank".parse().unwrap()));
        assert!(origin.allows(&target));
        assert!(origin.allows(&"http://127.0.0.1:7799/pc/assets/app.js".parse().unwrap()));
        assert!(origin.allows(&"edge-error://edgewebdata/".parse().unwrap()));
        assert!(!origin.allows(&"http://127.0.0.1:7799/pc/account".parse().unwrap()));
        assert!(!origin.allows(
            &"https://example.com/pc/user-browser/native"
                .parse()
                .unwrap()
        ));
    }

    #[test]
    fn diagnostic_urls_never_include_query_or_fragment() {
        let url: Url = "http://127.0.0.1:7799/pc/user-browser/native?provider=chatgpt#secret"
            .parse()
            .unwrap();
        assert_eq!(
            safe_url(&url),
            "http://127.0.0.1:7799/pc/user-browser/native"
        );
    }
}
