//! Native, read-only observation of a new, independently profiled research window.
mod types;
#[cfg(windows)]
mod windows;

pub(crate) use types::{HostConfig, HostEvent, HostHandle, HostSink};

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
