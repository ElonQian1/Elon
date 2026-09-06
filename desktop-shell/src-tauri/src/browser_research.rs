//! The research domain is separate from chat adapters and never executes financial actions.
mod files;
pub(crate) mod host;
mod ingest;
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
pub(crate) async fn run_browser_research(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, ResearchRuntime>,
    project_key: String,
    owner_key: String,
    command: ResearchCommand,
) -> Result<Value, String> {
    if webview.label() != crate::MAIN_WINDOW_LABEL {
        return Err("research_main_window_required".into());
    }
    let result = runtime.execute(&app, &project_key, &owner_key, command)?;
    if serde_json::to_vec(&result)
        .map_err(|_| "invalid_research_result")?
        .len()
        > 60 * 1024
    {
        return Err("research_result_too_large".into());
    }
    Ok(result)
}
