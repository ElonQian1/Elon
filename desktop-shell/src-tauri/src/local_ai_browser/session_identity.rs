use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use super::{
    display_error, initial_renderer_status, LocalAiBrowserRuntime, ProviderDefinition,
    LOCAL_AI_WINDOW_PREFIX, PROFILE_ROOT, SNAPSHOT_CACHE_FILE,
};

pub(super) fn window_label(provider: &ProviderDefinition, fingerprint: &str) -> String {
    format!("{LOCAL_AI_WINDOW_PREFIX}{}-{fingerprint}", provider.id)
}

pub(super) fn profile_directory(
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

pub(super) fn ensure_runtime_session(
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
