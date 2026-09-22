//! Native, read-only observation of a research window, or of an existing exchange session webview.
mod handshake;
mod types;
#[cfg(windows)]
mod windows;

pub(crate) use types::{HostConfig, HostEvent, HostHandle, HostSink};

/// Exchange session webviews created by `local_ai_browser` carry this label prefix.
pub(crate) const ATTACHED_LABEL_PREFIX: &str = "local-ai-";

pub(crate) fn open(
    app: &tauri::AppHandle,
    config: HostConfig,
    sink: HostSink,
) -> Result<HostHandle, String> {
    config.validate()?;
    #[cfg(windows)]
    {
        windows::open(app, config, sink)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, config, sink);
        Err("browser_research_host_unsupported".into())
    }
}

/// Attach capture to a webview that already exists under `config.label`; no window is created.
pub(crate) fn attach(
    app: &tauri::AppHandle,
    config: HostConfig,
    sink: HostSink,
) -> Result<HostHandle, String> {
    if !config.attached {
        return Err("browser_research_host_config_invalid".into());
    }
    config.validate()?;
    #[cfg(windows)]
    {
        windows::attach(app, config, sink)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, config, sink);
        Err("browser_research_host_unsupported".into())
    }
}

pub(crate) fn pause(handle: &HostHandle) {
    handle.pause();
}

pub(crate) fn resume(app: &tauri::AppHandle, handle: &HostHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        windows::resume(app, handle)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, handle);
        Err("browser_research_host_unsupported".into())
    }
}

/// One bounded `Runtime.evaluate` on the observed top document. Callers must apply the
/// development gate before reaching this; the host itself never decides policy.
pub(crate) fn evaluate(
    app: &tauri::AppHandle,
    handle: &HostHandle,
    expression: String,
) -> Result<serde_json::Value, String> {
    #[cfg(windows)]
    {
        windows::evaluate(app, handle, expression)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, handle, expression);
        Err("browser_research_host_unsupported".into())
    }
}
