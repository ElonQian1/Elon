use tauri::{AppHandle, Webview};

use super::{
    display_error, embedded_view, parse_start_url, state::LocalAiBrowserRuntime, ProviderDefinition,
};

pub(super) fn apply(
    app: &AppHandle,
    runtime: &LocalAiBrowserRuntime,
    provider: &ProviderDefinition,
    label: &str,
    page: &Webview,
    action: &str,
) -> Result<(), String> {
    match action {
        "restore" => {
            embedded_view::restore_popout(app, label)?;
            runtime.mark_window_status(label, "ready");
            runtime.mark_window_visible(label, true);
        }
        "reload" => embedded_view::reload_after_stop(page)?,
        "back" => page.eval("history.back();").map_err(display_error)?,
        "home" | "new_conversation_home" | "new_conversation_reload"
            if action == "home" || provider.id == "chatgpt" =>
        {
            if matches!(action, "new_conversation_home" | "new_conversation_reload") {
                runtime.mark_command_pending(label, "new_conversation", None);
            }
            if action == "new_conversation_reload" {
                embedded_view::reload_after_stop(page)?;
            } else {
                embedded_view::navigate_after_stop(page, parse_start_url(provider)?)?;
            }
        }
        _ => return Err("不支持的本地 AI 浏览器控制动作。".to_string()),
    }
    Ok(())
}
