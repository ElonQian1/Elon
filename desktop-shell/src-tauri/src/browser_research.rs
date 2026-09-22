//! The research domain is separate from chat adapters and never executes financial actions.
mod files;
pub(crate) mod host;
mod ingest;
mod ingest_queue;
mod model;
mod privacy;
mod query;
mod runtime;
#[cfg(test)]
mod tests;

use model::ResearchCommand;
pub(crate) use runtime::ResearchRuntime;
use serde_json::Value;
use tauri::{AppHandle, State, Webview};

#[tauri::command]
pub(crate) fn browser_research_host(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, ResearchRuntime>,
    owner_key: String,
) -> Result<Value, String> {
    if webview.label() != crate::MAIN_WINDOW_LABEL {
        return Err("research_main_window_required".into());
    }
    runtime.host_identity(&app, &owner_key)
}

#[tauri::command]
pub(crate) async fn run_browser_research(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, ResearchRuntime>,
    exchange_runtime: State<'_, crate::local_ai_browser::LocalAiBrowserRuntime>,
    project_key: String,
    owner_key: String,
    command: ResearchCommand,
) -> Result<Value, String> {
    if webview.label() != crate::MAIN_WINDOW_LABEL {
        return Err("research_main_window_required".into());
    }
    // A site that is also an exchange provider is researched inside the user's own login
    // window, so one login serves both the person and the AI. Other sites keep their own window.
    let attach_label = match (command.kind.as_str(), command.site_id.as_deref()) {
        ("open", Some(site_id)) => {
            crate::local_ai_browser::exchange_webview::ensure_exchange_session_for_site(
                &app,
                &webview,
                exchange_runtime,
                site_id,
                &owner_key,
            )
            .await
        }
        _ => None,
    };
    let result = runtime.execute(
        &app,
        &project_key,
        &owner_key,
        command,
        attach_label.as_deref(),
    )?;
    if serde_json::to_vec(&result)
        .map_err(|_| "invalid_research_result")?
        .len()
        > 60 * 1024
    {
        return Err("research_result_too_large".into());
    }
    Ok(result)
}
