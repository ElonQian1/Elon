use serde::Serialize;
use tauri::{
    webview::NewWindowResponse, AppHandle, Manager, Url, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use super::{
    display_error, ensure_main_webview, owner_fingerprint, provider, restore_window,
    ProviderDefinition, LOCAL_AI_NATIVE_WINDOW_PREFIX,
};

const NATIVE_CHAT_PATH: &str = "/pc/user-browser/native";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiNativeChatWindow {
    provider_id: &'static str,
    window_label: String,
    status: &'static str,
}

pub(super) fn open(
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
        return Ok(response(provider, label, "focused"));
    }

    let url = native_chat_url(&webview, provider)?;
    let origin = PageOrigin::from_url(&url)?;
    let navigation_origin = origin.clone();
    let popup_origin = origin;
    let window = WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(url))
        .title(format!("{} · 一龙聊天", provider.display_name))
        .inner_size(940.0, 760.0)
        .min_inner_size(720.0, 560.0)
        .center()
        .enable_clipboard_access()
        .on_navigation(move |candidate| navigation_origin.allows(candidate))
        .on_new_window(move |candidate, _features| {
            if popup_origin.allows(&candidate) {
                NewWindowResponse::Allow
            } else {
                NewWindowResponse::Deny
            }
        })
        .build()
        .map_err(display_error)?;
    restore_window(&window)?;
    Ok(response(provider, label, "created"))
}

pub(super) fn native_window_label(provider: &ProviderDefinition, fingerprint: &str) -> String {
    format!("{LOCAL_AI_NATIVE_WINDOW_PREFIX}{}-{fingerprint}", provider.id)
}

fn native_chat_url(
    webview: &WebviewWindow,
    provider: &ProviderDefinition,
) -> Result<Url, String> {
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
        url.scheme() == self.scheme
            && url.host_str().is_some_and(|host| host.eq_ignore_ascii_case(&self.host))
            && url.port_or_known_default() == self.port
            && (url.path() == NATIVE_CHAT_PATH || url.path().starts_with("/pc/assets/"))
    }
}
