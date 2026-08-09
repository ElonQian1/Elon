//! 本地 AI 网页会话宿主。
//!
//! WebView2 自己持有 Cookie、DOM storage 与缓存；本模块只按一龙账号和厂商
//! 隔离 Profile、限制导航，并把官方网页中用户可见的语义转换为受限内存事件。
//! Cookie、Token、请求头、原始响应与任意 URL 始终不进入 IPC。

#[path = "local_ai_browser/adapter.rs"]
mod adapter;
#[path = "local_ai_browser/google_ai_mode.rs"]
mod google_ai_mode;
#[path = "local_ai_browser/state.rs"]
mod state;

use std::{fs, path::PathBuf, process::Command};

use serde::Serialize;
use serde_json::{Map, Value};
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent},
    AppHandle, Manager, State, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

pub use state::LocalAiBrowserRuntime;
use state::LocalAiWebSessionState;

const RENDERER_PROTOCOL: &str = "yilong.ai.ui.v1";
const PROFILE_ROOT: &str = "ai-web-profiles";
const MAIN_WEBVIEW_LABEL: &str = "main";
const LOCAL_AI_WINDOW_PREFIX: &str = "local-ai-";

#[derive(Clone, Copy)]
struct ProviderDefinition {
    id: &'static str,
    display_name: &'static str,
    start_url: &'static str,
    start_host: &'static str,
    login_mode: &'static str,
    renderer_status: &'static str,
    semantic_adapter: bool,
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
    semantic_adapter: true,
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
    semantic_adapter: true,
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
pub fn list_local_ai_web_providers(
    webview: WebviewWindow,
) -> Result<Vec<LocalAiWebProvider>, String> {
    ensure_main_webview(&webview)?;
    Ok(PROVIDERS.iter().map(provider_summary).collect())
}

#[tauri::command]
pub async fn open_local_ai_web_session(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
) -> Result<LocalAiWebSession, String> {
    ensure_main_webview(&webview)?;
    let provider = provider(&provider_id)?;
    let owner_fingerprint = owner_fingerprint(&owner_key)?;
    let window_label = window_label(provider, &owner_fingerprint);
    runtime.ensure_session(
        &window_label,
        provider.id,
        initial_renderer_status(provider),
    );

    if let Some(window) = app.get_webview_window(&window_label) {
        restore_window(&window)?;
        runtime.mark_window_status(&window_label, "ready");
        request_adapter_snapshot(provider, &window);
        return Ok(session_response(provider, window_label, "focused"));
    }

    runtime.mark_opening(&window_label);
    let start_url = parse_start_url(provider)?;
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
    let window_state = runtime.inner().clone();
    let window_state_label = window_label.clone();

    let mut builder =
        WebviewWindowBuilder::new(&app, &window_label, WebviewUrl::External(bootstrap_url))
            .title(format!("{} · 一龙本地会话", provider.display_name))
            .inner_size(1180.0, 780.0)
            .min_inner_size(900.0, 620.0)
            .center()
            .data_directory(profile_directory)
            .incognito(false)
            .enable_clipboard_access();
    if provider.semantic_adapter {
        builder = builder.initialization_script(adapter_initialization_script(provider));
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
        .on_page_load(move |_window, payload| {
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
                    page_state.mark_page_finished(&page_label, payload.url())
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
    restore_window(&window)?;
    window.navigate(start_url).map_err(|error| {
        runtime.record_error(
            &window_label,
            format!("{} 首次导航失败：{error}", provider.display_name),
        );
        display_error(error)
    })?;
    Ok(session_response(provider, window_label, "created"))
}

#[tauri::command]
pub fn get_local_ai_web_session_state(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
) -> Result<LocalAiWebSessionState, String> {
    ensure_main_webview(&webview)?;
    let provider = provider(&provider_id)?;
    let fingerprint = owner_fingerprint(&owner_key)?;
    let label = window_label(provider, &fingerprint);
    runtime.ensure_session(&label, provider.id, initial_renderer_status(provider));
    if let Some(window) = app.get_webview_window(&label) {
        if window.is_minimized().unwrap_or(false) {
            runtime.mark_window_status(&label, "minimized");
        } else if runtime
            .snapshot(&label)
            .is_some_and(|state| state.window_status == "minimized")
        {
            runtime.mark_window_status(&label, "ready");
        }
        if let Ok(url) = window.url() {
            runtime.observe_url(&label, &url);
        }
    } else {
        runtime.mark_window_status(&label, "closed");
    }
    runtime
        .snapshot(&label)
        .ok_or_else(|| format!("尚未创建 {} 本地会话。", provider.display_name))
}

#[tauri::command]
pub fn control_local_ai_web_session(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
    action: String,
) -> Result<LocalAiWebSessionState, String> {
    ensure_main_webview(&webview)?;
    let provider = provider(&provider_id)?;
    let fingerprint = owner_fingerprint(&owner_key)?;
    let label = window_label(provider, &fingerprint);
    runtime.ensure_session(&label, provider.id, initial_renderer_status(provider));
    if action == "external" {
        open_fixed_external_url(provider.start_url)?;
        return runtime
            .snapshot(&label)
            .ok_or_else(|| format!("{} 本地会话状态不可用。", provider.display_name));
    }
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("请先打开 {} 本地网页会话。", provider.display_name))?;

    match action.as_str() {
        "restore" => restore_window(&window)?,
        "reload" => window.reload().map_err(display_error)?,
        "back" => window.eval("history.back();").map_err(display_error)?,
        "home" => window
            .navigate(parse_start_url(provider)?)
            .map_err(display_error)?,
        _ => return Err("不支持的本地 AI 浏览器控制动作。".to_string()),
    }
    restore_window(&window)?;
    runtime.mark_window_status(&label, "ready");
    runtime
        .snapshot(&label)
        .ok_or_else(|| format!("{} 本地会话状态不可用。", provider.display_name))
}

#[tauri::command]
pub fn run_local_ai_web_adapter_command(
    app: AppHandle,
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
    action: String,
    value: Option<String>,
    expected_draft: Option<String>,
) -> Result<(), String> {
    ensure_main_webview(&webview)?;
    let provider = provider(&provider_id)?;
    if !provider.semantic_adapter {
        return Err(format!(
            "{} 当前使用官方网页模式，尚未启用一龙原生语义界面。",
            provider.display_name
        ));
    }
    let fingerprint = owner_fingerprint(&owner_key)?;
    let label = window_label(provider, &fingerprint);
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("请先打开 {} 官方网页。", provider.display_name))?;
    runtime.mark_command_pending(&label);
    let command = adapter_command(provider, &action, value, expected_draft)?;
    let raw = serde_json::to_string(&command).map_err(display_error)?;
    let encoded = serde_json::to_string(&raw).map_err(display_error)?;
    let bridge = adapter_bridge_name(provider);
    window
        .eval(format!(
            "window.{bridge} && window.{bridge}.command({encoded});"
        ))
        .map_err(display_error)?;
    restore_window(&window)
}

#[tauri::command]
pub fn publish_local_ai_web_event(
    webview: WebviewWindow,
    runtime: State<'_, LocalAiBrowserRuntime>,
    payload: String,
) -> Result<(), String> {
    let label = webview.label();
    let provider = provider_for_window_label(label)
        .filter(|provider| provider.semantic_adapter)
        .ok_or_else(|| "可见语义事件只允许已登记的本地 AI 会话窗口发送。".to_string())?;
    let event = if provider.id == GOOGLE_AI_MODE.id {
        google_ai_mode::sanitize_event(&payload)?
    } else {
        adapter::sanitize_event(&payload)?
    };
    runtime.record_adapter_event(label, &event.kind, event.payload);
    Ok(())
}

#[tauri::command]
pub fn clear_local_ai_web_session(
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
    window
        .navigate(parse_start_url(provider)?)
        .map_err(display_error)?;
    runtime.mark_opening(&label);
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
    }
}

fn ensure_main_webview(webview: &WebviewWindow) -> Result<(), String> {
    if webview.label() == MAIN_WEBVIEW_LABEL {
        Ok(())
    } else {
        Err("本地 AI 浏览器命令只允许一龙 PC 主窗口调用。".to_string())
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
    if window.is_minimized().unwrap_or(false) {
        window.unminimize().map_err(display_error)?;
    }
    window.show().map_err(display_error)?;
    window.set_focus().map_err(display_error)
}

fn initial_renderer_status(provider: &ProviderDefinition) -> &'static str {
    if provider.semantic_adapter {
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
    if !provider.semantic_adapter {
        return;
    }
    let bridge = adapter_bridge_name(provider);
    let _ = window.eval(format!(
        "window.{bridge} && window.{bridge}.command('{{\"action\":\"snapshot\"}}');"
    ));
}

fn adapter_initialization_script(provider: &ProviderDefinition) -> String {
    if provider.id == GOOGLE_AI_MODE.id {
        google_ai_mode::initialization_script()
    } else {
        adapter::initialization_script()
    }
}

fn adapter_bridge_name(provider: &ProviderDefinition) -> &'static str {
    if provider.id == GOOGLE_AI_MODE.id {
        "__elonGoogleAiModeBridge"
    } else {
        "__elonChatGptBridge"
    }
}

fn adapter_command(
    provider: &ProviderDefinition,
    action: &str,
    value: Option<String>,
    expected_draft: Option<String>,
) -> Result<Value, String> {
    const CHATGPT_ACTIONS: &[&str] = &[
        "snapshot",
        "send_prompt",
        "stop_generation",
        "regenerate_response",
        "new_conversation",
        "list_conversations",
        "open_conversation",
        "start_google_login",
        "list_model_options",
        "list_composer_tools",
        "collect_model_options",
        "collect_composer_tools",
        "select_model_option",
        "select_composer_tool",
    ];
    const GOOGLE_AI_MODE_ACTIONS: &[&str] = &[
        "snapshot",
        "send_prompt",
        "stop_generation",
        "new_conversation",
    ];
    let actions = if provider.id == GOOGLE_AI_MODE.id {
        GOOGLE_AI_MODE_ACTIONS
    } else {
        CHATGPT_ACTIONS
    };
    if !actions.contains(&action) {
        return Err(format!("不支持的 {} 原生界面动作。", provider.display_name));
    }
    let mut command = Map::new();
    command.insert("action".to_string(), Value::String(action.to_string()));
    if let Some(value) = value {
        if value.chars().count() > 20_000 {
            return Err(format!("{} 输入内容过长。", provider.display_name));
        }
        command.insert("value".to_string(), Value::String(value));
    }
    if let Some(expected_draft) = expected_draft {
        if expected_draft.chars().count() > 20_000 {
            return Err(format!("{} 网页草稿过长。", provider.display_name));
        }
        command.insert("expectedDraft".to_string(), Value::String(expected_draft));
    }
    Ok(Value::Object(command))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn url(value: &str) -> Url {
        value.parse().expect("test URL")
    }

    #[test]
    fn bootstrap_and_vendor_navigation_are_allowed() {
        assert!(allows_navigation(&CHATGPT, &url("about:blank")));
        assert!(allows_navigation(
            &CHATGPT,
            &url("edge-error://edgewebdata/")
        ));
        assert!(allows_navigation(&CHATGPT, &url("https://chatgpt.com/")));
        assert!(allows_navigation(
            &CHATGPT,
            &url("https://auth.openai.com/login")
        ));
        assert!(allows_navigation(
            &CHATGPT,
            &url("https://accounts.google.com/o/oauth2/v2/auth")
        ));
    }

    #[test]
    fn google_ai_mode_navigation_is_scoped_to_official_search_hosts() {
        assert!(allows_navigation(
            &GOOGLE_AI_MODE,
            &url("https://www.google.com/aimode")
        ));
        assert!(allows_navigation(
            &GOOGLE_AI_MODE,
            &url("https://www.google.com/search?udm=50&q=rust")
        ));
        assert!(!allows_navigation(
            &GOOGLE_AI_MODE,
            &url("https://accounts.google.com/v3/signin/identifier")
        ));
        assert!(!allows_navigation(
            &GOOGLE_AI_MODE,
            &url("https://mail.google.com/mail/u/0/")
        ));
        assert!(!allows_navigation(
            &GOOGLE_AI_MODE,
            &url("https://google.com.evil.example/aimode")
        ));
    }

    #[test]
    fn google_ai_mode_is_registered_with_semantic_adapter() {
        let summary = provider_summary(&GOOGLE_AI_MODE);
        assert_eq!(summary.id, "google-ai-mode");
        assert_eq!(summary.login_mode, "guest_web_system_login");
        assert_eq!(summary.renderer_status, "active");
        assert!(GOOGLE_AI_MODE.semantic_adapter);
    }

    #[test]
    fn unsafe_navigation_is_rejected() {
        assert!(!allows_navigation(&CHATGPT, &url("http://chatgpt.com/")));
        assert!(!allows_navigation(
            &CHATGPT,
            &url("https://chatgpt.com:444/")
        ));
        assert!(!allows_navigation(
            &CHATGPT,
            &url("https://chatgpt.com.evil.example/")
        ));
        assert!(!allows_navigation(
            &CHATGPT,
            &url("https://mail.google.com/")
        ));
        assert!(!allows_navigation(
            &CHATGPT,
            &url("https://user@example.com/")
        ));
    }

    #[test]
    fn owner_fingerprint_is_stable_separate_and_path_safe() {
        let first = owner_fingerprint("account-15692409892").unwrap();
        let second = owner_fingerprint("account-15692409892").unwrap();
        let other = owner_fingerprint("another-account").unwrap();
        assert_eq!(first, second);
        assert_ne!(first, other);
        assert_eq!(first.len(), 16);
        assert!(first.chars().all(|value| value.is_ascii_hexdigit()));
    }

    #[test]
    fn adapter_command_does_not_accept_arbitrary_javascript() {
        assert!(adapter_command(&CHATGPT, "eval", Some("alert(1)".to_string()), None).is_err());
        assert!(adapter_command(&CHATGPT, "snapshot", None, None).is_ok());
        assert!(
            adapter_command(&GOOGLE_AI_MODE, "send_prompt", Some("hi".to_string()), None).is_ok()
        );
        assert!(adapter_command(&GOOGLE_AI_MODE, "start_google_login", None, None).is_err());
    }
}
