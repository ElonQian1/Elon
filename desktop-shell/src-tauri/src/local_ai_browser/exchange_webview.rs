use serde::Serialize;
use tauri::{AppHandle, Manager, State, Webview};

use super::{
    ensure_main_webview, open_web_session, provider_for_kind, providers_for_kind,
    resolve_owner_fingerprint, window_label, LocalAiBrowserRuntime, ProviderKind,
    DESKTOP_RUNTIME_VERSION,
};

const PROVIDER_SCHEMA: &str = "yilong.exchange_webview.provider.v1";
const SESSION_SCHEMA: &str = "yilong.exchange_webview.session.v1";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExchangeWebProvider {
    schema: &'static str,
    provider_id: &'static str,
    display_name: &'static str,
    start_host: &'static str,
    login_mode: &'static str,
    profile_scope: &'static str,
    desktop_runtime_version: u32,
    background_open_supported: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExchangeWebSession {
    schema: &'static str,
    provider_id: &'static str,
    window_label: String,
    status: &'static str,
    profile_scope: &'static str,
    cookie_access: &'static str,
}

#[tauri::command]
pub(crate) fn list_exchange_web_providers(
    webview: Webview,
) -> Result<Vec<ExchangeWebProvider>, String> {
    ensure_main_webview(&webview)?;
    Ok(providers_for_kind(ProviderKind::Exchange)
        .map(|provider| ExchangeWebProvider {
            schema: PROVIDER_SCHEMA,
            provider_id: provider.id,
            display_name: provider.display_name,
            start_host: provider.start_host,
            login_mode: provider.login_mode,
            profile_scope: "local_owner_provider",
            desktop_runtime_version: DESKTOP_RUNTIME_VERSION,
            background_open_supported: true,
        })
        .collect())
}

#[tauri::command]
pub(crate) async fn open_exchange_web_session(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, LocalAiBrowserRuntime>,
    provider_id: String,
    owner_key: String,
    show_window: Option<bool>,
) -> Result<ExchangeWebSession, String> {
    let provider = provider_for_kind(&provider_id, ProviderKind::Exchange)?;
    let session = open_web_session(
        app,
        webview,
        runtime,
        provider,
        owner_key,
        show_window.unwrap_or(true),
    )
    .await?;
    Ok(ExchangeWebSession {
        schema: SESSION_SCHEMA,
        provider_id: session.provider_id,
        window_label: session.window_label,
        status: session.status,
        profile_scope: session.profile_scope,
        cookie_access: session.cookie_access,
    })
}

/// Research on a site that is also an exchange provider rides the user's own login window.
/// Returns the webview label once it exists; an already open window is not refocused.
/// `None` means the site has no exchange provider and research should use its own window.
pub(crate) async fn ensure_exchange_session_for_site(
    app: &AppHandle,
    webview: &Webview,
    runtime: State<'_, LocalAiBrowserRuntime>,
    site_id: &str,
    owner_key: &str,
) -> Option<String> {
    let provider = provider_for_kind(site_id, ProviderKind::Exchange).ok()?;
    let fingerprint = resolve_owner_fingerprint(app, provider, owner_key).ok()?;
    let label = window_label(provider, &fingerprint);
    if app.get_webview(&label).is_some() {
        return Some(label);
    }
    open_web_session(
        app.clone(),
        webview.clone(),
        runtime,
        provider,
        owner_key.to_string(),
        true,
    )
    .await
    .ok()
    .map(|session| session.window_label)
}
