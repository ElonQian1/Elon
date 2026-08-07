//! 本地 AI 网页会话宿主。
//!
//! WebView2 自己持有 Cookie、DOM storage 与缓存；本模块只负责按一龙账号和厂商
//! 划分本地 Profile、限制顶层导航并管理窗口生命周期。这里故意不提供 Cookie
//! 枚举、Token 导出或任意 URL 打开能力，未来原生渲染器只能接收去凭证化的语义事件。

use std::{fs, path::PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const RENDERER_PROTOCOL: &str = "yilong.ai.ui.v1";
const PROFILE_ROOT: &str = "ai-web-profiles";
const MAIN_WEBVIEW_LABEL: &str = "main";

#[derive(Clone, Copy)]
struct ProviderDefinition {
    id: &'static str,
    display_name: &'static str,
    start_url: &'static str,
    allowed_domain_suffixes: &'static [&'static str],
    allowed_identity_hosts: &'static [&'static str],
}

const CHATGPT: ProviderDefinition = ProviderDefinition {
    id: "chatgpt",
    display_name: "ChatGPT",
    start_url: "https://chatgpt.com/",
    allowed_domain_suffixes: &["chatgpt.com", "openai.com"],
    // 只允许厂商登录流程已知的精确主机，不允许 *.google.com 等宽泛域名。
    // 身份提供商仍可拒绝嵌入式浏览器；本应用不规避其判断或真人验证。
    allowed_identity_hosts: &[
        "accounts.google.com",
        "appleid.apple.com",
        "login.live.com",
        "login.microsoftonline.com",
    ],
};

const PROVIDERS: &[ProviderDefinition] = &[CHATGPT];

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
pub fn open_local_ai_web_session(
    app: AppHandle,
    webview: WebviewWindow,
    provider_id: String,
    owner_key: String,
) -> Result<LocalAiWebSession, String> {
    ensure_main_webview(&webview)?;
    let provider = provider(&provider_id)?;
    let owner_fingerprint = owner_fingerprint(&owner_key)?;
    let window_label = window_label(provider, &owner_fingerprint);

    if let Some(window) = app.get_webview_window(&window_label) {
        window.show().map_err(display_error)?;
        window.set_focus().map_err(display_error)?;
        return Ok(session_response(provider, window_label, "focused"));
    }

    let start_url = parse_start_url(provider)?;
    let profile_directory = profile_directory(&app, provider, &owner_fingerprint)?;
    fs::create_dir_all(&profile_directory)
        .map_err(|error| format!("无法创建本地 AI 浏览器 Profile：{error}"))?;

    let navigation_provider = *provider;
    let page_provider_id = provider.id;
    WebviewWindowBuilder::new(&app, &window_label, WebviewUrl::External(start_url))
        .title(format!("{} · 一龙本地会话", provider.display_name))
        .inner_size(1180.0, 780.0)
        .min_inner_size(900.0, 620.0)
        .center()
        .data_directory(profile_directory)
        .incognito(false)
        .enable_clipboard_access()
        // 不注入初始化脚本；官方网页窗口不获得语义桥、Cookie API 或一龙业务状态。
        .on_navigation(move |url| {
            let allowed = allows_navigation(&navigation_provider, url);
            if !allowed {
                eprintln!(
                    "[elon-desktop][local-ai] 已阻止 {} 导航到 {}",
                    navigation_provider.id, url
                );
            }
            allowed
        })
        .on_page_load(move |_window, payload| {
            println!(
                "[elon-desktop][local-ai] {} 页面事件 {:?} -> {}",
                page_provider_id,
                payload.event(),
                payload.url()
            );
        })
        .build()
        .map_err(display_error)?;

    Ok(session_response(provider, window_label, "created"))
}

#[tauri::command]
pub fn clear_local_ai_web_session(
    app: AppHandle,
    webview: WebviewWindow,
    provider_id: String,
    owner_key: String,
) -> Result<ClearLocalAiWebSession, String> {
    ensure_main_webview(&webview)?;
    let provider = provider(&provider_id)?;
    let owner_fingerprint = owner_fingerprint(&owner_key)?;
    let label = window_label(provider, &owner_fingerprint);
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| "请先打开本地网页会话，再清除它的本地数据。".to_string())?;

    window.clear_all_browsing_data().map_err(display_error)?;
    window
        .navigate(parse_start_url(provider)?)
        .map_err(display_error)?;

    Ok(ClearLocalAiWebSession {
        provider_id: provider.id,
        status: "cleared",
    })
}

fn provider_summary(provider: &ProviderDefinition) -> LocalAiWebProvider {
    LocalAiWebProvider {
        id: provider.id,
        display_name: provider.display_name,
        start_host: provider.allowed_domain_suffixes[0],
        login_mode: "manual_web",
        profile_scope: "local_owner_provider",
        renderer_protocol: RENDERER_PROTOCOL,
        renderer_status: "reserved",
    }
}

fn ensure_main_webview(webview: &WebviewWindow) -> Result<(), String> {
    if caller_label_allowed(webview.label()) {
        Ok(())
    } else {
        Err("本地 AI 浏览器命令只允许一龙 PC 主窗口调用。".to_string())
    }
}

fn caller_label_allowed(label: &str) -> bool {
    label == MAIN_WEBVIEW_LABEL
}

fn provider(provider_id: &str) -> Result<&'static ProviderDefinition, String> {
    PROVIDERS
        .iter()
        .find(|provider| provider.id == provider_id.trim())
        .ok_or_else(|| format!("不支持的本地 AI 网页厂商：{provider_id}"))
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

    // 固定 FNV-1a 仅用于不可逆目录/窗口命名，不用于密码学或鉴权。
    let hash = owner_key
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    Ok(format!("{hash:016x}"))
}

fn window_label(provider: &ProviderDefinition, owner_fingerprint: &str) -> String {
    format!("local-ai-{}-{owner_fingerprint}", provider.id)
}

fn profile_directory(
    app: &AppHandle,
    provider: &ProviderDefinition,
    owner_fingerprint: &str,
) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|root| {
            root.join(PROFILE_ROOT)
                .join(owner_fingerprint)
                .join(provider.id)
        })
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
        renderer_status: "reserved",
    }
}

fn allows_navigation(provider: &ProviderDefinition, url: &Url) -> bool {
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
    provider.allowed_identity_hosts.contains(&host.as_str())
        || provider
            .allowed_domain_suffixes
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
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
    fn chatgpt_navigation_only_allows_https_vendor_and_exact_identity_hosts() {
        assert!(allows_navigation(&CHATGPT, &url("https://chatgpt.com/")));
        assert!(allows_navigation(
            &CHATGPT,
            &url("https://auth.openai.com/login")
        ));
        assert!(allows_navigation(
            &CHATGPT,
            &url("https://accounts.google.com/o/oauth2/v2/auth")
        ));
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
        let first = owner_fingerprint("account-15692409892").expect("fingerprint");
        let second = owner_fingerprint("account-15692409892").expect("fingerprint");
        let other = owner_fingerprint("another-account").expect("fingerprint");
        assert_eq!(first, second);
        assert_ne!(first, other);
        assert_eq!(first.len(), 16);
        assert!(first.chars().all(|value| value.is_ascii_hexdigit()));
    }

    #[test]
    fn provider_contract_keeps_renderer_bridge_reserved() {
        let summary = provider_summary(&CHATGPT);
        assert_eq!(summary.id, "chatgpt");
        assert_eq!(summary.profile_scope, "local_owner_provider");
        assert_eq!(summary.renderer_protocol, "yilong.ai.ui.v1");
        assert_eq!(summary.renderer_status, "reserved");
    }

    #[test]
    fn only_main_webview_may_request_local_ai_sessions() {
        assert!(caller_label_allowed("main"));
        assert!(!caller_label_allowed("local-ai-chatgpt-deadbeef"));
        assert!(!caller_label_allowed("main-shadow"));
    }
}
