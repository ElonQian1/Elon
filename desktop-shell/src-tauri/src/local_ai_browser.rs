//! 本地 AI 网页会话宿主。
//!
//! WebView2 自己持有 Cookie、DOM storage 与缓存；本模块只按一龙账号和厂商
//! 隔离 Profile、限制导航，并把官方网页中用户可见的语义转换为受限本机事件。
//! Cookie、Token、请求头、原始响应与任意 URL 始终不进入 IPC。

#[path = "local_ai_browser/adapter.rs"]
mod adapter;
#[path = "local_ai_browser/adapter_command.rs"]
mod adapter_command;
#[path = "local_ai_browser/adapter_content.rs"]
mod adapter_content;
#[path = "local_ai_browser/chatgpt_adapter_bootstrap.rs"]
mod chatgpt_adapter_bootstrap;
#[path = "local_ai_browser/google_ai_mode.rs"]
mod google_ai_mode;
#[path = "local_ai_browser/native_window.rs"]
mod native_window;
#[path = "local_ai_browser/native_window_state.rs"]
pub(crate) mod native_window_state;
#[path = "local_ai_browser/provider_adapter.rs"]
mod provider_adapter;
#[path = "local_ai_browser/semantic_context.rs"]
mod semantic_context;
#[path = "local_ai_browser/snapshot_cache.rs"]
mod snapshot_cache;
#[path = "local_ai_browser/state.rs"]
mod state;
#[cfg(test)]
#[path = "local_ai_browser/tests.rs"]
mod tests;

use std::{fs, path::PathBuf, process::Command};

use serde::Serialize;
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent},
    AppHandle, Manager, State, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

pub use native_window_state::{LocalAiNativeWindowRuntime, LocalAiNativeWindowState};
use provider_adapter::ProviderAdapter;
pub use state::LocalAiBrowserRuntime;
use state::LocalAiWebSessionState;

const RENDERER_PROTOCOL: &str = "yilong.ai.ui.v1";
const PROFILE_ROOT: &str = "ai-web-profiles";
const SNAPSHOT_CACHE_FILE: &str = "yilong-semantic-snapshot.v1.dpapi";
const MAIN_WEBVIEW_LABEL: &str = "main";
const LOCAL_AI_WINDOW_PREFIX: &str = "local-ai-";
const LOCAL_AI_NATIVE_WINDOW_PREFIX: &str = "local-ai-native-";

#[derive(Clone, Copy)]
struct ProviderDefinition {
    id: &'static str,
    display_name: &'static str,
    start_url: &'static str,
    start_host: &'static str,
    login_mode: &'static str,
    renderer_status: &'static str,
    adapter: Option<ProviderAdapter>,
    allowed_hosts: &'static [&'static str],
    allowed_domain_suffixes: &'static [&'static str],
    allowed_identity_hosts: &'static [&'static str],
    blocked_identity_hosts: &'static [&'static str],
}

const CHATGPT: ProviderDefinition = ProviderDefinition {
    id: "chatgpt",
    display_name: "ChatGPT",
    start_url: "https://chatgpt.com/",
    start_host: "chatgpt.com",
    login_mode: "manual_web",
    renderer_status: "active",
    adapter: Some(ProviderAdapter::ChatGpt),
    allowed_hosts: &[],
    allowed_domain_suffixes: &["chatgpt.com", "openai.com"],
    allowed_identity_hosts: &[
        "accounts.google.com",
        "appleid.apple.com",
        "login.live.com",
        "account.live.com",
        "login.microsoft.com",
        "login.microsoftonline.com",
        "login.windows.net",
    ],
    blocked_identity_hosts: &[],
};

const GOOGLE_AI_MODE: ProviderDefinition = ProviderDefinition {
    id: "google-ai-mode",
    display_name: "Google AI 模式",
    start_url: "https://www.google.com/aimode",
    start_host: "google.com/aimode",
    login_mode: "guest_web_system_login",
    renderer_status: "active",
    adapter: Some(ProviderAdapter::GoogleWeb),
    allowed_hosts: &["google.com", "www.google.com"],
    allowed_domain_suffixes: &[],
    allowed_identity_hosts: &[],
    blocked_identity_hosts: &["accounts.google.com"],
};

const PROVIDERS: &[ProviderDefinition] = &[GOOGLE_AI_MODE, CHATGPT];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiWebProvider {
    id: &'static str,
    display_name: &'static str,
    start_host: &'static str,
    login_mode: &'static str,
    profile_scope: &'static str,
    renderer_protocol: &'static str,
    renderer_status: &'static str,
    adapter_actions: &'static [&'static str],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiWebSession {
    provider_id: &'static str,
    window_label: String,
    status: &'static str,
    profile_scope: &'static str,
    cookie_access: &'static str,
    renderer_protocol: &'static str,
    renderer_status: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearLocalAiWebSession {
    provider_id: &'static str,
    status: &'static str,
}

#[tauri::command]
pub async fn open_local_ai_native_chat_window(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiNativeWindowRuntime>,
    provider_id: String,
    owner_key: String,
) -> Result<native_window::LocalAiNativeChatWindow, String> {
    native_window::open(
        app,
        webview,
        runtime.inner().clone(),
        provider_id,
        owner_key,
    )
    .await
}

#[tauri::command]
pub fn publish_local_ai_native_window_health(
    webview: WebviewWindow,
    runtime: State<'_, LocalAiNativeWindowRuntime>,
    report: native_window::LocalAiNativeWindowHealth,
) -> Result<(), String> {
    native_window::publish_health(webview, runtime.inner().clone(), report)
}

#[tauri::command]
pub fn list_local_ai_web_providers(
    webview: WebviewWindow,
) -> Result<Vec<LocalAiWebProvider>, String> {
    ensure_provider_list_webview(&webview)?;
    Ok(PROVIDERS.iter().map(provider_summary).collect())
}

#[tauri::command]
pub async fn open_local_ai_web_session(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
    show_window: Option<bool>,
) -> Result<LocalAiWebSession, String> {
    let provider = provider(&provider_id)?;
    let owner_fingerprint = owner_fingerprint(&owner_key)?;
    let show_window = show_window.unwrap_or(true);
    ensure_session_webview(&webview, provider, &owner_fingerprint)?;
    let window_label = window_label(provider, &owner_fingerprint);
    ensure_runtime_session(
        &app,
        runtime.inner(),
        provider,
        &owner_fingerprint,
        &window_label,
    )?;

    if let Some(window) = app.get_webview_window(&window_label) {
        if show_window {
            restore_window(&window)?;
            runtime.mark_window_visible(&window_label, true);
        }
        runtime.mark_window_status(&window_label, "ready");
        request_adapter_snapshot(provider, &window);
        return Ok(session_response(
            provider,
            window_label,
            if show_window { "focused" } else { "background" },
        ));
    }

    let cached_url = runtime.cached_restorable_url(&window_label);
    runtime.mark_opening(&window_label, show_window);
    let start_url = restorable_start_url(provider, cached_url.as_deref())?;
    let bootstrap_url = "about:blank"
        .parse()
        .map_err(|error| format!("WebView2 启动页无效：{error}"))?;
    let profile_directory = profile_directory(&app, provider, &owner_fingerprint)?;
    fs::create_dir_all(&profile_directory)
        .map_err(|error| format!("无法创建本地 AI 浏览器 Profile：{error}"))?;

    let navigation_provider = *provider;
    let navigation_state = runtime.inner().clone();
    let navigation_label = window_label.clone();
    let popup_provider = *provider;
    let popup_state = runtime.inner().clone();
    let popup_label = window_label.clone();
    let page_state = runtime.inner().clone();
    let page_label = window_label.clone();
    let page_provider = *provider;
    let window_state = runtime.inner().clone();
    let window_state_label = window_label.clone();

    let mut builder =
        WebviewWindowBuilder::new(&app, &window_label, WebviewUrl::External(bootstrap_url))
            .title(format!("{} · 一龙本地会话", provider.display_name))
            .inner_size(1180.0, 780.0)
            .min_inner_size(900.0, 620.0)
            .center()
            .visible(show_window)
            .data_directory(profile_directory)
            .incognito(false)
            .enable_clipboard_access();
    if let Some(adapter) = provider.adapter {
        builder = builder.initialization_script(adapter.initialization_script());
    }
    let window = builder
        .on_navigation(move |url| {
            let allowed = allows_navigation(&navigation_provider, url);
            let blocked_message = navigation_block_message(&navigation_provider, url);
            navigation_state.mark_navigation(
                &navigation_label,
                url,
                allowed,
                blocked_message.as_deref(),
            );
            println!(
                "[elon-desktop][local-ai] {} 导航 allowed={} -> {}",
                navigation_provider.id, allowed, url
            );
            if !allowed {
                eprintln!(
                    "[elon-desktop][local-ai] 已阻止 {} 导航到 {}",
                    navigation_provider.id, url
                );
            }
            allowed
        })
        .on_new_window(move |url, _features| {
            let allowed = allows_navigation(&popup_provider, &url);
            let blocked_message = navigation_block_message(&popup_provider, &url);
            popup_state.mark_navigation(&popup_label, &url, allowed, blocked_message.as_deref());
            if allowed {
                NewWindowResponse::Allow
            } else {
                NewWindowResponse::Deny
            }
        })
        .on_page_load(move |window, payload| {
            println!(
                "[elon-desktop][local-ai] 页面事件 {:?} -> {}",
                payload.event(),
                payload.url()
            );
            match payload.event() {
                PageLoadEvent::Started => {
                    page_state.mark_navigation(&page_label, payload.url(), true, None)
                }
                PageLoadEvent::Finished => {
                    page_state.mark_page_finished(&page_label, payload.url());
                    reconnect_adapter(&page_provider, &window);
                }
            }
        })
        .build()
        .map_err(|error| {
            runtime.record_error(
                &window_label,
                format!("无法创建 {} WebView2：{error}", provider.display_name),
            );
            display_error(error)
        })?;

    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            window_state.mark_window_status(&window_state_label, "closed");
        }
    });
    if show_window {
        restore_window(&window)?;
    }
    runtime.mark_window_visible(&window_label, show_window);
    window.navigate(start_url).map_err(|error| {
        runtime.record_error(
            &window_label,
            format!("{} 首次导航失败：{error}", provider.display_name),
        );
        display_error(error)
    })?;
    Ok(session_response(
        provider,
        window_label,
        if show_window { "created" } else { "background" },
    ))
}

#[tauri::command]
pub async fn get_local_ai_web_session_state(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
) -> Result<LocalAiWebSessionState, String> {
    let provider = provider(&provider_id)?;
    let fingerprint = owner_fingerprint(&owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = window_label(provider, &fingerprint);
    ensure_runtime_session(&app, runtime.inner(), provider, &fingerprint, &label)?;
    // 高频状态轮询只能读取宿主内存，不能向 Windows UI 线程发送同步 getter。
    // WebView2 加载或聚焦期间，url()/is_minimized() 会等待同一条消息循环，曾导致
    // 官方页和原生聊天窗一起不响应。URL 与关闭状态均由窗口事件回调持续维护。
    if app.get_webview_window(&label).is_none() {
        runtime.mark_window_status(&label, "closed");
    }
    runtime
        .snapshot(&label)
        .ok_or_else(|| format!("尚未创建 {} 本地会话。", provider.display_name))
}

#[tauri::command]
pub async fn control_local_ai_web_session(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
    action: String,
) -> Result<LocalAiWebSessionState, String> {
    let provider = provider(&provider_id)?;
    let fingerprint = owner_fingerprint(&owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = window_label(provider, &fingerprint);
    ensure_runtime_session(&app, runtime.inner(), provider, &fingerprint, &label)?;
    if action == "external" {
        open_fixed_external_url(provider.start_url)?;
        return runtime
            .snapshot(&label)
            .ok_or_else(|| format!("{} 本地会话状态不可用。", provider.display_name));
    }
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("请先打开 {} 本地网页会话。", provider.display_name))?;

    if action == "background" {
        window.hide().map_err(display_error)?;
        runtime.mark_window_visible(&label, false);
        return runtime
            .snapshot(&label)
            .ok_or_else(|| format!("{} 本地会话状态不可用。", provider.display_name));
    }

    match action.as_str() {
        "restore" => {
            restore_window(&window)?;
            runtime.mark_window_status(&label, "ready");
            runtime.mark_window_visible(&label, true);
        }
        "reload" => window.reload().map_err(display_error)?,
        "back" => window.eval("history.back();").map_err(display_error)?,
        "home" => window
            .navigate(parse_start_url(provider)?)
            .map_err(display_error)?,
        _ => return Err("不支持的本地 AI 浏览器控制动作。".to_string()),
    }
    runtime
        .snapshot(&label)
        .ok_or_else(|| format!("{} 本地会话状态不可用。", provider.display_name))
}

#[tauri::command]
pub async fn run_local_ai_web_adapter_command(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
    action: String,
    value: Option<String>,
    expected_draft: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    let provider = provider(&provider_id)?;
    let adapter = provider.adapter.ok_or_else(|| {
        format!(
            "{} 当前使用官方网页模式，尚未启用一龙原生语义界面。",
            provider.display_name
        )
    })?;
    let fingerprint = owner_fingerprint(&owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = window_label(provider, &fingerprint);
    ensure_runtime_session(&app, runtime.inner(), provider, &fingerprint, &label)?;
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("请先打开 {} 官方网页。", provider.display_name))?;
    if action != "snapshot" {
        runtime.mark_command_pending_with_value(
            &label,
            &action,
            request_id.as_deref(),
            value.as_deref(),
        );
    }
    let command = adapter_command::build(
        provider.display_name,
        adapter.supported_actions(),
        &action,
        value,
        expected_draft,
        request_id,
    )?;
    let raw = serde_json::to_string(&command).map_err(display_error)?;
    window
        .eval(adapter.page_invocation_script(&raw)?)
        .map_err(display_error)
}

#[tauri::command]
pub async fn open_local_ai_cached_conversation(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
    conversation_id: String,
) -> Result<LocalAiWebSessionState, String> {
    let provider = provider(&provider_id)?;
    let fingerprint = owner_fingerprint(&owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    if conversation_id.len() != 16 || !conversation_id.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("本机会话缓存标识无效。".to_string());
    }
    let label = window_label(provider, &fingerprint);
    ensure_runtime_session(&app, runtime.inner(), provider, &fingerprint, &label)?;
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("请先打开 {} 官方网页。", provider.display_name))?;
    let restorable_url = runtime
        .activate_cached_conversation(&label, &conversation_id)
        .ok_or_else(|| "本机会话缓存已失效，请刷新会话列表。".to_string())?;
    let url = restorable_url
        .parse::<Url>()
        .map_err(|error| format!("本机会话缓存地址无效：{error}"))?;
    if !allows_navigation(provider, &url) {
        return Err("本机会话缓存不再属于当前 AI 厂商。".to_string());
    }
    window.navigate(url).map_err(display_error)?;
    runtime
        .snapshot(&label)
        .ok_or_else(|| format!("{} 本地会话状态不可用。", provider.display_name))
}

#[tauri::command]
pub fn publish_local_ai_web_event(
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    payload: String,
) -> Result<(), String> {
    let label = webview.label();
    let provider = provider_for_window_label(label)
        .filter(|provider| provider.adapter.is_some())
        .ok_or_else(|| "可见语义事件只允许已登记的本地 AI 会话窗口发送。".to_string())?;
    let event = provider.adapter.unwrap().sanitize_event(&payload)?;
    runtime.record_adapter_event_with_context(
        label,
        &event.kind,
        event.payload,
        event.page_context_key.as_deref(),
    );
    Ok(())
}

#[tauri::command]
pub async fn clear_local_ai_web_session(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
) -> Result<ClearLocalAiWebSession, String> {
    ensure_main_webview(&webview)?;
    let provider = provider(&provider_id)?;
    let fingerprint = owner_fingerprint(&owner_key)?;
    let label = window_label(provider, &fingerprint);
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| "请先打开本地网页会话，再清除它的本地数据。".to_string())?;
    window.clear_all_browsing_data().map_err(display_error)?;
    runtime.clear_snapshots(&label);
    window
        .navigate(parse_start_url(provider)?)
        .map_err(display_error)?;
    let window_visible = runtime
        .snapshot(&label)
        .is_some_and(|state| state.window_visible);
    runtime.mark_opening(&label, window_visible);
    Ok(ClearLocalAiWebSession {
        provider_id: provider.id,
        status: "cleared",
    })
}

fn provider_summary(provider: &ProviderDefinition) -> LocalAiWebProvider {
    LocalAiWebProvider {
        id: provider.id,
        display_name: provider.display_name,
        start_host: provider.start_host,
        login_mode: provider.login_mode,
        profile_scope: "local_owner_provider",
        renderer_protocol: RENDERER_PROTOCOL,
        renderer_status: provider.renderer_status,
        adapter_actions: provider.adapter.map_or(
            &[] as &'static [&'static str],
            ProviderAdapter::supported_actions,
        ),
    }
}

fn ensure_main_webview(webview: &WebviewWindow) -> Result<(), String> {
    if webview.label() == MAIN_WEBVIEW_LABEL {
        Ok(())
    } else {
        Err("本地 AI 浏览器命令只允许一龙 PC 主窗口调用。".to_string())
    }
}

fn ensure_provider_list_webview(webview: &WebviewWindow) -> Result<(), String> {
    if webview.label() == MAIN_WEBVIEW_LABEL
        || webview.label().starts_with(LOCAL_AI_NATIVE_WINDOW_PREFIX)
    {
        Ok(())
    } else {
        Err("AI 网页厂商列表只允许一龙 PC 窗口读取。".to_string())
    }
}

fn ensure_session_webview(
    webview: &WebviewWindow,
    provider: &ProviderDefinition,
    fingerprint: &str,
) -> Result<(), String> {
    let expected_native = native_window::native_window_label(provider, fingerprint);
    if webview.label() == MAIN_WEBVIEW_LABEL || webview.label() == expected_native {
        Ok(())
    } else {
        Err("当前一龙窗口不能控制这个本地 AI 会话。".to_string())
    }
}

fn provider(provider_id: &str) -> Result<&'static ProviderDefinition, String> {
    PROVIDERS
        .iter()
        .find(|provider| provider.id == provider_id.trim())
        .ok_or_else(|| format!("不支持的本地 AI 网页厂商：{provider_id}"))
}

fn provider_for_window_label(label: &str) -> Option<&'static ProviderDefinition> {
    PROVIDERS
        .iter()
        .find(|provider| label.starts_with(&format!("{LOCAL_AI_WINDOW_PREFIX}{}-", provider.id)))
}

fn parse_start_url(provider: &ProviderDefinition) -> Result<Url, String> {
    provider
        .start_url
        .parse()
        .map_err(|error| format!("{} 入口地址无效：{error}", provider.display_name))
}

fn restorable_start_url(
    provider: &ProviderDefinition,
    cached_url: Option<&str>,
) -> Result<Url, String> {
    let fallback = parse_start_url(provider)?;
    let Some(cached) = cached_url.and_then(|value| value.parse::<Url>().ok()) else {
        return Ok(fallback);
    };
    if !allows_navigation(provider, &cached)
        || cached.query().is_some()
        || cached.fragment().is_some()
    {
        return Ok(fallback);
    }
    let host = cached.host_str().unwrap_or_default();
    let path = cached.path();
    let restorable = match provider.id {
        "chatgpt" => {
            host == "chatgpt.com"
                && (path == "/" || path.starts_with("/c/") || path.starts_with("/g/"))
        }
        "google-ai-mode" => matches!(host, "google.com" | "www.google.com") && path == "/aimode",
        _ => false,
    };
    Ok(if restorable { cached } else { fallback })
}

fn owner_fingerprint(owner_key: &str) -> Result<String, String> {
    let owner_key = owner_key.trim();
    if owner_key.is_empty()
        || owner_key.chars().count() > 128
        || owner_key.chars().any(char::is_control)
    {
        return Err("一龙账号标识无效，无法创建本地隔离 Profile。".to_string());
    }
    let hash = owner_key
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    Ok(format!("{hash:016x}"))
}

fn window_label(provider: &ProviderDefinition, fingerprint: &str) -> String {
    format!("{LOCAL_AI_WINDOW_PREFIX}{}-{fingerprint}", provider.id)
}

fn profile_directory(
    app: &AppHandle,
    provider: &ProviderDefinition,
    fingerprint: &str,
) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|root| root.join(PROFILE_ROOT).join(fingerprint).join(provider.id))
        .map_err(display_error)
}

fn snapshot_cache_path(
    app: &AppHandle,
    provider: &ProviderDefinition,
    fingerprint: &str,
) -> Result<PathBuf, String> {
    profile_directory(app, provider, fingerprint).map(|profile| profile.join(SNAPSHOT_CACHE_FILE))
}

fn ensure_runtime_session(
    app: &AppHandle,
    runtime: &LocalAiBrowserRuntime,
    provider: &ProviderDefinition,
    fingerprint: &str,
    label: &str,
) -> Result<(), String> {
    runtime.ensure_session_with_cache(
        label,
        provider.id,
        initial_renderer_status(provider),
        snapshot_cache_path(app, provider, fingerprint)?,
    );
    Ok(())
}

fn session_response(
    provider: &ProviderDefinition,
    window_label: String,
    status: &'static str,
) -> LocalAiWebSession {
    LocalAiWebSession {
        provider_id: provider.id,
        window_label,
        status,
        profile_scope: "local_owner_provider",
        cookie_access: "webview_only",
        renderer_protocol: RENDERER_PROTOCOL,
        renderer_status: provider.renderer_status,
    }
}

fn allows_navigation(provider: &ProviderDefinition, url: &Url) -> bool {
    if url.as_str() == "about:blank" {
        return true;
    }
    if url.scheme() == "edge-error" && url.host_str() == Some("edgewebdata") {
        return true;
    }
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|port| port != 443)
    {
        return false;
    }
    let Some(host) = url.host_str().map(str::to_ascii_lowercase) else {
        return false;
    };
    if provider.blocked_identity_hosts.contains(&host.as_str()) {
        return false;
    }
    provider.allowed_hosts.contains(&host.as_str())
        || provider.allowed_identity_hosts.contains(&host.as_str())
        || provider
            .allowed_domain_suffixes
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

fn restore_window(window: &WebviewWindow) -> Result<(), String> {
    // Setter 调用只投递到事件循环，不执行会阻塞等待回执的窗口 getter。
    window.unminimize().map_err(display_error)?;
    window.show().map_err(display_error)?;
    window.set_focus().map_err(display_error)
}

fn initial_renderer_status(provider: &ProviderDefinition) -> &'static str {
    if provider.adapter.is_some() {
        "connecting"
    } else {
        provider.renderer_status
    }
}

fn navigation_block_message(provider: &ProviderDefinition, url: &Url) -> Option<String> {
    if allows_navigation(provider, url) {
        return None;
    }
    if provider.id == GOOGLE_AI_MODE.id && url.host_str() == Some("accounts.google.com") {
        return Some(
            "Google 官方要求账号登录在系统浏览器完成；本地窗口不会接收或共享该登录 Cookie。请点击“系统浏览器”继续。"
                .to_string(),
        );
    }
    Some(format!(
        "页面尝试离开 {} 允许的官方网页域名，已由一龙拦截。",
        provider.display_name
    ))
}

fn request_adapter_snapshot(provider: &ProviderDefinition, window: &WebviewWindow) {
    if let Some(adapter) = provider.adapter {
        if let Ok(script) = adapter.page_invocation_script(r#"{"action":"snapshot"}"#) {
            let _ = window.eval(script);
        }
    }
}

fn reconnect_adapter(provider: &ProviderDefinition, window: &WebviewWindow) {
    if let Some(adapter) = provider.adapter {
        let _ = window.eval(adapter.initialization_script());
        request_adapter_snapshot(provider, window);
    }
}

#[cfg(windows)]
fn open_fixed_external_url(url: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("无法打开系统浏览器：{error}"))
}

#[cfg(not(windows))]
fn open_fixed_external_url(_url: &str) -> Result<(), String> {
    Err("当前平台不支持系统浏览器回退。".to_string())
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
