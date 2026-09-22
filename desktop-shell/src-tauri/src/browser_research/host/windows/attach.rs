//! Bind capture to an exchange session webview that `local_ai_browser` already owns.
//! The page, profile and login state stay where the user put them; only CDP observation is added.
use super::{enable, install, watch_destroyed, Stage, CAPTURES};
use super::{HostConfig, HostHandle};
use tauri::Manager;

pub(super) fn attach_on_main(
    app: &tauri::AppHandle,
    config: HostConfig,
    handle: &HostHandle,
    generation: u64,
) -> Result<(), &'static str> {
    let webview = app
        .get_webview(&config.label)
        .ok_or("browser_research_window_unavailable")?;
    if !handle.handshake_current(generation) {
        return Ok(());
    }
    watch_destroyed(&webview.window(), handle);
    let existing = CAPTURES.with(|states| states.borrow().get(&config.label).cloned());
    let Some(context) = existing else {
        return install(app, config, handle.clone(), generation, false);
    };
    // A previous research session already subscribed this webview. Rebind the live
    // capture to the new session instead of stacking a second set of CDP receivers.
    {
        let mut state = context.borrow_mut();
        state.config = config;
        state.handle = handle.clone();
        state.generation = handle.generation();
        state.requests.clear();
        state.contexts.clear();
        state.scripts.clear();
        state.request_bindings.clear();
        state.reads.clear_waiting();
        state.ready = false;
    }
    if !handle.handshake_stage(generation, Stage::NativeAttached) {
        return Ok(());
    }
    enable::run(&context, generation, false);
    Ok(())
}
