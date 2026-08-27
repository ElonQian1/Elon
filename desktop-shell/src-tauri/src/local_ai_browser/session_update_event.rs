use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub(super) const NAME: &str = "elon:local-ai-session-updated";

/// Notify the main React surface that sanitized state changed without copying
/// response text or a private upstream payload through the event channel.
pub(super) fn emit(
    app: &AppHandle,
    main_webview_label: &str,
    provider_id: &str,
    window_label: &str,
    kind: &str,
) {
    let _ = app.emit_to(
        main_webview_label,
        NAME,
        LocalAiWebSessionUpdate {
            provider_id,
            window_label,
            kind,
        },
    );
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalAiWebSessionUpdate<'a> {
    provider_id: &'a str,
    window_label: &'a str,
    kind: &'a str,
}
